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
pub struct AudioNativeEngine {
    pub file: String,
    pub version: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioDopplerReference {
    pub factor: f32,
    pub speed_of_sound: f32,
    pub minimum_pitch: f32,
    pub velocity_mode: String,
    pub engine: AudioNativeEngine,
    pub method: String,
    pub method_rva: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioPlaybackReference {
    pub doppler: AudioDopplerReference,
    pub bindings: Vec<AudioBinding>,
    pub clips: Vec<AudioClipReference>,
    pub manager: SourceId,
    pub real_voices: usize,
    pub virtual_voices: usize,
    pub queue_capacity: usize,
    pub evidence: ManagedEvidence,
}
impl AudioPlaybackReference {
    pub fn binding(&self, kind: u32) -> &AudioBinding {
        self.bindings
            .iter()
            .find(|b| b.kind == kind)
            .expect("validated audio binding")
    }
    pub fn validate(&self) -> Result<(), ReferenceError> {
        let d = &self.doppler;
        require(
            d.factor == 1.
                && d.speed_of_sound == 340.
                && d.minimum_pitch == 1e-6
                && d.velocity_mode == "transform_delta"
                && d.method == "AudioSource::CacheFrameParameters"
                && d.method_rva == 0x9a5840
                && d.engine.file == "UnityPlayer.dll"
                && d.engine.version == "2021.3.18f1 (3129e69bc0c7)"
                && d.engine.sha256
                    == "553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86",
            "native Doppler evidence",
        )?;

        require(
            self.bindings.len() == 10 && !self.clips.is_empty(),
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
            (10, "storm_inside"),
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
                    && b.doppler >= 0.
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

// Required clip identity and timing stay typed even when playback routes are invalid.
#[derive(Clone, Debug, Deserialize)]
pub struct AudioGateClip {
    pub id: u32,
    pub source: SourceId,
    pub duration_seconds: f64,
    pub frames: u64,
    pub frequency: u32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct AudioGateBinding {
    pub role: String,
    pub kind: u32,
    pub clips: Vec<u32>,
    pub pitch: [f32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value", into = "serde_json::Value")]
pub struct AudioReference {
    pub bindings: Vec<AudioGateBinding>,
    pub clips: Vec<AudioGateClip>,
    playback: Result<AudioPlaybackReference, String>,
    record: serde_json::Value,
}
impl TryFrom<serde_json::Value> for AudioReference {
    type Error = String;
    fn try_from(record: serde_json::Value) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Gates {
            bindings: Vec<AudioGateBinding>,
            clips: Vec<AudioGateClip>,
        }
        let gates: Gates = serde_json::from_value(record.clone()).map_err(|e| e.to_string())?;
        let playback = serde_json::from_value::<AudioPlaybackReference>(record.clone())
            .map_err(|e| e.to_string())
            .and_then(|p| {
                p.validate().map_err(|e| e.to_string())?;
                Ok(p)
            });
        let result = Self {
            bindings: gates.bindings,
            clips: gates.clips,
            playback,
            record,
        };
        result.validate_gates().map_err(|e| e.to_string())?;
        Ok(result)
    }
}
impl From<AudioReference> for serde_json::Value {
    fn from(reference: AudioReference) -> Self {
        reference.record
    }
}
impl AudioReference {
    pub fn binding(&self, kind: u32) -> &AudioGateBinding {
        self.bindings
            .iter()
            .find(|b| b.kind == kind)
            .expect("validated audio gate")
    }
    pub fn playback(&self) -> Result<&AudioPlaybackReference, &str> {
        self.playback.as_ref().map_err(String::as_str)
    }
    pub fn validate(&self, _manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        self.validate_gates()?;
        self.playback()
            .map(|_| ())
            .map_err(|e| ReferenceError(e.into()))
    }
    pub fn validate_gates(&self) -> Result<(), ReferenceError> {
        require(
            self.bindings.len() == 10 && !self.clips.is_empty(),
            "audio gate count",
        )?;
        for (index, clip) in self.clips.iter().enumerate() {
            require(
                clip.id == index as u32
                    && clip.source.path_id > 0
                    && clip.frames > 0
                    && clip.frequency > 0
                    && clip.duration_seconds.is_finite()
                    && clip.duration_seconds > 0.
                    && (clip.duration_seconds - clip.frames as f64 / clip.frequency as f64).abs()
                        < 1e-6,
                "required source audio duration",
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
            (10, "storm_inside"),
        ] {
            require(
                self.bindings
                    .iter()
                    .filter(|b| b.kind == kind && b.role == role)
                    .count()
                    == 1,
                "audio gate role",
            )?;
            let b = self.binding(kind);
            require(
                !b.clips.is_empty()
                    && b.clips.iter().all(|id| (*id as usize) < self.clips.len())
                    && finite(&b.pitch)
                    && b.pitch[0] > 0.
                    && b.pitch[1] >= b.pitch[0],
                "audio gate clips and pitch",
            )?;
        }
        Ok(())
    }
}
