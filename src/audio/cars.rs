//! Car and engine part sounds. Every car copy plays through this one path.
//! Source `enginescript` loops and clips, `carscript` horn and switch, `talajnezo` wheels.
use super::{AudioAction, inverse_lerp};
use crate::reference::{
    CarAudioReference, CarShotReference, CarSoundReference, CarVoiceReference,
    EngineAudioReference, ItemId, SkidReference,
};
use crate::world::ObjectKey;
use rapier3d::math::{Pose, Vector};

/// Browser playback kind of car voice template 0. Template `id` plays as kind `CAR_KIND + id`.
pub const CAR_KIND: u32 = 0x100;
/// Browser route id of instance 0. The voices of instance `i` use route `CAR_ROUTE + i`.
pub const CAR_ROUTE: u32 = 0x6000_0000;
/// Car instances start here. Engine part instances are their item ids.
const CAR_INSTANCE: u32 = 0x80_0000;
/// Signal capacity: the loops of many cars in range.
const CAPACITY: usize = 256;

/// One car voice frame (a loop) or one clip (a one-shot) for the browser.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarSignal {
    pub kind: u32,
    pub instance: u32,
    pub action: AudioAction,
    pub clip: u32,
    pub gain: f32,
    pub pitch: f32,
    pub position: [f64; 3],
}

/// Gain and pitch of the `SAlap` loop at `rpm`.
pub fn idle_voice(a: &EngineAudioReference, rpm: f32) -> (f32, f32) {
    let [min, max] = a.idle_rpm;
    let [low, high] = a.idle_pitch;
    (
        inverse_lerp(max, min, rpm) * a.idle_gain,
        low + (high - low) * (inverse_lerp(0., max, rpm) + inverse_lerp(max, max * 2., rpm)),
    )
}

/// Gain and pitch of the `SMotor` loop at `rpm`. `max_rpm` is `enginescript.maxRpm`.
pub fn motor_voice(a: &EngineAudioReference, rpm: f32, max_rpm: f32) -> (f32, f32) {
    let [min, max] = a.motor_rpm;
    let [low, high] = a.motor_pitch;
    (
        inverse_lerp(min, max, rpm) * a.motor_gain,
        low + (high - low)
            * (inverse_lerp(min, max_rpm, rpm) + inverse_lerp(max_rpm, max_rpm * 2., rpm)),
    )
}

/// Inputs of one engine part this step.
pub struct EngineSoundInput<'a> {
    pub item: ItemId,
    pub audio: &'a EngineAudioReference,
    /// `enginescript.maxRpm`.
    pub max_rpm: f32,
    /// `enginescript.engineStartTimeModifier`.
    pub start_time_modifier: f32,
    pub rpm: f32,
    pub running: bool,
    pub starter: bool,
    /// The kick start plays the starter clip once per clip length.
    pub kick_start: bool,
    /// Engine part pose relative to the physics origin.
    pub pose: Pose,
    /// Mixer of the car that holds the engine. None: the template mixer.
    pub mixer: Option<u32>,
}

/// Inputs of one car this step.
pub struct CarSoundInput<'a> {
    pub audio: &'a CarAudioReference,
    /// Chassis pose relative to the physics origin.
    pub pose: Pose,
    /// A sleeping chassis is quiet.
    pub active: bool,
    pub horn: bool,
    pub ambient_temperature: f32,
    pub wheels: &'a crate::wheel_effects::WheelEffects,
    pub skid: &'a SkidReference,
    pub mixer: Option<u32>,
}

struct EngineSound {
    item: ItemId,
    running: bool,
    last_start: f64,
    seen: bool,
}

struct CarSound {
    key: ObjectKey,
    instance: u32,
    horn: f32,
    seen: bool,
}

/// Per-copy sound state and the signals of the current step.
pub struct CarSounds {
    engines: Vec<EngineSound>,
    cars: Vec<CarSound>,
    next: u32,
    /// Absolute physics origin of this step.
    origin: [f64; 3],
    /// Loops of this step. A loop that is missing stops.
    pub loops: Vec<CarSignal>,
    /// One-shots since the last browser frame.
    pub events: Vec<CarSignal>,
    /// `(instance, mixer)` of each routed car and engine part this step.
    pub routes: Vec<(u32, u32)>,
}

impl Default for CarSounds {
    fn default() -> Self {
        Self {
            engines: Vec::with_capacity(16),
            cars: Vec::with_capacity(16),
            next: 0,
            origin: [0.; 3],
            loops: Vec::with_capacity(CAPACITY),
            events: Vec::with_capacity(64),
            routes: Vec::with_capacity(32),
        }
    }
}

impl CarSounds {
    pub fn reset(&mut self) {
        self.engines.clear();
        self.cars.clear();
        self.loops.clear();
        self.events.clear();
        self.routes.clear();
    }
    /// Starts a step. Copies that the step does not name lose their state at `end`.
    pub fn begin(&mut self, origin: [f64; 3]) {
        self.origin = origin;
        self.loops.clear();
        self.routes.clear();
        for e in &mut self.engines {
            e.seen = false;
        }
        for c in &mut self.cars {
            c.seen = false;
        }
    }
    pub fn end(&mut self) {
        self.engines.retain(|e| e.seen);
        self.cars.retain(|c| c.seen);
    }
    fn absolute(&self, pose: Pose, position: [f32; 3]) -> [f64; 3] {
        let p = pose * Vector::from_array(position);
        std::array::from_fn(|i| self.origin[i] + f64::from(p[i]))
    }
    fn push_loop(
        &mut self,
        instance: u32,
        sound: &CarSoundReference,
        clip: u32,
        gain: f32,
        pitch: f32,
        pose: Pose,
    ) {
        if self.loops.len() < self.loops.capacity() {
            self.loops.push(CarSignal {
                kind: CAR_KIND + sound.voice,
                instance,
                action: AudioAction::Update,
                clip,
                gain,
                pitch,
                position: self.absolute(pose, sound.position),
            });
        }
    }
    fn push_shot(&mut self, instance: u32, shot: &CarShotReference, pose: Pose) {
        if self.events.len() < self.events.capacity() {
            self.events.push(CarSignal {
                kind: CAR_KIND + shot.voice,
                instance,
                action: AudioAction::Start,
                clip: shot.clip,
                gain: shot.gain,
                pitch: 1.,
                position: self.absolute(pose, shot.position),
            });
        }
    }
    /// Source `enginescript.Update` sounds of one engine part. Sound follows running, not
    /// attachment. `clip_seconds` returns a clip's source length.
    pub fn engine(
        &mut self,
        input: EngineSoundInput,
        templates: &[CarVoiceReference],
        clip_seconds: impl Fn(u32) -> f32,
        seconds: f64,
    ) {
        let a = input.audio;
        let index = match self.engines.iter().position(|e| e.item == input.item) {
            Some(index) => index,
            None => {
                // The first step syncs the running edge, so a restored engine does not stop.
                self.engines.push(EngineSound {
                    item: input.item,
                    running: input.running,
                    last_start: f64::NEG_INFINITY,
                    seen: true,
                });
                self.engines.len() - 1
            }
        };
        self.engines[index].seen = true;
        let instance = input.item.0;
        if let Some(mixer) = input.mixer {
            self.routes.push((instance, mixer));
        }
        if input.running {
            let (gain, pitch) = idle_voice(a, input.rpm);
            self.push_loop(
                instance,
                &a.idle,
                template_clip(templates, a.idle.voice),
                gain,
                pitch,
                input.pose,
            );
            let (gain, pitch) = motor_voice(a, input.rpm, input.max_rpm);
            self.push_loop(
                instance,
                &a.motor,
                template_clip(templates, a.motor.voice),
                gain,
                pitch,
                input.pose,
            );
        }
        if self.engines[index].running
            && !input.running
            && let Some(stop) = &a.stop
        {
            self.push_shot(instance, stop, input.pose);
        }
        self.engines[index].running = input.running;
        // The starter clip repeats each clip length times engineStartTimeModifier.
        // The kick start repeats it each clip length.
        let modifier = if input.starter {
            Some(input.start_time_modifier)
        } else if input.kick_start {
            Some(1.0)
        } else {
            None
        };
        if let Some(modifier) = modifier
            && let Some(starter) = &a.starter
            && seconds - self.engines[index].last_start
                > f64::from(clip_seconds(starter.clip) * modifier)
        {
            self.engines[index].last_start = seconds;
            self.push_shot(instance, starter, input.pose);
        }
    }
    /// The browser instance of car `key`, added on first use.
    fn car_index(&mut self, key: &ObjectKey) -> usize {
        if let Some(index) = self.cars.iter().position(|c| &c.key == key) {
            return index;
        }
        self.cars.push(CarSound {
            key: key.clone(),
            instance: CAR_INSTANCE + self.next,
            horn: 0.,
            seen: true,
        });
        self.next = (self.next + 1) % CAR_INSTANCE;
        self.cars.len() - 1
    }
    /// Source `carscript` horn and `talajnezo` wheel loops of car `key`.
    pub fn car(
        &mut self,
        key: &ObjectKey,
        input: CarSoundInput,
        templates: &[CarVoiceReference],
        dt: f32,
    ) {
        let index = self.car_index(key);
        self.cars[index].seen = true;
        let instance = self.cars[index].instance;
        let a = input.audio;
        if let Some(mixer) = input.mixer {
            self.routes.push((instance, mixer));
        }
        if let Some(h) = &a.horn {
            let target = if input.horn { 1. } else { 0. };
            let step = h.attack_release_rate * dt;
            let gain = &mut self.cars[index].horn;
            *gain += (target - *gain).clamp(-step, step);
            let gain = *gain;
            if gain > 0. {
                let t = inverse_lerp(
                    h.temperature[0],
                    h.temperature[1],
                    input.ambient_temperature,
                );
                let pitch = h.pitch[0] + (h.pitch[1] - h.pitch[0]) * t;
                self.push_loop(
                    instance,
                    &h.sound,
                    template_clip(templates, h.sound.voice),
                    gain,
                    pitch,
                    input.pose,
                );
            }
        }
        if !input.active {
            return;
        }
        for (w, r) in a.wheels.iter().enumerate() {
            let s = input.wheels.sound(w);
            if s.ground_playing {
                // talajnezo does not set the GroundSource pitch.
                let template = &templates[r.ground.voice as usize - 1];
                let clip = template.clip.unwrap_or(0);
                self.push_loop(
                    instance,
                    &r.ground,
                    clip,
                    s.ground_gain,
                    template.settings.pitch,
                    input.pose,
                );
            }
            if s.spring_playing {
                let clip = template_clip(templates, r.spring.voice);
                self.push_loop(
                    instance,
                    &r.spring,
                    clip,
                    s.spring_gain,
                    s.spring_pitch,
                    input.pose,
                );
            }
            if s.slide_playing {
                let clip = if s.slide_road {
                    input.skid.road_clip
                } else {
                    input.skid.dirt_clip
                };
                self.push_loop(
                    instance,
                    &r.slide,
                    clip,
                    s.slide_gain,
                    s.slide_pitch,
                    input.pose,
                );
            }
        }
    }
    /// A `carscript` one-shot of car `key` at chassis pose `pose`.
    pub fn car_shot(&mut self, key: &ObjectKey, shot: &CarShotReference, pose: Pose) {
        let instance = self
            .cars
            .iter()
            .find(|c| &c.key == key)
            .map_or(0, |c| c.instance);
        self.push_shot(instance, shot, pose);
    }
}

/// The authored clip of a loop template. Validation requires one.
fn template_clip(templates: &[CarVoiceReference], voice: u32) -> u32 {
    templates[voice as usize - 1].clip.unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_audio() -> EngineAudioReference {
        serde_json::from_value(crate::reference::engine_audio_fixture()).unwrap()
    }

    fn template(id: u32, clip: u32, one_shot: bool) -> CarVoiceReference {
        let curve = serde_json::json!({"keys": [{"time": 0.0, "value": 1.0, "in_tangent": 0.0,
            "out_tangent": 0.0, "in_weight": 0.0, "out_weight": 0.0, "weighted_mode": 0}],
            "pre_wrap_mode": 2, "post_wrap_mode": 2, "rotation_order": 4});
        serde_json::from_value(
            serde_json::json!({"id": id, "source": {"file": "a", "path_id": id},
            "one_shot": one_shot, "clip": clip, "mixer": 1, "doppler": 1.0, "pan": 0.0,
            "settings": {"priority": 80, "loop": !one_shot, "gain": 1.0, "pitch": 1.0,
                "min_distance": 1.0, "max_distance": 50.0, "rolloff_mode": 1, "spatial_blend": 1.0,
                "spatial_blend_curve": curve, "rolloff_curve": curve}}),
        )
        .unwrap()
    }

    /// A world car engine part: the record rpm drives the motor loop; the engine part pose
    /// places it; a stop plays the stop clip once.
    #[test]
    fn engine_part_motor_loop_follows_rpm() {
        let audio = engine_audio();
        let templates = [
            template(1, 5, false),
            template(2, 6, false),
            template(3, 0, true),
        ];
        let mut sounds = CarSounds::default();
        let pose = Pose::from_translation(Vector::new(2., 0., -1.));
        let step = |sounds: &mut CarSounds, rpm: f32, running: bool, seconds: f64| {
            sounds.begin([100., 0., 0.]);
            sounds.engine(
                EngineSoundInput {
                    item: ItemId(9),
                    audio: &audio,
                    max_rpm: 6000.,
                    start_time_modifier: 1.,
                    rpm,
                    running,
                    starter: false,
                    kick_start: false,
                    pose,
                    mixer: Some(4),
                },
                &templates,
                |_| 1.,
                seconds,
            );
            sounds.end();
        };
        step(&mut sounds, 900., true, 0.);
        let motor = sounds
            .loops
            .iter()
            .find(|s| s.kind == CAR_KIND + 2)
            .unwrap();
        // inverse_lerp(500, 1300, 900) * 0.8 and 0.8 + 0.7 * inverse_lerp(500, 6000, 900).
        assert!((motor.gain - 0.4).abs() < 1e-6, "{}", motor.gain);
        assert!((motor.pitch - (0.8 + 0.7 * 400. / 5500.)).abs() < 1e-6);
        assert_eq!((motor.clip, motor.instance), (6, 9));
        assert_eq!(motor.position, [102., 0.5, -1.]);
        assert_eq!(sounds.routes, [(9, 4)]);
        step(&mut sounds, 3000., true, 0.1);
        let motor = sounds
            .loops
            .iter()
            .find(|s| s.kind == CAR_KIND + 2)
            .unwrap();
        assert_eq!(motor.gain, 0.8, "full gain above SMaxMotorRpm");
        let idle = sounds
            .loops
            .iter()
            .find(|s| s.kind == CAR_KIND + 1)
            .unwrap();
        assert_eq!(idle.gain, 0., "idle fades out above SMaxAlapRpm");
        assert!(sounds.events.is_empty());
        step(&mut sounds, 0., false, 0.2);
        assert!(sounds.loops.is_empty(), "a stopped engine has no loops");
        assert_eq!(sounds.events.len(), 1);
        assert_eq!(
            (sounds.events[0].clip, sounds.events[0].action),
            (8, AudioAction::Start)
        );
    }
}
