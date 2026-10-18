//! Source camera and rigid-object motion. The graph owns draw dispatch and submission.
use super::{postprocessing::pipeline_failure, view_frame::ViewSnapshot};
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
};

#[derive(Clone, Copy, ShaderType)]
pub struct MotionCameraParams {
    pub raster_vp: Mat4,
    pub current_vp: Mat4,
    pub previous_vp: Mat4,
    pub camera_to_world: Mat4,
    pub inverse_projection: Mat4,
    pub projection: Vec4,
}
#[derive(Clone, Copy, ShaderType)]
pub struct MotionObjectParams {
    pub current_model: Mat4,
    pub previous_model: Mat4,
    pub control: UVec4,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MotionMode {
    Camera,
    Object,
    ForceNoMotion,
}

/// Register once; preserve the first model during subsequent updates of that motion frame.
pub struct MotionModelHistory {
    current: Mat4,
    committed_model: Mat4,
    previous: Mat4,
    frame: u32,
    completed: u32,
}
impl MotionModelHistory {
    pub fn new(model: Mat4, frame: u32) -> Self {
        Self {
            current: model,
            committed_model: model,
            previous: model,
            frame,
            completed: 0,
        }
    }
    pub fn update(&mut self, frame: u32, model: Mat4) {
        if frame != self.frame {
            self.previous = self.committed_model;
            self.frame = frame;
        }
        self.current = model;
    }
    pub fn params(&self, frame: u32, mode: MotionMode) -> MotionObjectParams {
        MotionObjectParams {
            current_model: self.current,
            previous_model: if frame == self.frame {
                self.previous
            } else {
                self.current
            },
            control: UVec4::new(u32::from(mode == MotionMode::ForceNoMotion), 0, 0, 0),
        }
    }
    pub fn eligible(&self, frame: u32, mode: MotionMode) -> bool {
        match mode {
            MotionMode::Camera => false,
            MotionMode::ForceNoMotion => true,
            MotionMode::Object => {
                frame == self.frame && !approximately(self.current, self.previous)
            }
        }
    }
    pub fn completed(&mut self, token: u32) {
        if token > self.completed && token == self.frame {
            self.previous = self.current;
            self.committed_model = self.current;
            self.completed = token;
        }
    }
    pub fn rebase(&mut self, delta: Vec3) {
        let shift = Mat4::from_translation(-delta);
        self.current = shift * self.current;
        self.committed_model = shift * self.committed_model;
        self.previous = shift * self.previous;
    }
}
fn approximately(a: Mat4, b: Mat4) -> bool {
    a.to_cols_array()
        .iter()
        .zip(b.to_cols_array())
        .all(|(a, b)| (a - b).abs() <= f32::from_bits(0x3727c5ac))
}

pub struct SourceMotionPass {
    _depth: Texture,
    pub copied_depth: TextureView,
    pub extent: UVec2,
    pub logical_bytes: u64,
    uniform: UniformBuffer<MotionCameraParams>,
    camera_binding: BindGroup,
    object_camera_binding: BindGroup,
    pipeline: CachedRenderPipelineId,
    previous_vp: Mat4,
    pending: Option<(u32, Mat4)>,
    completed: u32,
}
impl SourceMotionPass {
    pub fn camera_history_matches(&self, snapshot: &ViewSnapshot) -> bool {
        self.previous_vp == snapshot.raster_projection * snapshot.view_from_world
    }
    pub fn recreate(&mut self, snapshot: &ViewSnapshot) {
        self.register_camera(snapshot);
        self.pending = None;
        self.completed = 0;
    }
    pub fn camera_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "source_motion_camera",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    uniform_buffer::<MotionCameraParams>(false),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                ),
            ),
        )
    }
    pub fn object_camera_layout() -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "source_motion_object_camera",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<MotionCameraParams>(false),
            ),
        )
    }
    pub fn object_layout(limits: &WgpuLimits) -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor::new(
            "source_motion_object",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                GpuArrayBuffer::<MotionObjectParams>::binding_layout(limits),
            ),
        )
    }
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shader: Handle<Shader>,
        camera_depth: &TextureView,
        snapshot: &ViewSnapshot,
        previous: Option<Self>,
    ) -> Result<Self, String> {
        let extent = snapshot.extent;
        if extent.x == 0
            || extent.y == 0
            || extent.max_element() > device.limits().max_texture_dimension_2d
        {
            return Err("Invalid source motion extent".into());
        }
        let depth = device.create_texture(&TextureDescriptor {
            label: Some("source_motion_copied_depth"),
            size: Extent3d {
                width: extent.x,
                height: extent.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth24PlusStencil8,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let copied_depth = depth.create_view(&default());
        let previous_vp = previous.as_ref().map_or(
            snapshot.raster_projection * snapshot.view_from_world,
            |old| old.previous_vp,
        );
        let mut uniform = UniformBuffer::from(camera_params(snapshot, previous_vp)?);
        uniform.write_buffer(device, queue);
        let layout = Self::camera_layout();
        let camera_binding = device.create_bind_group(
            "source_motion_camera",
            &cache.get_bind_group_layout(&layout),
            &BindGroupEntries::sequential((uniform.binding().unwrap(), camera_depth)),
        );
        let object_camera_binding = device.create_bind_group(
            "source_motion_object_camera",
            &cache.get_bind_group_layout(&Self::object_camera_layout()),
            &BindGroupEntries::single(uniform.binding().unwrap()),
        );
        let pipeline = previous.as_ref().map_or_else(
            || {
                cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("source_motion_camera".into()),
                    layout: vec![layout],
                    vertex: VertexState {
                        shader: shader.clone(),
                        entry_point: Some("camera_vertex".into()),
                        ..default()
                    },
                    fragment: Some(FragmentState {
                        shader,
                        entry_point: Some("camera_fragment".into()),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rg16Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                        ..default()
                    }),
                    depth_stencil: Some(DepthStencilState {
                        format: TextureFormat::Depth24PlusStencil8,
                        depth_write_enabled: Some(true),
                        depth_compare: Some(CompareFunction::Always),
                        stencil: default(),
                        bias: default(),
                    }),
                    ..default()
                })
            },
            |old| old.pipeline,
        );
        Ok(Self {
            _depth: depth,
            copied_depth,
            extent,
            logical_bytes: u64::from(extent.x) * u64::from(extent.y) * 4,
            uniform,
            camera_binding,
            object_camera_binding,
            pipeline,
            previous_vp,
            pending: None,
            completed: previous.map_or(0, |old| old.completed),
        })
    }
    pub fn binding(&self) -> &BindGroup {
        &self.object_camera_binding
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        cache.get_render_pipeline(self.pipeline).is_some()
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        pipeline_failure(cache, &[self.pipeline])
    }
    pub fn encode_camera(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        motion: &TextureView,
        snapshot: &ViewSnapshot,
        token: u32,
    ) -> Result<bool, String> {
        if snapshot.extent != self.extent {
            return Err("Source motion extent changed without resize".into());
        }
        if token == 0 || token <= self.completed {
            return Ok(false);
        }
        if !self.ready(cache) {
            return Ok(false);
        }
        let params = camera_params(snapshot, self.previous_vp)?;
        self.uniform.set(params);
        self.uniform.write_buffer(device, queue);
        let colors = [Some(RenderPassColorAttachment {
            view: motion,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("source_motion_camera"),
            color_attachments: &colors,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.copied_depth,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..default()
        });
        pass.set_pipeline(cache.get_render_pipeline(self.pipeline).unwrap());
        pass.set_bind_group(0, &self.camera_binding, &[]);
        pass.draw(0..3, 0..1);
        self.pending = Some((token, params.current_vp));
        Ok(true)
    }
    pub fn completed(&mut self, token: u32) -> bool {
        if token <= self.completed {
            return false;
        }
        let Some((pending, vp)) = self.pending else {
            return false;
        };
        if pending != token {
            return false;
        }
        self.previous_vp = vp;
        self.pending = None;
        self.completed = token;
        true
    }
    /// Camera registration is separate from effect ResetHistory.
    pub fn register_camera(&mut self, snapshot: &ViewSnapshot) {
        self.previous_vp = snapshot.raster_projection * snapshot.view_from_world;
        self.pending = None;
    }
    pub fn rebase(&mut self, delta: Vec3) {
        let shift = Mat4::from_translation(delta);
        self.previous_vp *= shift;
        if let Some((_, vp)) = &mut self.pending {
            *vp *= shift;
        }
    }
}
fn camera_params(snapshot: &ViewSnapshot, previous_vp: Mat4) -> Result<MotionCameraParams, String> {
    if !snapshot.near.is_finite()
        || !snapshot.far.is_finite()
        || snapshot.near <= 0.
        || snapshot.far <= snapshot.near
    {
        return Err("Invalid source motion projection range".into());
    }
    let mut native = snapshot.raster_projection.to_cols_array_2d();
    for c in &mut native {
        c[1] = -c[1];
        c[2] = c[3] - 2. * c[2];
    }
    let inverse_projection = Mat4::from_cols_array_2d(&native).inverse();
    let reflect = Mat4::from_scale(Vec3::new(1., 1., -1.));
    let params = MotionCameraParams {
        raster_vp: snapshot.raster_projection * snapshot.view_from_world,
        current_vp: snapshot.nonjittered_projection * snapshot.view_from_world,
        previous_vp,
        camera_to_world: (reflect * snapshot.view_from_world).inverse(),
        inverse_projection,
        projection: Vec4::new(
            snapshot.near,
            snapshot.far,
            snapshot.far / snapshot.near - 1.,
            1.,
        ),
    };
    if [
        params.raster_vp,
        params.current_vp,
        params.previous_vp,
        params.camera_to_world,
        params.inverse_projection,
    ]
    .iter()
    .any(|m| !m.is_finite())
    {
        return Err("Invalid source motion matrix".into());
    }
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_params_keep_forced_zero_and_unsubmitted_history() {
        let registered = Mat4::from_translation(Vec3::X);
        let moved = Mat4::from_translation(Vec3::Y);
        let mut history = MotionModelHistory::new(registered, 1);
        history.update(1, moved);
        let packed = [
            history.params(1, MotionMode::Object),
            history.params(1, MotionMode::ForceNoMotion),
        ];
        for params in packed {
            assert_eq!(params.current_model, moved);
            assert_eq!(params.previous_model, registered);
        }
        assert_eq!(packed[0].control.x, 0);
        assert_eq!(packed[1].control.x, 1);
        history.completed(2);
        assert_eq!(
            history.params(1, MotionMode::Object).previous_model,
            registered
        );
        history.update(2, Mat4::IDENTITY);
        assert_eq!(
            history.params(2, MotionMode::Object).previous_model,
            registered
        );
        history.completed(2);
        assert!(!history.eligible(2, MotionMode::Object));
        assert!(!history.eligible(2, MotionMode::Camera));
        assert!(history.eligible(2, MotionMode::ForceNoMotion));
    }
    #[test]
    fn rigid_history_uses_registration_completed_and_stale_frame_rules() {
        let registered = Mat4::IDENTITY;
        let moved = Mat4::from_translation(Vec3::X);
        let later = Mat4::from_translation(Vec3::Y);
        let mut history = MotionModelHistory::new(registered, 1);
        assert!(!history.eligible(1, MotionMode::Object));
        assert!(history.eligible(1, MotionMode::ForceNoMotion));
        history.update(1, moved);
        assert!(history.eligible(1, MotionMode::Object));
        assert!(!history.eligible(1, MotionMode::Camera));
        assert_eq!(
            history.params(1, MotionMode::Object).previous_model,
            registered
        );
        assert_eq!(history.params(2, MotionMode::Object).previous_model, moved);
        history.completed(1);
        history.update(2, later);
        assert_eq!(history.params(2, MotionMode::Object).previous_model, moved);
        history.update(3, registered);
        assert_eq!(history.params(3, MotionMode::Object).previous_model, moved);
        history.completed(2);
        assert_eq!(history.params(3, MotionMode::Object).previous_model, moved);
        assert_eq!(history.params(3, MotionMode::ForceNoMotion).control.x, 1);
    }
    #[test]
    fn rebase_transports_both_models_without_changing_clip_motion() {
        let mut history =
            MotionModelHistory::new(Mat4::from_translation(Vec3::new(2., 3., -5.)), 1);
        history.update(1, Mat4::from_translation(Vec3::new(4., 3., -5.)));
        let before = history.params(1, MotionMode::Object);
        let vp = Mat4::perspective_rh_gl(1., 1.5, 0.3, 100.);
        let vertex = Vec4::new(0.2, 0.1, 0., 1.);
        let delta = Vec3::new(400., 0., -800.);
        history.rebase(delta);
        let after = history.params(1, MotionMode::Object);
        let transported = vp * Mat4::from_translation(delta);
        assert!(
            (vp * before.current_model * vertex - transported * after.current_model * vertex)
                .abs()
                .max_element()
                < 1e-4
        );
        assert!(
            (vp * before.previous_model * vertex - transported * after.previous_model * vertex)
                .abs()
                .max_element()
                < 1e-4
        );
    }
}
