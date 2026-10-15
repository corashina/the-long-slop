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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceParameter<T> {
    pub override_state: bool,
    pub value: T,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTextureParameter {
    pub override_state: bool,
    pub value: Option<super::SourceId>,
    pub default_state: u32,
    pub texture: Option<u32>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceGradingSpline {
    pub curve: super::TorqueCurve,
    pub cached_data: Vec<f32>,
    pub r#loop: bool,
    pub range: f32,
    pub zero_value: f32,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlassesGradingParameters {
    #[serde(rename = "blueCurve")]
    pub blue_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "brightness")]
    pub brightness: SourceParameter<f32>,
    #[serde(rename = "colorFilter")]
    pub color_filter: SourceParameter<[f32; 4]>,
    #[serde(rename = "contrast")]
    pub contrast: SourceParameter<f32>,
    #[serde(rename = "enabled")]
    pub enabled: SourceParameter<bool>,
    #[serde(rename = "externalLut")]
    pub external_lut: SourceTextureParameter,
    #[serde(rename = "gain")]
    pub gain: SourceParameter<[f32; 4]>,
    #[serde(rename = "gamma")]
    pub gamma: SourceParameter<[f32; 4]>,
    #[serde(rename = "gradingMode")]
    pub grading_mode: SourceParameter<u32>,
    #[serde(rename = "greenCurve")]
    pub green_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "hueShift")]
    pub hue_shift: SourceParameter<f32>,
    #[serde(rename = "hueVsHueCurve")]
    pub hue_vs_hue_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "hueVsSatCurve")]
    pub hue_vs_sat_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "ldrLut")]
    pub ldr_lut: SourceTextureParameter,
    #[serde(rename = "ldrLutContribution")]
    pub ldr_lut_contribution: SourceParameter<f32>,
    #[serde(rename = "lift")]
    pub lift: SourceParameter<[f32; 4]>,
    #[serde(rename = "lumVsSatCurve")]
    pub lum_vs_sat_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "masterCurve")]
    pub master_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "mixerBlueOutBlueIn")]
    pub mixer_blue_out_blue_in: SourceParameter<f32>,
    #[serde(rename = "mixerBlueOutGreenIn")]
    pub mixer_blue_out_green_in: SourceParameter<f32>,
    #[serde(rename = "mixerBlueOutRedIn")]
    pub mixer_blue_out_red_in: SourceParameter<f32>,
    #[serde(rename = "mixerGreenOutBlueIn")]
    pub mixer_green_out_blue_in: SourceParameter<f32>,
    #[serde(rename = "mixerGreenOutGreenIn")]
    pub mixer_green_out_green_in: SourceParameter<f32>,
    #[serde(rename = "mixerGreenOutRedIn")]
    pub mixer_green_out_red_in: SourceParameter<f32>,
    #[serde(rename = "mixerRedOutBlueIn")]
    pub mixer_red_out_blue_in: SourceParameter<f32>,
    #[serde(rename = "mixerRedOutGreenIn")]
    pub mixer_red_out_green_in: SourceParameter<f32>,
    #[serde(rename = "mixerRedOutRedIn")]
    pub mixer_red_out_red_in: SourceParameter<f32>,
    #[serde(rename = "postExposure")]
    pub post_exposure: SourceParameter<f32>,
    #[serde(rename = "redCurve")]
    pub red_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "satVsSatCurve")]
    pub sat_vs_sat_curve: SourceParameter<SourceGradingSpline>,
    #[serde(rename = "saturation")]
    pub saturation: SourceParameter<f32>,
    #[serde(rename = "temperature")]
    pub temperature: SourceParameter<f32>,
    #[serde(rename = "tint")]
    pub tint: SourceParameter<f32>,
    #[serde(rename = "toneCurveGamma")]
    pub tone_curve_gamma: SourceParameter<f32>,
    #[serde(rename = "toneCurveShoulderAngle")]
    pub tone_curve_shoulder_angle: SourceParameter<f32>,
    #[serde(rename = "toneCurveShoulderLength")]
    pub tone_curve_shoulder_length: SourceParameter<f32>,
    #[serde(rename = "toneCurveShoulderStrength")]
    pub tone_curve_shoulder_strength: SourceParameter<f32>,
    #[serde(rename = "toneCurveToeLength")]
    pub tone_curve_toe_length: SourceParameter<f32>,
    #[serde(rename = "toneCurveToeStrength")]
    pub tone_curve_toe_strength: SourceParameter<f32>,
    #[serde(rename = "tonemapper")]
    pub tonemapper: SourceParameter<u32>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GlassesEffect {
    ColorGrading {
        source: super::SourceId,
        active: bool,
        parameters: GlassesGradingParameters,
    },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlassesProfile {
    pub source: super::SourceId,
    pub effects: Vec<GlassesEffect>,
}
impl SourceGradingSpline {
    fn valid(&self) -> bool {
        super::weather::curve_valid(&self.curve)
            && super::finite(&self.cached_data)
            && self.cached_data.len() == 128
            && self.range.is_finite()
            && self.zero_value.is_finite()
    }
}
impl GlassesGradingParameters {
    fn finite(&self) -> bool {
        self.blue_curve.value.valid()
            && self.brightness.value.is_finite()
            && super::finite(&self.color_filter.value)
            && self.contrast.value.is_finite()
            && super::finite(&self.gain.value)
            && super::finite(&self.gamma.value)
            && self.green_curve.value.valid()
            && self.hue_shift.value.is_finite()
            && self.hue_vs_hue_curve.value.valid()
            && self.hue_vs_sat_curve.value.valid()
            && self.ldr_lut_contribution.value.is_finite()
            && super::finite(&self.lift.value)
            && self.lum_vs_sat_curve.value.valid()
            && self.master_curve.value.valid()
            && self.mixer_blue_out_blue_in.value.is_finite()
            && self.mixer_blue_out_green_in.value.is_finite()
            && self.mixer_blue_out_red_in.value.is_finite()
            && self.mixer_green_out_blue_in.value.is_finite()
            && self.mixer_green_out_green_in.value.is_finite()
            && self.mixer_green_out_red_in.value.is_finite()
            && self.mixer_red_out_blue_in.value.is_finite()
            && self.mixer_red_out_green_in.value.is_finite()
            && self.mixer_red_out_red_in.value.is_finite()
            && self.post_exposure.value.is_finite()
            && self.red_curve.value.valid()
            && self.sat_vs_sat_curve.value.valid()
            && self.saturation.value.is_finite()
            && self.temperature.value.is_finite()
            && self.tint.value.is_finite()
            && self.tone_curve_gamma.value.is_finite()
            && self.tone_curve_shoulder_angle.value.is_finite()
            && self.tone_curve_shoulder_length.value.is_finite()
            && self.tone_curve_shoulder_strength.value.is_finite()
            && self.tone_curve_toe_length.value.is_finite()
            && self.tone_curve_toe_strength.value.is_finite()
    }
}
impl GlassesProfile {
    pub fn validate(
        &self,
        index: usize,
        m: &super::ReferenceManifest,
    ) -> Result<(), super::ReferenceError> {
        use super::{SourceId, require};
        let sid = |file: &str, path_id| SourceId {
            file: file.into(),
            path_id,
        };
        require(
            self.source == sid("sharedassets1.assets", 130128 + index as i64)
                && self.effects.len() == 1,
            "glasses profile identity",
        )?;
        let GlassesEffect::ColorGrading {
            source,
            active,
            parameters: p,
        } = &self.effects[0];
        require(
            *source == sid("sharedassets1.assets", 130142 + index as i64) && *active && p.finite(),
            "glasses effect source",
        )?;
        require(
            p.enabled.override_state
                && p.enabled.value
                && p.grading_mode.override_state
                && p.grading_mode.value == 1
                && p.tonemapper.override_state
                && p.tonemapper.value == 0
                && p.post_exposure.override_state
                && p.post_exposure.value == if index == 0 { -2. } else { 0. }
                && p.tone_curve_toe_length.override_state
                && p.tone_curve_toe_length.value == 0.5
                && p.tone_curve_toe_strength.override_state
                && p.tone_curve_toe_strength.value == 0.,
            "glasses grading source overrides",
        )?;
        require(
            !p.blue_curve.override_state
                && !p.brightness.override_state
                && !p.color_filter.override_state
                && !p.contrast.override_state
                && !p.external_lut.override_state
                && !p.gain.override_state
                && !p.gamma.override_state
                && !p.green_curve.override_state
                && !p.hue_shift.override_state
                && !p.hue_vs_hue_curve.override_state
                && !p.hue_vs_sat_curve.override_state
                && !p.ldr_lut.override_state
                && !p.ldr_lut_contribution.override_state
                && !p.lift.override_state
                && !p.lum_vs_sat_curve.override_state
                && !p.master_curve.override_state
                && !p.mixer_blue_out_green_in.override_state
                && !p.mixer_blue_out_red_in.override_state
                && !p.mixer_green_out_blue_in.override_state
                && !p.mixer_green_out_red_in.override_state
                && !p.mixer_red_out_blue_in.override_state
                && !p.mixer_red_out_green_in.override_state
                && !p.red_curve.override_state
                && !p.sat_vs_sat_curve.override_state
                && !p.saturation.override_state
                && !p.temperature.override_state
                && !p.tint.override_state
                && !p.tone_curve_gamma.override_state
                && !p.tone_curve_shoulder_angle.override_state
                && !p.tone_curve_shoulder_length.override_state
                && !p.tone_curve_shoulder_strength.override_state,
            "unexpected glasses override",
        )?;
        let rgb = [
            [100., 100., 100.],
            [50., 100., 200.],
            [75., 200., 75.],
            [200., 150., 80.],
            [150., 50., 50.],
            [150., 150., 50.],
        ][index];
        for (p, value) in [
            &p.mixer_red_out_red_in,
            &p.mixer_green_out_green_in,
            &p.mixer_blue_out_blue_in,
        ]
        .into_iter()
        .zip(rgb)
        {
            require(
                p.override_state == (index > 0) && p.value == value,
                "glasses source channel mixer",
            )?;
        }
        for (p, expected, default) in [
            (&p.ldr_lut, Some(sid("sharedassets0.assets", 255)), 4),
            (&p.external_lut, None, 1),
        ] {
            require(
                p.value == expected
                    && !p.override_state
                    && p.default_state == default
                    && match (&p.value, p.texture) {
                        (Some(s), Some(id)) => m
                            .lighting
                            .textures
                            .iter()
                            .any(|t| t.id == id && &t.source == s),
                        (None, None) => true,
                        _ => false,
                    },
                "glasses profile resource",
            )?;
        }
        Ok(())
    }
}
