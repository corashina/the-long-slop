//! Source rock heat sounds: `soundplayscript.Update` and `PlaySound`.
use super::{AudioAction, AudioKind, AudioState, SurvivalAudioEvent, xorshift};
use crate::lighting::{CycleOutput, CyclePhase};
use crate::reference::AmbienceReference;
use crate::world::{GeneratedId, StructureInstance};
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
}
