use super::*;
use crate::{gles, CommandEncoder as _};
use alloc::{sync::Arc, vec::Vec};
use core::num::NonZeroU32;

fn encoder() -> gles::CommandEncoder {
    let counters = Arc::new(wgt::HalCounters::default());
    counters.command_encoders.add(1);
    gles::CommandEncoder {
        cmd_buffer: Default::default(),
        state: Default::default(),
        private_caps: gles::PrivateCapabilities::empty(),
        counters,
    }
}

fn bindings(encoder: &gles::CommandEncoder) -> Vec<(u32, Option<glow::Sampler>)> {
    encoder
        .cmd_buffer
        .commands
        .iter()
        .filter_map(|command| match *command {
            C::BindSampler(unit, sampler) => Some((unit, sampler)),
            _ => None,
        })
        .collect()
}

#[test]
fn sampler_cache_tracks_units_and_explicit_unbinding() {
    let mut encoder = encoder();
    let sampler = glow::NativeSampler(NonZeroU32::new(1).unwrap());
    encoder.rebind_sampler_states(1, 0);
    encoder.rebind_sampler_states(1, 0);
    assert_eq!(bindings(&encoder), [(0, None)]);

    encoder.state.samplers[0] = Some(sampler);
    encoder.state.texture_slots[0].sampler_index = Some(0);
    encoder.state.texture_slots[1].sampler_index = Some(0);
    encoder.rebind_sampler_states(3, 1);
    encoder.state.texture_slots[0].tex_target = glow::TEXTURE_CUBE_MAP;
    encoder.rebind_sampler_states(3, 1);
    assert_eq!(
        bindings(&encoder),
        [(0, None), (0, Some(sampler)), (1, Some(sampler))]
    );

    encoder.state.texture_slots[0].sampler_index = None;
    encoder.rebind_sampler_states(1, 0);
    encoder.rebind_sampler_states(1, 0);
    assert_eq!(bindings(&encoder).last(), Some(&(0, None)));
    assert_eq!(bindings(&encoder).len(), 4);
}

#[test]
fn sampler_cache_resolves_pipeline_mapping_before_comparison() {
    let mut encoder = encoder();
    let sampler = glow::NativeSampler(NonZeroU32::new(1).unwrap());
    let other = glow::NativeSampler(NonZeroU32::new(2).unwrap());
    encoder.state.samplers[0] = Some(sampler);
    encoder.state.samplers[1] = Some(sampler);
    let mut pipeline = gles::PipelineInner {
        program: glow::NativeProgram(NonZeroU32::new(1).unwrap()),
        sampler_map: Default::default(),
        first_instance_location: None,
        immediates_descs: Default::default(),
        clip_distance_count: 0,
    };
    pipeline.sampler_map[0] = Some(0);
    encoder.set_pipeline_inner(&pipeline);
    pipeline.sampler_map[0] = Some(1);
    encoder.set_pipeline_inner(&pipeline);
    assert_eq!(bindings(&encoder), [(0, Some(sampler))]);

    encoder.state.samplers[1] = Some(other);
    encoder.rebind_sampler_states(0, 2);
    pipeline.sampler_map[0] = None;
    encoder.set_pipeline_inner(&pipeline);
    assert_eq!(
        bindings(&encoder),
        [(0, Some(sampler)), (0, Some(other)), (0, None)]
    );
}

#[test]
fn sampler_cache_preserves_dynamic_buffer_ranges() {
    let mut encoder = encoder();
    let sampler = glow::NativeSampler(NonZeroU32::new(1).unwrap());
    let buffer = glow::NativeBuffer(NonZeroU32::new(1).unwrap());
    encoder.state.texture_slots[0].sampler_index = Some(0);
    let layout = gles::PipelineLayout {
        group_infos: alloc::vec![Some(gles::BindGroupLayoutInfo {
            entries: Arc::from([
                wgt::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgt::ShaderStages::VERTEX,
                    ty: wgt::BindingType::Buffer {
                        ty: wgt::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgt::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgt::ShaderStages::FRAGMENT,
                    ty: wgt::BindingType::Sampler(wgt::SamplerBindingType::Filtering),
                    count: None,
                },
            ]),
            binding_to_slot: alloc::vec![0, 0].into_boxed_slice(),
        })]
        .into_boxed_slice(),
        naga_options: naga::back::glsl::Options {
            version: naga::back::glsl::Version::Embedded {
                version: 300,
                is_webgl: false,
            },
            writer_flags: naga::back::glsl::WriterFlags::empty(),
            binding_map: Default::default(),
            zero_initialize_workgroup_memory: true,
        },
    };
    let group = gles::BindGroup {
        contents: alloc::vec![
            gles::RawBinding::Buffer {
                raw: buffer,
                offset: 256,
                size: 64
            },
            gles::RawBinding::Sampler(sampler),
        ]
        .into_boxed_slice(),
    };
    unsafe { encoder.set_bind_group(&layout, 0, &group, &[256]) };
    unsafe { encoder.set_bind_group(&layout, 0, &group, &[512]) };
    assert_eq!(bindings(&encoder), [(0, Some(sampler))]);
    let ranges: Vec<_> = encoder
        .cmd_buffer
        .commands
        .iter()
        .filter_map(|command| match *command {
            C::BindBuffer {
                target,
                slot,
                buffer,
                offset,
                size,
            } => Some((target, slot, buffer, offset, size)),
            _ => None,
        })
        .collect();
    assert_eq!(
        ranges,
        [
            (glow::UNIFORM_BUFFER, 0, buffer, 512, 64),
            (glow::UNIFORM_BUFFER, 0, buffer, 768, 64),
        ]
    );
}

#[test]
fn sampler_cache_starts_unknown_for_each_pass_and_encoding() {
    let mut encoder = encoder();
    encoder.rebind_sampler_states(1, 0);
    let desc = crate::RenderPassDescriptor {
        label: None,
        extent: wgt::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        sample_count: 1,
        color_attachments: &[],
        depth_stencil_attachment: None,
        multiview_mask: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };
    unsafe { encoder.begin_render_pass(&desc).unwrap() };
    encoder.rebind_sampler_states(1, 0);
    encoder.rebind_sampler_states(1, 0);
    unsafe { encoder.end_render_pass() };
    unsafe {
        encoder.begin_compute_pass(&crate::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        })
    };
    encoder.rebind_sampler_states(1, 0);
    unsafe { encoder.end_compute_pass() };
    assert_eq!(bindings(&encoder), [(0, None), (0, None), (0, None)]);

    let recorded = unsafe { encoder.end_encoding().unwrap() };
    unsafe { encoder.begin_encoding(None).unwrap() };
    encoder.rebind_sampler_states(1, 0);
    assert_eq!(bindings(&encoder), [(0, None)]);
    assert!(!recorded.commands.is_empty());
}
