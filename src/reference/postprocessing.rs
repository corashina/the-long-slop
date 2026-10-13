//! Validated source effects, camera order, and exact histogram bounds.
#[derive(Clone, Debug)]
pub struct HistogramLayout {
    pub source: [u32; 2],
    pub samples: [u32; 2],
    pub sample_count: u32,
    pub tiles: u32,
    pub columns: u32,
    pub scatter: [u32; 2],
    pub reduction_rows: Vec<u32>,
    pub total_bound: u32,
}
impl HistogramLayout {
    pub fn new(width: u32, height: u32, max_dimension: u32) -> Result<Self, String> {
        if width == 0
            || height == 0
            || width > max_dimension
            || height > max_dimension
            || max_dimension < 32
        {
            return Err("Invalid source histogram extent".into());
        }
        let samples = [width.div_ceil(2), height.div_ceil(2)];
        let count = u64::from(samples[0]) * u64::from(samples[1]);
        let bound = (64 * u64::from(samples[0]) - 8 * u64::from(width.div_ceil(8)))
            .checked_mul(u64::from(samples[1]))
            .ok_or("Source histogram exceeds uint32 transport")?;
        if bound > u64::from(u32::MAX) || count * 6 > u64::from(u32::MAX) {
            return Err("Source histogram exceeds uint32 transport".into());
        }
        let tiles = (count as u32).div_ceil(32);
        let mut columns = 1u32;
        while u64::from(columns) * u64::from(columns) * 32 < u64::from(tiles) {
            columns += 1;
        }
        columns = columns.min(max_dimension / 32);
        let scatter = [columns * 32, tiles.div_ceil(columns)];
        if scatter[1] > max_dimension {
            return Err("Source histogram scatter exceeds device extent".into());
        }
        let mut reduction_rows = Vec::new();
        let mut rows = tiles;
        loop {
            rows = rows.div_ceil(256);
            if rows > max_dimension {
                return Err("Source histogram reduction exceeds device extent".into());
            }
            reduction_rows.push(rows);
            if rows == 1 {
                break;
            }
        }
        Ok(Self {
            source: [width, height],
            samples,
            sample_count: count as u32,
            tiles,
            columns,
            scatter,
            reduction_rows,
            total_bound: bound as u32,
        })
    }
    pub fn logical_bytes(&self) -> u64 {
        u64::from(self.scatter[0]) * u64::from(self.scatter[1]) * 8
            + self
                .reduction_rows
                .iter()
                .map(|rows| 32 * u64::from(*rows) * 16)
                .sum::<u64>()
    }
}

/// Active source exposure settings. Percent filtering is converted only at GPU binding.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposureSettings {
    pub filtering_percent: [f32; 2],
    pub luminance_ev: [f32; 2],
    pub key: f32,
    pub speed_up: f32,
    pub speed_down: f32,
}
impl ExposureSettings {
    pub fn validate(&self) -> bool {
        self.filtering_percent
            .iter()
            .chain(self.luminance_ev.iter())
            .all(|v| v.is_finite())
            && self.filtering_percent[0] >= 1.
            && self.filtering_percent[1] <= 99.
            && self.filtering_percent[1] - self.filtering_percent[0] >= 0.01
            && self.luminance_ev[0] <= self.luminance_ev[1]
            && self
                .luminance_ev
                .iter()
                .all(|v| v.exp2().is_finite() && v.exp2() > 0.)
            && self.key.is_finite()
            && self.key > 0.
            && self.speed_up.is_finite()
            && self.speed_up >= 0.
            && self.speed_down.is_finite()
            && self.speed_down >= 0.
    }
}

/// Main-camera effects after source startup controls and volume blending.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostprocessingRecord {
    pub main_layer: super::SourceId,
    pub stop_nan: bool,
    pub taa: TaaSettings,
    pub exposure: ExposureSettings,
    pub bloom: BloomSettings,
    pub occlusion: OcclusionSettings,
    pub motion_blur: MotionBlurSettings,
    pub grading: GradingSettings,
    pub blue_noise: Vec<super::SourceId>,
    pub heat_haze_weight: f32,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaaSettings {
    pub jitter_spread: f32,
    pub stationary_blending: f32,
    pub motion_blending: f32,
    pub sharpness: f32,
}
impl TaaSettings {
    pub fn validate(&self) -> bool {
        [
            self.jitter_spread,
            self.stationary_blending,
            self.motion_blending,
        ]
        .iter()
        .all(|value| value.is_finite() && (0. ..=1.).contains(value))
            && self.sharpness.is_finite()
            && self.sharpness >= 0.
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BloomSettings {
    pub intensity: f32,
    pub threshold: f32,
    pub soft_knee: f32,
    pub clamp: f32,
    pub diffusion: f32,
    pub anamorphic_ratio: f32,
    pub color: [f32; 4],
    pub fast: bool,
    pub dirt: super::SourceId,
    pub dirt_intensity: f32,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcclusionSettings {
    pub intensity: f32,
    pub thickness: f32,
    pub noise_tolerance: f32,
    pub blur_tolerance: f32,
    pub upsample_tolerance: f32,
    pub direct_lighting_strength: f32,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionBlurSettings {
    pub samples: u32,
    pub shutter_range: [f32; 2],
    pub speed_range: [f32; 2],
    pub shutter_lerp: f32,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradingCurves {
    SourceDefault,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradingSettings {
    pub lut_size: u32,
    pub curves: GradingCurves,
    pub temperature: f32,
    pub tint: f32,
    pub hue_shift: f32,
    pub saturation: f32,
    pub post_exposure: f32,
    pub contrast: f32,
    pub color_filter: [f32; 4],
    pub channel_mixer: [[f32; 3]; 3],
    pub trackballs: [[f32; 4]; 3],
}
impl PostprocessingRecord {
    pub fn validate(
        &self,
        textures: &std::collections::HashSet<super::SourceId>,
    ) -> Result<(), super::ReferenceError> {
        use super::{finite, require};
        require(
            !self.main_layer.file.is_empty() && self.main_layer.path_id != 0,
            "postprocessing main layer",
        )?;
        require(self.taa.validate(), "postprocessing temporal antialiasing")?;
        require(self.exposure.validate(), "postprocessing exposure")?;
        let bloom = &self.bloom;
        require(
            finite(&[
                bloom.intensity,
                bloom.threshold,
                bloom.soft_knee,
                bloom.clamp,
                bloom.diffusion,
                bloom.anamorphic_ratio,
                bloom.dirt_intensity,
            ]) && finite(&bloom.color)
                && bloom.color.iter().all(|v| *v >= 0.)
                && bloom.intensity >= 0.
                && bloom.threshold >= 0.
                && bloom.clamp > 0.
                && (0. ..=1.).contains(&bloom.soft_knee)
                && (1. ..=10.).contains(&bloom.diffusion)
                && bloom.anamorphic_ratio == 0.
                && bloom.fast
                && bloom.dirt_intensity >= 0.
                && textures.contains(&bloom.dirt),
            "postprocessing source bloom variant or dirt texture",
        )?;
        let ao = self.occlusion;
        require(
            finite(&[
                ao.intensity,
                ao.thickness,
                ao.noise_tolerance,
                ao.blur_tolerance,
                ao.upsample_tolerance,
                ao.direct_lighting_strength,
            ]) && ao.intensity >= 0.
                && ao.thickness > 0.
                && (0. ..=1.).contains(&ao.direct_lighting_strength),
            "postprocessing multiscale occlusion",
        )?;
        let motion = self.motion_blur;
        require(
            motion.samples == 4
                && finite(&motion.shutter_range)
                && finite(&motion.speed_range)
                && motion.shutter_range[0] >= 0.
                && motion.shutter_range[1] <= 360.
                && motion.shutter_range[0] <= motion.shutter_range[1]
                && motion.speed_range[0] < motion.speed_range[1]
                && motion.shutter_lerp.is_finite()
                && motion.shutter_lerp >= 0.,
            "postprocessing motion blur",
        )?;
        let grading = &self.grading;
        require(
            grading.lut_size == 33
                && finite(&[
                    grading.temperature,
                    grading.tint,
                    grading.hue_shift,
                    grading.saturation,
                    grading.post_exposure,
                    grading.contrast,
                ])
                && finite(&grading.color_filter)
                && grading.channel_mixer.iter().all(|values| finite(values))
                && grading.trackballs.iter().all(|values| finite(values)),
            "postprocessing Neutral grading",
        )?;
        require(
            self.blue_noise.len() == 64
                && self
                    .blue_noise
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == 64
                && self
                    .blue_noise
                    .iter()
                    .all(|source| textures.contains(source)),
            "postprocessing blue noise resources",
        )?;
        require(
            self.heat_haze_weight == 0.,
            "postprocessing startup heat haze interval",
        )
    }
}

/// Source camera order. Probe captures have a separate scene path and no player effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraRole {
    Sky,
    Far,
    Main,
    Foreground,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraClear {
    Skybox,
    Depth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraEffectScope {
    MainStack,
    DitherOnly,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraEffects {
    pub source: super::SourceId,
    pub volume_mask: u32,
    pub scope: CameraEffectScope,
    pub stop_nan: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraStage {
    pub role: CameraRole,
    pub source: super::SourceId,
    pub order: f32,
    pub clear: CameraClear,
    pub near: f32,
    pub far: f32,
    pub field_of_view: f32,
    pub mask: u32,
    pub hdr: bool,
    pub allow_msaa: bool,
    pub postprocessing: Option<CameraEffects>,
}
impl CameraStage {
    pub fn validate_stack(stages: &[Self]) -> bool {
        stages.len() == 4
            && stages.iter().map(|s| s.role).eq([
                CameraRole::Sky,
                CameraRole::Far,
                CameraRole::Main,
                CameraRole::Foreground,
            ])
            && stages.windows(2).all(|pair| pair[0].order < pair[1].order)
            && stages.iter().all(|stage| {
                let valid_source =
                    |source: &super::SourceId| !source.file.is_empty() && source.path_id != 0;
                valid_source(&stage.source)
                    && super::finite(&[stage.order, stage.near, stage.far, stage.field_of_view])
                    && stage.near > 0.
                    && stage.far > stage.near
                    && stage.field_of_view > 0.
                    && stage.field_of_view < 180.
                    && stage.hdr
                    && stage.allow_msaa
                    && match (&stage.role, &stage.postprocessing) {
                        (CameraRole::Sky, None) => matches!(stage.clear, CameraClear::Skybox),
                        (CameraRole::Main, Some(effect)) => {
                            valid_source(&effect.source)
                                && matches!(stage.clear, CameraClear::Depth)
                                && effect.scope == CameraEffectScope::MainStack
                                && effect.volume_mask == 2097408
                        }
                        (CameraRole::Far, Some(effect)) => {
                            valid_source(&effect.source)
                                && matches!(stage.clear, CameraClear::Depth)
                                && effect.scope == CameraEffectScope::DitherOnly
                                && effect.volume_mask == 0
                        }
                        (CameraRole::Foreground, Some(effect)) => {
                            valid_source(&effect.source)
                                && matches!(stage.clear, CameraClear::Depth)
                                && effect.scope == CameraEffectScope::DitherOnly
                                && effect.volume_mask == 32
                        }
                        _ => false,
                    }
            })
    }
}
