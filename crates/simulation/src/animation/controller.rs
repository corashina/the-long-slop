//! Source Animator state machine for one checked layer: triggers, the `Speed` parameter,
//! AnyState and state transitions, exit times, fixed-duration crossfades and clip sampling.
//! Transitions have interruption source None: a crossfade runs to its end.
use super::clip::{LocalPart, sample_clip};
use crate::reference::{MeleeMotion, MotionTransition};
use crate::vehicle::VehiclePose;
use rapier3d::math::{Rotation, Vector};

/// State behaviour callbacks (`StateMachineBehaviour.OnStateEnter` and `OnStateExit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateEvent {
    Enter(usize),
    Exit(usize),
}

#[derive(Clone, Copy, Debug)]
struct Playing {
    state: usize,
    /// Normalized time.
    time: f32,
}

#[derive(Clone, Copy, Debug)]
struct Crossfade {
    next: Playing,
    elapsed: f32,
    duration: f32,
}

#[derive(Debug)]
pub struct MotionController {
    triggers: u64,
    speed: f32,
    current: Playing,
    crossfade: Option<Crossfade>,
}
impl MotionController {
    pub fn new(m: &MeleeMotion) -> Self {
        Self {
            triggers: 0,
            speed: 0.,
            current: Playing {
                state: m.default_state as usize,
                time: 0.,
            },
            crossfade: None,
        }
    }
    /// The Animator is off: it restarts in the default state.
    pub fn reset(&mut self, m: &MeleeMotion) {
        *self = Self::new(m);
    }
    /// Parameter index for `SetTrigger` and `ResetTrigger`.
    pub fn parameter(m: &MeleeMotion, name: &str) -> Option<usize> {
        m.parameters.iter().position(|p| p.name == name)
    }
    pub fn set_trigger(&mut self, parameter: usize) {
        self.triggers |= 1 << parameter;
    }
    pub fn reset_trigger(&mut self, parameter: usize) {
        self.triggers &= !(1 << parameter);
    }
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
    pub fn state(&self) -> usize {
        self.current.state
    }
    fn trigger(&self, m: &MeleeMotion, id: u32) -> Option<usize> {
        m.parameters.iter().position(|p| p.id == id)
    }
    fn length(m: &MeleeMotion, state: usize) -> f32 {
        let s = &m.states[state];
        let c = &m.clips[s.clip_slot as usize];
        let length = c.stop_time - c.start_time;
        if length > 0. { length } else { s.clip_duration }
    }
    fn state_speed(&self, m: &MeleeMotion, state: usize) -> f32 {
        let s = &m.states[state];
        if s.speed_parameter != 0 {
            s.speed * self.speed
        } else {
            s.speed
        }
    }
    fn step(&self, m: &MeleeMotion, playing: &mut Playing, dt: f32) {
        playing.time += dt * self.state_speed(m, playing.state) / Self::length(m, playing.state);
    }
    /// True when every condition trigger is set and the exit time, if any, passed in this step.
    fn ready(&self, m: &MeleeMotion, t: &MotionTransition, from: f32, to: f32) -> bool {
        if t.has_exit_time && !crossed(from, to, t.exit_time) {
            return false;
        }
        t.conditions.iter().all(|c| {
            self.trigger(m, c.parameter)
                .is_some_and(|p| self.triggers & (1 << p) != 0)
        })
    }
    pub fn advance(&mut self, m: &MeleeMotion, dt: f32, mut event: impl FnMut(StateEvent)) {
        let callback = |state: usize| m.states[state].callback.is_some();
        let from = self.current.time;
        let mut current = self.current;
        self.step(m, &mut current, dt);
        self.current = current;
        if let Some(mut fade) = self.crossfade {
            self.step(m, &mut fade.next, dt);
            fade.elapsed += dt;
            self.crossfade = Some(fade);
            if fade.elapsed >= fade.duration {
                if callback(self.current.state) {
                    event(StateEvent::Exit(self.current.state));
                }
                self.current = fade.next;
                self.crossfade = None;
            }
            return;
        }
        let state = self.current.state;
        let to = self.current.time;
        let Some(t) = m
            .any_state_transitions
            .iter()
            .filter(|t| t.can_transition_to_self || t.destination as usize != state)
            .chain(&m.states[state].transitions)
            .find(|t| self.ready(m, t, from, to))
        else {
            return;
        };
        for c in &t.conditions {
            if let Some(p) = self.trigger(m, c.parameter) {
                self.reset_trigger(p);
            }
        }
        let destination = t.destination as usize;
        let duration = if t.fixed_duration {
            t.duration
        } else {
            t.duration * Self::length(m, state)
        };
        let next = Playing {
            state: destination,
            time: t.offset,
        };
        if callback(destination) {
            event(StateEvent::Enter(destination));
        }
        if duration > 0. {
            self.crossfade = Some(Crossfade {
                next,
                elapsed: 0.,
                duration,
            });
        } else {
            if callback(state) {
                event(StateEvent::Exit(state));
            }
            self.current = next;
        }
    }
    fn sample(m: &MeleeMotion, playing: Playing) -> (Vector, Rotation) {
        let s = &m.states[playing.state];
        let clip = &m.clips[s.clip_slot as usize];
        let length = clip.stop_time - clip.start_time;
        let fraction = if s.r#loop {
            playing.time.rem_euclid(1.)
        } else {
            playing.time.clamp(0., 1.)
        };
        let time = clip.start_time + fraction * length.max(0.);
        let mut position = Vector::ZERO;
        let mut rotation = Rotation::IDENTITY;
        sample_clip(clip, time, |_, part| match part {
            LocalPart::Position(p) => position = p,
            LocalPart::Rotation(r) => rotation = r,
            LocalPart::Scale(_) => {}
        });
        (position, rotation)
    }
    /// Animated target pose in its parent frame, in converted space.
    pub fn pose(&self, m: &MeleeMotion) -> VehiclePose {
        let (mut position, mut rotation) = Self::sample(m, self.current);
        if let Some(fade) = self.crossfade {
            let weight = (fade.elapsed / fade.duration).clamp(0., 1.);
            let (p, r) = Self::sample(m, fade.next);
            position = position.lerp(p, weight);
            rotation = rotation.slerp(r, weight);
        }
        VehiclePose {
            position: position.to_array(),
            rotation: rotation.to_array(),
        }
    }
}

/// True when a loop of normalized time passes `exit` between `from` and `to`.
fn crossed(from: f32, to: f32, exit: f32) -> bool {
    if to <= from {
        return false;
    }
    if exit >= 1. {
        from < exit && exit <= to
    } else {
        (to - exit).floor() > (from - exit).floor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{
        MotionClip, MotionCondition, MotionParameter, MotionReceipt, MotionState, MotionTrack,
        NodeId, SourceId,
    };

    const SPEED: u32 = 7;
    const ATTACK: u32 = 9;

    fn source() -> SourceId {
        SourceId {
            file: "test".into(),
            path_id: 1,
        }
    }
    fn transition(destination: u32, exit_time: Option<f32>, trigger: bool) -> MotionTransition {
        MotionTransition {
            id: 0,
            full_path_id: 0,
            user_id: 0,
            destination,
            duration: 0.1,
            offset: 0.,
            exit_time: exit_time.unwrap_or(0.),
            has_exit_time: exit_time.is_some(),
            fixed_duration: true,
            can_transition_to_self: false,
            interruption_source: 0,
            ordered_interruption: true,
            conditions: trigger
                .then_some(MotionCondition {
                    mode: 1,
                    parameter: ATTACK,
                    threshold: 0.,
                    exit_time: 0.,
                })
                .into_iter()
                .collect(),
        }
    }
    fn state(
        clip_slot: u32,
        speed_parameter: u32,
        callback: bool,
        transitions: Vec<MotionTransition>,
    ) -> MotionState {
        MotionState {
            id: clip_slot,
            name_id: 0,
            name: "state".into(),
            path_id: 0,
            tag_id: 0,
            clip_slot,
            speed: 1.,
            speed_parameter,
            cycle_offset: 0.,
            cycle_offset_parameter: 0,
            time_parameter: 0,
            mirror_parameter: 0,
            mirror: false,
            r#loop: clip_slot == 0,
            ik_on_feet: false,
            write_default_values: true,
            clip_duration: 1.,
            clip_cycle_offset: 0.,
            clip_mirror: false,
            callback: callback.then(source),
            transitions,
        }
    }
    fn clip(stop_time: f32, z: f32) -> MotionClip {
        MotionClip {
            source: source(),
            source_tracks: MotionReceipt {
                bytes: 0,
                file: String::new(),
                sha256: String::new(),
            },
            name: "clip".into(),
            start_time: 0.,
            stop_time,
            sample_rate: 60.,
            r#loop: false,
            cycle_offset: 0.,
            tracks: vec![MotionTrack {
                node: 0,
                attribute: 1,
                path_hash: 0,
                custom_type: 0,
                curve_offset: 0,
                storage: "constant".into(),
                interpolation: "CONSTANT".into(),
                times: vec![0.],
                values: vec![vec![0., 0., z]],
                in_tangents: vec![vec![0.; 3]],
                out_tangents: vec![vec![0.; 3]],
                coefficients: Vec::new(),
            }],
            events: Vec::new(),
        }
    }
    /// Idle (0) goes to an attack (1) on a trigger. The attack returns at exit time 0.8.
    fn motion() -> MeleeMotion {
        MeleeMotion {
            controller: source(),
            handler: source(),
            animator: source(),
            target: NodeId(0),
            coordinate_space: "unity_parent_local".into(),
            clip_slots: vec![source(), source()],
            parameters: vec![
                MotionParameter {
                    id: SPEED,
                    name: "Speed".into(),
                    kind: "float".into(),
                    index: 0,
                    default_float: Some(0.),
                    default_bool: None,
                },
                MotionParameter {
                    id: ATTACK,
                    name: "Attack".into(),
                    kind: "trigger".into(),
                    index: 0,
                    default_float: None,
                    default_bool: Some(false),
                },
            ],
            default_state: 0,
            layer_binding: 0,
            layer_weight: 0.,
            layer_blending_mode: 0,
            layer_ik_pass: false,
            states: vec![
                state(0, 0, false, vec![transition(1, None, true)]),
                state(1, SPEED, true, Vec::new()),
            ],
            any_state_transitions: vec![transition(0, Some(0.8), false)],
            selectors: Vec::new(),
            clips: vec![clip(0., 1.), clip(1., 2.)],
        }
    }

    #[test]
    fn trigger_enters_the_attack_and_exit_time_returns_with_one_callback_each() {
        let m = motion();
        let mut c = MotionController::new(&m);
        let mut events = Vec::new();
        c.set_speed(1.);
        c.advance(&m, 0.05, |e| events.push(e));
        assert!(events.is_empty() && c.state() == 0);
        c.set_trigger(MotionController::parameter(&m, "Attack").unwrap());
        c.advance(&m, 0.05, |e| events.push(e));
        assert_eq!(events, [StateEvent::Enter(1)]);
        // Half way through the crossfade the pose is between the clips.
        c.advance(&m, 0.05, |e| events.push(e));
        assert!((c.pose(&m).position[2] - -1.5).abs() < 1e-5);
        c.advance(&m, 0.05, |e| events.push(e));
        assert_eq!(c.state(), 1);
        // Speed 0 holds the attack before its exit time.
        c.set_speed(0.);
        c.advance(&m, 2., |e| events.push(e));
        assert_eq!(events.len(), 1);
        c.set_speed(1.);
        for _ in 0..20 {
            c.advance(&m, 0.05, |e| events.push(e));
        }
        assert_eq!(events, [StateEvent::Enter(1), StateEvent::Exit(1)]);
        assert_eq!(c.state(), 0);
    }

    #[test]
    fn exit_time_crosses_once_per_loop() {
        assert!(crossed(0.7, 0.85, 0.8));
        assert!(!crossed(0.81, 0.9, 0.8));
        assert!(crossed(1.75, 1.82, 0.8));
        assert!(!crossed(0.5, 0.5, 0.8));
    }
}
