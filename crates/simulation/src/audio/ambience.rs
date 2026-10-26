//! Source rock heat sounds (`soundplayscript`) and ambience zones (`ambienceZone`).
use super::{AudioAction, AudioKind, AudioState, SurvivalAudioEvent, xorshift};
use crate::lighting::{CycleOutput, CyclePhase};
use crate::reference::{AmbienceReference, AmbienceZoneReference};
use crate::world::{GeneratedId, StructureInstance, place_point};
use std::collections::HashMap;

const CAPACITY: usize = 256;

struct RockTimer {
    ambience: usize,
    position: [f64; 3],
    last: f64,
    wait: f64,
    seen: bool,
}

/// One source timer per published placement of an ambience prototype.
pub struct RockAmbience {
    timers: HashMap<GeneratedId, RockTimer>,
    revision: Option<u64>,
}

impl Default for RockAmbience {
    fn default() -> Self {
        Self {
            timers: HashMap::with_capacity(CAPACITY),
            revision: None,
        }
    }
}

/// True when `lerp` is inside the source window of the current phase.
fn window(a: &AmbienceReference, phase: CyclePhase, lerp: f32) -> bool {
    let w = match phase {
        CyclePhase::Dawn => a.phases.morning,
        CyclePhase::Day => a.phases.day,
        CyclePhase::Dusk => a.phases.dusk,
        CyclePhase::Night => a.phases.night,
    };
    w.is_some_and(|[min, max]| lerp > min && lerp < max)
}

impl RockAmbience {
    pub fn reset(&mut self) {
        self.timers.clear();
        self.revision = None;
    }
    /// A new timer tries on its first step inside a window, as the source timer starts at zero.
    fn add(&mut self, id: &GeneratedId, ambience: usize, position: [f64; 3], now: f64) {
        if let Some(timer) = self.timers.get_mut(id) {
            timer.seen = true;
        } else if self.timers.len() < CAPACITY {
            let timer = RockTimer {
                ambience,
                position,
                last: now,
                wait: 0.,
                seen: true,
            };
            self.timers.insert(id.clone(), timer);
        }
    }
    /// Matches the timers to the published structures when the stream revision changes.
    pub fn sync(
        &mut self,
        revision: u64,
        structures: &[StructureInstance],
        ambience: &[AmbienceReference],
        now: f64,
    ) {
        if self.revision == Some(revision) {
            return;
        }
        self.revision = Some(revision);
        for timer in self.timers.values_mut() {
            timer.seen = false;
        }
        for s in structures {
            let p = &s.placement;
            if let Some(i) = ambience.iter().position(|a| a.prototype == p.prototype) {
                self.add(&p.id, i, p.pose.position, now);
            }
        }
        self.timers.retain(|_, t| t.seen);
    }
    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        ambience: &[AmbienceReference],
        phase: CyclePhase,
        lerp: f32,
        now: f64,
        listener: [f64; 3],
        range: f32,
        random: &mut u32,
        mut emit: impl FnMut([f64; 3], u32, f32),
    ) {
        let range = f64::from(range);
        for t in self.timers.values_mut() {
            let d: f64 = (0..3).map(|i| (t.position[i] - listener[i]).powi(2)).sum();
            let a = &ambience[t.ambience];
            // The source tests the phase before it resets the timer.
            if d > range * range || now - t.last <= t.wait || !window(a, phase, lerp) {
                continue;
            }
            t.last = now;
            t.wait = f64::from(a.interval[0] + (a.interval[1] - a.interval[0]) * xorshift(random));
            if xorshift(random) > a.chance {
                continue;
            }
            let index = ((xorshift(random) * a.clips.len() as f32) as usize).min(a.clips.len() - 1);
            let gain = a.gain[0] + (a.gain[1] - a.gain[0]) * xorshift(random);
            emit(t.position, a.clips[index], gain);
        }
    }
}

const ZONE_CAPACITY: usize = 16;

struct Zone {
    id: GeneratedId,
    zone: usize,
    position: [f64; 3],
    playing: bool,
    seen: bool,
}

/// One source zone per published placement. The slot index is the loop emitter.
pub struct ZoneAmbience {
    slots: [Option<Zone>; ZONE_CAPACITY],
    revision: Option<u64>,
}

impl Default for ZoneAmbience {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            revision: None,
        }
    }
}

/// The result of one source `FixedUpdate` for one zone slot.
#[derive(Debug, PartialEq)]
enum ZoneStep {
    Idle,
    Play {
        gain: f32,
        lerp: f32,
        position: [f64; 3],
    },
    Stop,
}

impl ZoneAmbience {
    pub fn reset(&mut self) {
        self.slots = std::array::from_fn(|_| None);
        self.revision = None;
    }
    /// Matches the zones to the published structures. Calls `stop` for each removed playing zone.
    fn sync(
        &mut self,
        revision: u64,
        structures: &[StructureInstance],
        zones: &[AmbienceZoneReference],
        mut stop: impl FnMut(u32),
    ) {
        if self.revision == Some(revision) {
            return;
        }
        self.revision = Some(revision);
        for z in self.slots.iter_mut().flatten() {
            z.seen = false;
        }
        for s in structures {
            let p = &s.placement;
            for (i, zone) in zones.iter().enumerate() {
                if zone.prototype != p.prototype {
                    continue;
                }
                let slots = &mut self.slots;
                if let Some(z) = slots
                    .iter_mut()
                    .flatten()
                    .find(|z| z.zone == i && z.id == p.id)
                {
                    z.seen = true;
                } else if let Some(slot) = slots.iter_mut().find(|z| z.is_none()) {
                    *slot = Some(Zone {
                        id: p.id.clone(),
                        zone: i,
                        position: place_point(p.pose, p.scale, zone.pose.translation),
                        playing: false,
                        seen: true,
                    });
                }
            }
        }
        for (emitter, slot) in self.slots.iter_mut().enumerate() {
            if slot.as_ref().is_some_and(|z| !z.seen) && slot.take().is_some_and(|z| z.playing) {
                stop(emitter as u32);
            }
        }
    }
    /// Source order: a stopped zone starts on this step and sets its gain on the next step.
    fn step(
        &mut self,
        slot: usize,
        zones: &[AmbienceZoneReference],
        listener: [f64; 3],
    ) -> ZoneStep {
        let Some(z) = &mut self.slots[slot] else {
            return ZoneStep::Idle;
        };
        let zone = &zones[z.zone];
        let d = (0..3)
            .map(|i| (z.position[i] - listener[i]).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        // Mathf.InverseLerp(maxDist, minDist, distance).
        let lerp =
            ((d - zone.max_distance) / (zone.min_distance - zone.max_distance)).clamp(0., 1.);
        if z.playing {
            if lerp == 0. {
                z.playing = false;
                return ZoneStep::Stop;
            }
            return ZoneStep::Play {
                gain: zone.volume * lerp,
                lerp,
                position: z.position,
            };
        }
        z.playing = lerp > 0.;
        ZoneStep::Idle
    }
}

impl AudioState {
    pub fn sync_zones(&mut self, revision: u64, structures: &[StructureInstance]) {
        let loops = &mut self.survival_loops;
        self.zones.sync(
            revision,
            structures,
            &self.reference.ambience_zones,
            |emitter| loops.retain(|v| v.kind != AudioKind::AmbienceZone || v.emitter != emitter),
        );
    }
    /// Plays the zones near `listener`. Returns the largest playing lerp for the ambient duck.
    pub fn zone_step(&mut self, listener: [f64; 3]) -> f32 {
        let mut duck = 0f32;
        for slot in 0..ZONE_CAPACITY {
            match self
                .zones
                .step(slot, &self.reference.ambience_zones, listener)
            {
                ZoneStep::Idle => {}
                ZoneStep::Play {
                    gain,
                    lerp,
                    position,
                } => {
                    duck = duck.max(lerp);
                    self.source_loop(AudioKind::AmbienceZone, slot as u32, gain, position);
                }
                ZoneStep::Stop => self.stop_survival_loop(AudioKind::AmbienceZone, slot as u32),
            }
        }
        duck
    }
}

impl AudioState {
    pub fn sync_rocks(&mut self, revision: u64, structures: &[StructureInstance]) {
        let now = self.game_seconds;
        self.rocks
            .sync(revision, structures, &self.reference.ambience, now);
    }
    /// Emits rock heat sounds for timers within the shared hearing range of `listener`.
    pub fn rock_step(&mut self, cycle: &CycleOutput, listener: [f64; 3]) {
        let session = self.session;
        let events = &mut self.survival_events;
        self.rocks.step(
            &self.reference.ambience,
            cycle.phase,
            cycle.progress,
            self.game_seconds,
            listener,
            self.reference.shared.settings.max_distance,
            &mut self.random,
            |position, clip, gain| {
                if events.len() < events.capacity() {
                    events.push(SurvivalAudioEvent {
                        session,
                        emitter: 0,
                        kind: AudioKind::RockHeat,
                        action: AudioAction::Start,
                        clip,
                        gain,
                        pitch: 1.,
                        position,
                    });
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::SourceId;
    use crate::world::{Placement, WorldBounds, WorldLayer};

    fn rock() -> AmbienceReference {
        let method =
            r#"{"class":"c","method":"m","file":"f","line_start":1,"line_end":2,"sha256":""}"#;
        serde_json::from_str(&format!(
            r#"{{"prototype":{{"file":"s","path_id":7}},"clips":[3,4],
            "phases":{{"dusk":[0.5,1.0],"night":[0.5,1.0]}},"interval":[10,30],"chance":0.5,
            "gain":[0.5,1],"pitch":[1,1],"priority":100,"method":{method}}}"#
        ))
        .unwrap()
    }
    fn id(ordinal: u32) -> GeneratedId {
        GeneratedId {
            layer: WorldLayer::Prop,
            generator: SourceId {
                file: "s".into(),
                path_id: 1,
            },
            unit: [0, 0],
            ordinal,
            children: Vec::new(),
        }
    }
    fn structure(ordinal: u32, path_id: i64) -> StructureInstance {
        StructureInstance {
            fixed_decoration: false,
            cpu_bytes: 0,
            placement: Placement {
                id: id(ordinal),
                prototype: SourceId {
                    file: "s".into(),
                    path_id,
                },
                pose: crate::world::AbsolutePose {
                    position: [ordinal as f64, 0., 0.],
                    rotation: [0., 0., 0., 1.],
                },
                scale: [1.; 3],
                bounds: WorldBounds {
                    min: [0.; 3],
                    max: [0.; 3],
                },
                object_seed: 0,
            },
            bodies: Vec::new(),
            colliders: Vec::new(),
        }
    }
    /// Steps one timer and returns the played gains.
    fn run(
        state: &mut RockAmbience,
        phase: CyclePhase,
        lerp: f32,
        now: f64,
        random: &mut u32,
    ) -> usize {
        let mut played = 0;
        state.step(
            &[rock()],
            phase,
            lerp,
            now,
            [0.; 3],
            50.,
            random,
            |_, clip, gain| {
                assert!([3, 4].contains(&clip) && (0.5..=1.).contains(&gain));
                played += 1;
            },
        );
        played
    }
    fn single() -> RockAmbience {
        let mut state = RockAmbience::default();
        state.sync(1, &[structure(1, 7)], &[rock()], 0.);
        state
    }
    fn timer(state: &RockAmbience) -> (f64, f64) {
        let t = &state.timers[&id(1)];
        (t.last, t.wait)
    }

    #[test]
    fn plays_only_in_phase_windows() {
        let mut random = 0x9e3779b9;
        for (phase, lerp, expected) in [
            (CyclePhase::Day, 0.7, false),
            (CyclePhase::Dawn, 0.7, false),
            (CyclePhase::Dusk, 0.5, false),
            (CyclePhase::Dusk, 0.7, true),
            (CyclePhase::Night, 0.9, true),
        ] {
            let mut state = single();
            let mut played = 0;
            for step in 1..=10_000 {
                played += run(&mut state, phase, lerp, f64::from(step) * 0.02, &mut random);
            }
            // 200 s with 10 to 30 s waits and a 0.5 chance gives about 5 sounds.
            assert_eq!(played > 0, expected, "{phase:?} {lerp}");
            if !expected {
                assert_eq!(timer(&state), (0., 0.), "no reset outside a window");
            }
        }
    }

    #[test]
    fn failed_roll_resets_timer() {
        let mut random = 0x9e3779b9;
        let mut state = single();
        // Find a random state whose chance roll fails.
        loop {
            let mut probe = random;
            xorshift(&mut probe);
            if xorshift(&mut probe) > 0.5 {
                break;
            }
            xorshift(&mut random);
        }
        assert_eq!(run(&mut state, CyclePhase::Night, 0.9, 1., &mut random), 0);
        let (last, wait) = timer(&state);
        assert_eq!(last, 1.);
        assert!((10. ..=30.).contains(&wait));
        assert_eq!(
            run(&mut state, CyclePhase::Night, 0.9, 1. + wait, &mut random),
            0
        );
        assert_eq!(timer(&state).0, 1., "waits for the interval");
    }

    #[test]
    fn sync_follows_published_structures() {
        let mut state = RockAmbience::default();
        let a = [structure(1, 7), structure(2, 7), structure(3, 8)];
        state.sync(1, &a, &[rock()], 0.);
        assert_eq!(state.timers.len(), 2);
        state.timers.get_mut(&id(1)).unwrap().last = 5.;
        state.sync(2, &a[..1], &[rock()], 9.);
        assert_eq!(state.timers.len(), 1);
        assert_eq!(timer(&state).0, 5., "kept timers keep their state");
        state.sync(3, &[], &[rock()], 9.);
        assert!(state.timers.is_empty());
    }

    fn zone() -> AmbienceZoneReference {
        let method =
            r#"{"class":"c","method":"m","file":"f","line_start":1,"line_end":2,"sha256":""}"#;
        serde_json::from_str(&format!(
            r#"{{"prototype":{{"file":"s","path_id":7}},
            "pose":{{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]}},
            "min_distance":200,"max_distance":1000,"volume":0.5,"clip":3,"method":{method}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn zone_follows_source_lerp_and_order() {
        let zones = [zone()];
        let mut state = ZoneAmbience::default();
        let mut stops = 0;
        state.sync(1, &[structure(0, 7)], &zones, |_| stops += 1);
        let mut step = |d: f64| state.step(0, &zones, [d, 0., 0.]);
        assert_eq!(step(1200.), ZoneStep::Idle, "no start at lerp 0");
        assert_eq!(
            step(600.),
            ZoneStep::Idle,
            "starts; gain from the next step"
        );
        let play = |gain, lerp| ZoneStep::Play {
            gain,
            lerp,
            position: [0.; 3],
        };
        assert_eq!(step(600.), play(0.25, 0.5));
        assert_eq!(step(0.), play(0.5, 1.));
        assert_eq!(step(1200.), ZoneStep::Stop);
        assert_eq!(step(1200.), ZoneStep::Idle);
        step(0.);
        state.sync(2, &[], &zones, |_| stops += 1);
        assert_eq!(stops, 1, "a removed playing zone stops");
        assert_eq!(state.step(0, &zones, [0.; 3]), ZoneStep::Idle);
    }

    #[test]
    fn duck_recovers_at_source_rate() {
        let reference: crate::reference::AmbientReference = serde_json::from_str(
            r#"{"source":{"file":"level1","path_id":1},"day_volume":[1,1],
            "night_volume":[1,1],"modifier":1,"only_modifier":1,"recovery_per_second":0.1}"#,
        )
        .unwrap();
        let cycle = CycleOutput {
            phase: CyclePhase::Day,
            progress: 0.,
            cycle_seconds: 0.,
            directional_euler_degrees: [0.; 3],
            directional_color: [0.; 4],
            directional_intensity: 0.,
            ambient: 0.,
            reflection_intensity: 0.,
            sky_exposure: 0.,
            sky_tint: [0.; 4],
            ground_color: [0.; 4],
            atmosphere_thickness: 0.,
            fog_density: 0.,
            fog_color: [0.; 4],
            roadside_enabled: false,
            biome_color: [1., 0., 0., 0.],
            ambient_temperature: 0.,
            clouds: Default::default(),
            cloud_noise_coordinate: 0.,
            cloud_noise: 0.,
            star_color: [0.; 4],
            star_rotation_degrees: 0.,
            flare_progress: 0.,
            flare_brightness: 0.,
            shader_time_seconds: 0.,
            grading_temperature: None,
        };
        let mut ambient = crate::audio::AmbientState::new(&reference);
        ambient.duck(0.5);
        assert_eq!(ambient.only_modifier, 0.5);
        ambient.duck(0.2);
        assert_eq!(ambient.only_modifier, 0.5, "the duck keeps the minimum");
        assert_eq!(ambient.gains(&reference, &cycle, 0., 1.), [0.5; 2]);
        assert!((ambient.only_modifier - 0.6).abs() < 1e-6);
    }
}
