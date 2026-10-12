use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioClipReference {
    pub id: u32,
    pub source: SourceId,
    pub path: String,
    pub sha256: String,
    pub duration_seconds: f64,
    pub frames: u64,
    pub frequency: u32,
    pub channels: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioCurveKey {
    pub time: f32,
    pub value: f32,
    #[serde(rename = "inSlope")]
    pub in_slope: f32,
    #[serde(rename = "outSlope")]
    pub out_slope: f32,
    #[serde(rename = "inWeight")]
    pub in_weight: f32,
    #[serde(rename = "outWeight")]
    pub out_weight: f32,
    #[serde(rename = "weightedMode")]
    pub weighted_mode: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioBinding {
    pub role: String,
    pub kind: u32,
    pub source: SourceId,
    pub audio_source: SourceId,
    pub transform: SourceId,
    pub offset: [f32; 3],
    pub clips: Vec<u32>,
    pub gain: [f32; 2],
    pub pitch: [f32; 2],
    pub looped: bool,
    pub spatial_blend: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff_mode: u32,
    pub rolloff_curve: Vec<AudioCurveKey>,
    pub priority: u32,
    pub mixer: SourceId,
    pub mixer_route: Vec<u32>,
    pub mixer_gain: f32,
    pub mixer_pitch: f32,
    pub clip_length_lifetime: bool,
    pub doppler: f32,
    pub spread: f32,
    pub pan: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioReference {
    pub bindings: Vec<AudioBinding>,
    pub clips: Vec<AudioClipReference>,
    pub manager: SourceId,
    pub real_voices: usize,
    pub virtual_voices: usize,
    pub queue_capacity: usize,
    pub evidence: ManagedEvidence,
}
impl AudioReference {
    pub fn binding(&self, kind: u32) -> &AudioBinding {
        self.bindings
            .iter()
            .find(|b| b.kind == kind)
            .expect("validated audio binding")
    }
    pub fn validate(&self, _manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.bindings.len() == 9 && !self.clips.is_empty(),
            "selected audio bindings",
        )?;
        require(
            self.real_voices == 128 && self.virtual_voices == 512 && self.queue_capacity == 512,
            "audio voice bounds",
        )?;
        require(
            self.manager.file == "globalgamemanagers" && self.manager.path_id == 4,
            "audio manager",
        )?;
        require(
            self.evidence.assembly_sha256
                == "9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e"
                && !self.evidence.methods.is_empty(),
            "audio evidence",
        )?;
        for (i, c) in self.clips.iter().enumerate() {
            require(
                c.id == i as u32
                    && hash(&c.sha256)
                    && c.path
                        == format!(
                            "reference/survival-audio-{}-{}.wav",
                            c.source.file, c.source.path_id
                        )
                    && c.source.path_id > 0
                    && c.frames > 0
                    && c.frequency > 0
                    && c.channels > 0
                    && c.duration_seconds.is_finite()
                    && c.duration_seconds > 0.
                    && (c.duration_seconds - c.frames as f64 / c.frequency as f64).abs() < 1e-6,
                "audio WAV receipt",
            )?;
        }
        for (kind, role) in [
            (0, "eat"),
            (1, "burp"),
            (2, "fart"),
            (4, "sleep"),
            (5, "death"),
            (6, "storm"),
            (7, "ambient_day"),
            (8, "ambient_night"),
            (9, "shit"),
        ] {
            let entries: Vec<_> = self
                .bindings
                .iter()
                .filter(|b| b.kind == kind && b.role == role)
                .collect();
            require(entries.len() == 1, "audio role")?;
            let b = entries[0];
            require(
                !b.clips.is_empty()
                    && b.clips.iter().all(|c| (*c as usize) < self.clips.len())
                    && b.source.path_id > 0
                    && b.audio_source.path_id > 0
                    && b.transform.path_id > 0
                    && b.mixer.path_id > 0
                    && !b.mixer_route.is_empty(),
                "audio pointers",
            )?;
            require(
                finite(&b.offset)
                    && finite(&b.gain)
                    && finite(&b.pitch)
                    && b.gain[0] >= 0.
                    && b.gain[1] >= b.gain[0]
                    && b.pitch[0] > 0.
                    && b.pitch[1] >= b.pitch[0]
                    && finite(&[
                        b.spatial_blend,
                        b.min_distance,
                        b.max_distance,
                        b.mixer_gain,
                        b.mixer_pitch,
                        b.doppler,
                        b.spread,
                        b.pan,
                    ])
                    && (b.spatial_blend == 0. || b.spatial_blend == 1.)
                    && b.min_distance > 0.
                    && b.max_distance > b.min_distance
                    && b.rolloff_mode <= 2
                    && b.priority <= 256
                    && b.mixer_gain >= 0.
                    && b.mixer_pitch > 0.
                    && b.spread == 0.
                    && b.pan == 0.,
                "audio playback values",
            )?;
            require(
                !b.rolloff_curve.is_empty()
                    && b.rolloff_curve.iter().all(|k| {
                        finite(&[
                            k.time,
                            k.value,
                            k.in_slope,
                            k.out_slope,
                            k.in_weight,
                            k.out_weight,
                        ]) && k.weighted_mode == 0
                    })
                    && b.rolloff_curve.windows(2).all(|p| p[1].time > p[0].time),
                "audio attenuation curve",
            )?;
        }
        Ok(())
    }
}
