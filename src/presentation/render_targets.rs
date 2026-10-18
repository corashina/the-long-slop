//! Persistent source render targets. Allocation is restricted to startup or resize.
use super::surface_views::VIEW_COUNT;

use bevy::render::{
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
};

pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
pub const SHADOW_FORMAT: TextureFormat = TextureFormat::Depth16Unorm;
pub const COLLECTOR_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
pub const MOTION_FORMAT: TextureFormat = TextureFormat::Rg16Float;
pub const EXPOSURE_FORMAT: TextureFormat = TextureFormat::R32Float;
pub const HISTOGRAM_FORMAT: TextureFormat = TextureFormat::Rgba32Uint;

/// Validate the wgpu device contract, separately from raw WebGL capability probes.
pub fn validate_source_device(
    device: &RenderDevice,
    adapter: &RenderAdapter,
) -> Result<(), String> {
    let limits = device.limits();
    if limits.max_sampled_textures_per_shader_stage < 16
        || limits.max_samplers_per_shader_stage < 16
        || limits.max_vertex_attributes < 10
        || limits.max_texture_dimension_3d < 33
    {
        return Err("Source lighting requires16 textures,16 samplers,10 vertex attributes, and a33-cube LUT".into());
    }
    let sampled_target = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    for format in [
        HDR_FORMAT,
        DEPTH_FORMAT,
        SHADOW_FORMAT,
        COLLECTOR_FORMAT,
        MOTION_FORMAT,
        HISTOGRAM_FORMAT,
        EXPOSURE_FORMAT,
    ] {
        let features = adapter.get_texture_format_features(format);
        if !features.allowed_usages.contains(sampled_target) {
            return Err(format!(
                "Source rendering requires sampled render target {format:?}"
            ));
        }
    }
    for format in [HDR_FORMAT, DEPTH_FORMAT] {
        if !adapter
            .get_texture_format_features(format)
            .flags
            .contains(TextureFormatFeatureFlags::MULTISAMPLE_X8)
        {
            return Err(format!("Source rendering requires8-sample {format:?}"));
        }
    }
    let hdr = adapter.get_texture_format_features(HDR_FORMAT).flags;
    if !hdr.contains(TextureFormatFeatureFlags::FILTERABLE | TextureFormatFeatureFlags::BLENDABLE) {
        return Err(
            "Source HDR and histogram scatter require filterable, blendable RGBA16F".into(),
        );
    }
    Ok(())
}

pub struct Target {
    pub texture: Texture,
    pub attachment: TextureView,
    pub sampled: Option<TextureView>,
}
impl Target {
    fn new(
        device: &RenderDevice,
        label: &'static str,
        width: u32,
        height: u32,
        samples: u32,
        format: TextureFormat,
        sampled: bool,
    ) -> Self {
        // WebGL2 multisample attachments use renderbuffers. Only resolved
        // single-sample textures support sampling and copy operations.
        let usage = TextureUsages::RENDER_ATTACHMENT
            | if samples == 1 {
                TextureUsages::COPY_SRC
            } else {
                TextureUsages::empty()
            }
            | if sampled {
                TextureUsages::TEXTURE_BINDING
            } else {
                TextureUsages::empty()
            };
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        // Combined depth/stencil attachments must retain both aspects.
        let attachment = texture.create_view(&TextureViewDescriptor::default());
        let sampled = sampled.then(|| {
            if format == DEPTH_FORMAT {
                texture.create_view(&TextureViewDescriptor {
                    aspect: TextureAspect::DepthOnly,
                    ..Default::default()
                })
            } else {
                attachment.clone()
            }
        });
        Self {
            texture,
            attachment,
            sampled,
        }
    }
}

/// One color pair is shared by the main camera stack. Probe faces reuse a second pair.
pub struct SourceColorTargets {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub resolved: Target,
    pub multisample: Option<Target>,
    pub logical_bytes: u64,
}
impl SourceColorTargets {
    pub fn new(
        device: &RenderDevice,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<Self, String> {
        let limit = device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > limit || height > limit || !matches!(samples, 1 | 8)
        {
            return Err("Source color extent or sample count is unsupported".into());
        }
        let resolved = Target::new(device, "source_hdr", width, height, 1, HDR_FORMAT, true);
        let multisample = (samples > 1).then(|| {
            Target::new(
                device,
                "source_msaa_hdr",
                width,
                height,
                samples,
                HDR_FORMAT,
                false,
            )
        });
        Ok(Self {
            width,
            height,
            samples,
            resolved,
            multisample,
            logical_bytes: u64::from(width)
                * u64::from(height)
                * 8
                * (1 + if samples > 1 { u64::from(samples) } else { 0 }),
        })
    }
    pub fn attachment(&self) -> &TextureView {
        self.multisample
            .as_ref()
            .map_or(&self.resolved.attachment, |target| &target.attachment)
    }
    pub fn resolve_target(&self) -> Option<&TextureView> {
        self.multisample.as_ref().map(|_| &self.resolved.attachment)
    }
}

/// Depth, motion, attenuation, and shadow storage belong to exactly one surface view.
/// Color ownership is shared without sharing any writable view uniform.
pub struct SourceViewTargets {
    pub colors: std::sync::Arc<SourceColorTargets>,
    pub atlas_size: u32,
    pub multisample_depth: Option<Target>,
    pub depth: Target,
    pub motion: Target,
    pub collector: Target,
    pub atlas: Target,
    /// Excludes shared color storage. Count each color pair once in the graph total.
    pub logical_bytes: u64,
}
impl SourceViewTargets {
    pub fn new(
        device: &RenderDevice,
        colors: std::sync::Arc<SourceColorTargets>,
        atlas_size: u32,
    ) -> Result<Self, String> {
        if atlas_size == 0 || atlas_size > device.limits().max_texture_dimension_2d {
            return Err("Source shadow atlas extent is unsupported".into());
        }
        let SourceColorTargets {
            width,
            height,
            samples,
            ..
        } = *colors;
        let multisample_depth = (samples > 1).then(|| {
            Target::new(
                device,
                "source_msaa_depth",
                width,
                height,
                samples,
                DEPTH_FORMAT,
                false,
            )
        });
        let depth = Target::new(device, "source_depth", width, height, 1, DEPTH_FORMAT, true);
        let motion = Target::new(
            device,
            "source_motion",
            width,
            height,
            1,
            MOTION_FORMAT,
            true,
        );
        let collector = Target::new(
            device,
            "source_screen_shadows",
            width,
            height,
            1,
            COLLECTOR_FORMAT,
            true,
        );
        let atlas = Target::new(
            device,
            "source_shadow_atlas",
            atlas_size,
            atlas_size,
            1,
            SHADOW_FORMAT,
            true,
        );
        // D24S8 uses four logical bytes. Backend allocation overhead is excluded.
        let logical_bytes = u64::from(width)
            * u64::from(height)
            * (12
                + if samples > 1 {
                    4 * u64::from(samples)
                } else {
                    0
                })
            + 2 * u64::from(atlas_size).pow(2);
        Ok(Self {
            colors,
            atlas_size,
            multisample_depth,
            depth,
            motion,
            collector,
            atlas,
            logical_bytes,
        })
    }
}

/// Three stack views share color storage; six capture views share single-sample scratch.
/// Depth and collector storage never alias between views.
pub struct SurfaceTargets {
    pub stack: std::sync::Arc<SourceColorTargets>,
    pub capture: std::sync::Arc<SourceColorTargets>,
    pub views: [std::sync::Arc<SourceViewTargets>; VIEW_COUNT],
    pub logical_bytes: u64,
}
impl SurfaceTargets {
    pub fn new(device: &RenderDevice, width: u32, height: u32) -> Result<Self, String> {
        use std::sync::Arc;
        let atlas = crate::reference::shadows::source_atlas_size(width, height)
            .ok_or("Source stack atlas extent is invalid")?;
        let capture_atlas = crate::reference::shadows::source_atlas_size(64, 64).unwrap();
        let stack = Arc::new(SourceColorTargets::new(device, width, height, 8)?);
        let capture = Arc::new(SourceColorTargets::new(device, 64, 64, 1)?);
        let mut views = Vec::with_capacity(VIEW_COUNT);
        for index in 0..VIEW_COUNT {
            let (color, atlas_size) = if index < 3 {
                (&stack, atlas)
            } else {
                (&capture, capture_atlas)
            };
            views.push(Arc::new(SourceViewTargets::new(
                device,
                color.clone(),
                atlas_size,
            )?));
        }
        let logical_bytes = stack.logical_bytes
            + capture.logical_bytes
            + views.iter().map(|view| view.logical_bytes).sum::<u64>();
        Ok(Self {
            stack,
            capture,
            views: views.try_into().ok().unwrap(),
            logical_bytes,
        })
    }
    /// Main resize preserves fixed capture targets and published probe storage.
    pub fn resize_stack(
        &mut self,
        device: &RenderDevice,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let atlas = crate::reference::shadows::source_atlas_size(width, height)
            .ok_or("Source stack atlas extent is invalid")?;
        let stack = std::sync::Arc::new(SourceColorTargets::new(device, width, height, 8)?);
        for index in 0..3 {
            self.views[index] =
                std::sync::Arc::new(SourceViewTargets::new(device, stack.clone(), atlas)?);
        }
        self.stack = stack;
        self.logical_bytes = self.stack.logical_bytes
            + self.capture.logical_bytes
            + self
                .views
                .iter()
                .map(|view| view.logical_bytes)
                .sum::<u64>();
        Ok(())
    }
    pub const TEXTURES: u32 = 3 + 3 * 5 + 6 * 4 + 3 * 5;
}

/// A scratch cube has fixed per-face render views and per-mip sample views.
/// The latter prevent attachment and sampled-subresource overlap during filtering.
pub struct ScratchCube {
    _texture: Texture,
    pub faces: [[TextureView; 6]; 7],
    pub levels: [TextureView; 7],
    pub cube: TextureView,
}
fn probe_texture(device: &RenderDevice, label: &'static str) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 6,
        },
        mip_level_count: 7,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: HDR_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
fn cube_view(texture: &Texture, level: u32, count: u32) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        base_mip_level: level,
        mip_level_count: Some(count),
        array_layer_count: Some(6),
        ..Default::default()
    })
}
impl ScratchCube {
    fn new(device: &RenderDevice, label: &'static str) -> Self {
        let texture = probe_texture(device, label);
        let faces = face_views(&texture);
        let levels = std::array::from_fn(|mip| cube_view(&texture, mip as u32, 1));
        let cube = cube_view(&texture, 0, 7);
        Self {
            _texture: texture,
            faces,
            levels,
            cube,
        }
    }
}
fn face_views(texture: &Texture) -> [[TextureView; 6]; 7] {
    std::array::from_fn(|mip| {
        std::array::from_fn(|face| {
            texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2),
                base_mip_level: mip as u32,
                mip_level_count: Some(1),
                base_array_layer: face as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        })
    })
}
/// Capture and filtering never write into the cube sampled by scene materials.
/// Fixed textures and views are allocated once. Publication copies all seven HDR levels.
pub struct ProbeTargets {
    pub base: ScratchCube,
    pub auxiliary: ScratchCube,
    pub published: Texture,
    pub published_view: TextureView,
    pub published_faces: [[TextureView; 6]; 7],
    pub capture_sample: Texture,
    pub capture_sample_view: TextureView,
}
impl ProbeTargets {
    pub const LOGICAL_BYTES: u64 = 786408;
    pub fn new(device: &RenderDevice, queue: &RenderQueue) -> Self {
        let base = ScratchCube::new(device, "source_probe_base");
        let auxiliary = ScratchCube::new(device, "source_probe_auxiliary");
        let published = probe_texture(device, "source_probe_published");
        let published_view = cube_view(&published, 0, 7);
        let published_faces = face_views(&published);
        // Source reflectionBounces=1 keeps candidate selection, but substitutes
        // the builtin black cube for every registered cube probe during capture.
        // Its six pixels have zero color and alpha; decode is (1,1,0,0).
        let capture_sample = device.create_texture(&TextureDescriptor {
            label: Some("source_capture_black_cube"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &capture_sample,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &[0; 24],
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
        );
        let capture_sample_view = cube_view(&capture_sample, 0, 1);
        Self {
            base,
            auxiliary,
            published,
            published_view,
            published_faces,
            capture_sample,
            capture_sample_view,
        }
    }
    /// Scene recreation invalidates the retained publication before any new scene samples it.
    pub fn clear_publication(&self, queue: &RenderQueue) {
        const ZERO_FACE: [u8; 64 * 64 * 8] = [0; 64 * 64 * 8];
        for mip in 0..7 {
            let side = 64 >> mip;
            for face in 0..6 {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: &self.published,
                        mip_level: mip,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: face,
                        },
                        aspect: TextureAspect::All,
                    },
                    &ZERO_FACE[..(side * side * 8) as usize],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(side * 8),
                        rows_per_image: Some(side),
                    },
                    Extent3d {
                        width: side,
                        height: side,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }
}
