//! Render bundles for own draw lists. A list re-records only when its commands change.
//! Bevy resource IDs are never reused, so equal commands bind the same resources.
use bevy::render::{
    render_phase::TrackedRenderPass,
    render_resource::{
        BindGroup, BindGroupId, Buffer, BufferId, IndexFormat, RenderPipeline, RenderPipelineId,
        TextureFormat,
    },
    renderer::RenderDevice,
};

/// The attachments of the passes that execute a bundle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BundleTarget {
    pub color: Option<TextureFormat>,
    pub depth: TextureFormat,
    pub samples: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    Pipeline(RenderPipelineId),
    BindGroup(BindGroupId, [u32; 2], usize),
    Vertex(BufferId),
    Index(BufferId, IndexFormat),
    DrawIndexedIndirect(BufferId, u64),
}
/// State slots: pipeline, bind groups 0-3, vertex buffer, index buffer.
const SLOTS: usize = 7;

enum Sink<'a, 'b> {
    Commands(&'b mut Vec<Command>),
    Bundle(&'b mut wgpu::RenderBundleEncoder<'a>),
}
/// One run of a draw list. The first run collects the commands. A changed list runs again into a bundle.
/// Both runs skip state that is already set.
pub struct Encoder<'a, 'b> {
    sink: Sink<'a, 'b>,
    state: [Option<Command>; SLOTS],
}
impl<'a> Encoder<'a, '_> {
    fn changed(&mut self, slot: usize, command: Command) -> bool {
        let changed = self.state[slot] != Some(command);
        self.state[slot] = Some(command);
        if changed && let Sink::Commands(list) = &mut self.sink {
            list.push(command);
        }
        changed
    }
    pub fn set_pipeline(&mut self, pipeline: &'a RenderPipeline) {
        if self.changed(0, Command::Pipeline(pipeline.id()))
            && let Sink::Bundle(bundle) = &mut self.sink
        {
            bundle.set_pipeline(pipeline);
        }
    }
    /// Bind groups 0-3 with at most two dynamic offsets.
    pub fn set_bind_group(&mut self, index: u32, group: &'a BindGroup, offsets: &[u32]) {
        let mut words = [0; 2];
        words[..offsets.len()].copy_from_slice(offsets);
        let command = Command::BindGroup(group.id(), words, offsets.len());
        if self.changed(1 + index as usize, command)
            && let Sink::Bundle(bundle) = &mut self.sink
        {
            bundle.set_bind_group(index, &**group, offsets);
        }
    }
    /// The whole buffer in vertex slot 0.
    pub fn set_vertex_buffer(&mut self, buffer: &'a Buffer) {
        if self.changed(5, Command::Vertex(buffer.id()))
            && let Sink::Bundle(bundle) = &mut self.sink
        {
            bundle.set_vertex_buffer(0, *buffer.slice(..));
        }
    }
    pub fn set_index_buffer(&mut self, buffer: &'a Buffer, format: IndexFormat) {
        if self.changed(6, Command::Index(buffer.id(), format))
            && let Sink::Bundle(bundle) = &mut self.sink
        {
            bundle.set_index_buffer(*buffer.slice(..), format);
        }
    }
    pub fn draw_indexed_indirect(&mut self, buffer: &'a Buffer, offset: u64) {
        match &mut self.sink {
            Sink::Commands(list) => list.push(Command::DrawIndexedIndirect(buffer.id(), offset)),
            Sink::Bundle(bundle) => bundle.draw_indexed_indirect(buffer, offset),
        }
    }
}

/// The bundle of one draw list and the commands it holds.
#[derive(Default)]
pub struct DrawBundle {
    target: Option<BundleTarget>,
    commands: Vec<Command>,
    next: Vec<Command>,
    bundle: Option<wgpu::RenderBundle>,
}
impl DrawBundle {
    /// Run `draws` and execute its commands as a bundle. Returns the result of the first run.
    /// `draws` must give the same commands and result on each run in one call.
    pub fn draw<'w, R>(
        &mut self,
        device: &RenderDevice,
        target: BundleTarget,
        pass: &mut TrackedRenderPass<'w>,
        mut draws: impl for<'b> FnMut(&mut Encoder<'w, 'b>) -> R,
    ) -> R {
        self.next.clear();
        let result = draws(&mut Encoder {
            sink: Sink::Commands(&mut self.next),
            state: [None; SLOTS],
        });
        if self.next.is_empty() {
            self.bundle = None;
            self.commands.clear();
            return result;
        }
        if self.bundle.is_none() || self.target != Some(target) || self.next != self.commands {
            let colors = [target.color];
            let mut encoder = device.wgpu_device().create_render_bundle_encoder(
                &wgpu::RenderBundleEncoderDescriptor {
                    label: Some("source_draw_bundle"),
                    color_formats: if target.color.is_some() { &colors } else { &[] },
                    depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                        format: target.depth,
                        depth_read_only: false,
                        // The source passes have no stencil operations.
                        stencil_read_only: true,
                    }),
                    sample_count: target.samples,
                    multiview: None,
                },
            );
            draws(&mut Encoder {
                sink: Sink::Bundle(&mut encoder),
                state: [None; SLOTS],
            });
            self.bundle = Some(encoder.finish(&wgpu::RenderBundleDescriptor {
                label: Some("source_draw_bundle"),
            }));
            self.target = Some(target);
            std::mem::swap(&mut self.commands, &mut self.next);
        }
        // This also resets the pass tracking. A bundle clears the pass state after it runs.
        pass.wgpu_pass()
            .execute_bundles(std::iter::once(self.bundle.as_ref().unwrap()));
        result
    }
}
