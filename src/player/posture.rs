//! Source duck transitions and retained ordered clearance contacts.
use crate::{
    reference::PlayerPostureReference,
    simulation::{FrameInput, keys},
};

pub(super) struct PostureState {
    value: f32,
    last_ducked: bool,
    rising: bool,
    space_duck: bool,
    duck_jumped: bool,
    can_duck_jump: bool,
    contacts: [SensorContact; 10],
}

#[derive(Clone, Copy, Default)]
struct SensorContact {
    counter: u8,
    sensed: bool,
}
impl SensorContact {
    fn step(&mut self, contact: bool, hold: u8) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.sensed = false;
        }
        if contact {
            self.sensed = true;
            self.counter = hold;
        }
    }
}

impl PostureState {
    pub(super) fn new(value: f32) -> Self {
        Self {
            value,
            last_ducked: false,
            rising: false,
            space_duck: false,
            duck_jumped: false,
            can_duck_jump: false,
            contacts: [SensorContact::default(); 10],
        }
    }
    pub(super) fn value(&self) -> f32 {
        self.value
    }
    pub(super) fn restore(&mut self, value: f32) {
        *self = Self::new(value);
    }
    pub(super) fn contacts(&mut self, hits: [bool; 10], hold: u8) {
        for (contact, hit) in self.contacts.iter_mut().zip(hits) {
            contact.step(hit, hold);
        }
    }
    fn grow(&self, amount: f32, reference: &PlayerPostureReference) -> f32 {
        let mut value = 0.0;
        for (sensor, contact) in reference.clearance.iter().zip(self.contacts) {
            if contact.sensed {
                break;
            }
            value = amount.clamp(0.0, (sensor.posture - self.value).max(0.0));
        }
        value.clamp(0.0, 1.0)
    }
    /// Returns the source force multiplier for this fixed step.
    pub(super) fn step(
        &mut self,
        input: FrameInput,
        wheel_allowed: bool,
        grounded: bool,
        reference: &PlayerPostureReference,
        dt: f32,
        duck_force: f32,
        inverse_force: f32,
    ) -> f32 {
        if grounded {
            self.can_duck_jump = true;
            self.duck_jumped = false;
        }
        if input.released & keys::CROUCH != 0 && self.last_ducked {
            self.rising = true;
        }
        if input.pressed & keys::CROUCH != 0 {
            self.last_ducked = true;
        }
        let mut duck = input.held & keys::CROUCH != 0 || self.space_duck;
        if input.held & keys::CROUCH != 0 {
            self.rising = false;
        }
        let space_released = input.held & keys::SPACE == 0 && self.space_duck;
        if space_released {
            if self.last_ducked {
                self.rising = true;
            }
            self.space_duck = false;
        }
        let mut force = 0.0;
        let ctrl_jump = input.pressed & keys::CROUCH != 0
            && self.can_duck_jump
            && (!grounded || input.held & keys::SPACE != 0);
        if ctrl_jump && self.value >= 0.5 {
            force += duck_force;
            self.duck_jumped = true;
            self.can_duck_jump = false;
            self.last_ducked = true;
            duck = true;
        }
        if grounded {
            if input.pressed & keys::SPACE != 0 {
                force += 1.0;
                self.can_duck_jump = true;
                self.duck_jumped = false;
            }
        } else {
            if (input.released & keys::CROUCH != 0 || space_released) && self.duck_jumped {
                force -= inverse_force;
                self.duck_jumped = false;
            }
            if input.pressed & keys::SPACE != 0 && self.can_duck_jump {
                self.space_duck = true;
                if self.value >= 0.5 {
                    self.last_ducked = true;
                    duck = true;
                    self.duck_jumped = true;
                    self.can_duck_jump = false;
                    force += duck_force;
                }
            }
        }
        if duck && self.last_ducked {
            self.value = if self.value > reference.ctrl_value {
                reference.ctrl_value
            } else {
                self.value + self.grow(reference.ctrl_value - self.value, reference)
            };
        }
        if wheel_allowed && input.wheel.is_finite() && input.wheel != 0.0 {
            self.last_ducked = false;
            self.rising = false;
            let delta = input.wheel * reference.scroll_step;
            self.value += if delta < 0.0 {
                delta
            } else {
                self.grow(delta, reference)
            };
        }
        if self.rising && !self.space_duck {
            self.value += self.grow(dt * reference.rise_rate, reference);
        }
        self.value = self.value.clamp(0.0, 1.0);
        force
    }
}

#[derive(Default)]
pub(super) struct MovementSpeed {
    limit: f32,
    takeoff: f32,
}
impl MovementSpeed {
    pub(super) fn limit(&mut self, grounded: bool, requested: f32) -> f32 {
        if grounded {
            self.limit = requested;
        } else {
            self.limit = self.limit.clamp(-self.takeoff, self.takeoff);
        }
        self.limit
    }
    pub(super) fn grounded_velocity(&mut self, magnitude: f32) {
        self.takeoff = magnitude;
    }
}

pub(super) fn speed_multiplier(value: f32, ctrl: f32, low: f32, duck: f32) -> f32 {
    if value >= ctrl {
        duck + (1.0 - duck) * ((value - ctrl) / (1.0 - ctrl))
    } else {
        low + (duck - low) * (value / ctrl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sensor_keeps_three_steps_then_clears_on_fourth() {
        let mut sensor = SensorContact::default();
        sensor.step(true, 3);
        for counter in [2, 1, 0] {
            sensor.step(false, 3);
            assert!(sensor.sensed);
            assert_eq!(sensor.counter, counter);
        }
        sensor.step(false, 3);
        assert!(!sensor.sensed);
        sensor.step(true, 3);
        sensor.step(false, 3);
        sensor.step(true, 3);
        assert_eq!(sensor.counter, 3);
    }
    #[test]
    fn source_speed_endpoints() {
        for (posture, speed) in [(0., 0.525), (0.3, 1.05), (1., 1.75)] {
            assert!((1.75 * speed_multiplier(posture, 0.3, 0.3, 0.6) - speed).abs() < 1e-6);
        }
        assert_eq!(1.75 * 2.5, 4.375);
    }
}

#[cfg(test)]
pub(super) fn test_reference() -> PlayerPostureReference {
    use crate::reference::{PlayerClearanceReference, PlayerSphereReference, SourceId, Transform};
    let transform = |y| Transform {
        translation: [0., y, 0.],
        rotation: [0., 0., 0., 1.],
        scale: [1.; 3],
    };
    PlayerPostureReference {
        capsule_membership: 1 << 12,
        capsule_filter: 4_172_190_207,
        standing_head: transform(0.65),
        prone_head: transform(-0.685),
        minimum_radius: 0.1,
        maximum_radius: 0.3,
        ctrl_value: 0.3,
        scroll_step: 0.05,
        rise_rate: 5.0,
        sensor_hold_steps: 3,
        feet: PlayerSphereReference {
            source: SourceId {
                file: "level1".into(),
                path_id: 57258,
            },
            center: [0., -0.573, 0.],
            radius: 0.3,
            membership: 1 << 12,
            filter: 4_172_190_207,
        },
        clearance: std::array::from_fn(|i| {
            let posture = (i + 1) as f32 / 10.;
            PlayerClearanceReference {
                posture,
                sphere: PlayerSphereReference {
                    source: SourceId {
                        file: "sharedassets1.assets".into(),
                        path_id: 124758,
                    },
                    center: [0., -0.685 + 1.335 * posture, 0.],
                    radius: 0.15,
                    membership: 1 << 14,
                    filter: 4_037_970_431,
                },
            }
        }),
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;
    fn step(state: &mut PostureState, input: FrameInput, grounded: bool) -> f32 {
        state.step(input, true, grounded, &test_reference(), 0.02, 1.25, 0.5)
    }
    #[test]
    fn ctrl_rises_from_point_two_and_release_stands_at_source_rate() {
        let mut state = PostureState::new(0.2);
        step(
            &mut state,
            FrameInput {
                pressed: keys::CROUCH,
                held: keys::CROUCH,
                ..Default::default()
            },
            true,
        );
        assert_eq!(state.value(), 0.3);
        step(
            &mut state,
            FrameInput {
                released: keys::CROUCH,
                ..Default::default()
            },
            true,
        );
        assert!((state.value() - 0.4).abs() < 1e-6);
        for _ in 0..10 {
            step(&mut state, FrameInput::default(), true);
        }
        assert_eq!(state.value(), 1.);
    }
    #[test]
    fn wheel_posture_persists_and_endpoints_clamp() {
        let mut state = PostureState::new(1.);
        step(
            &mut state,
            FrameInput {
                wheel: -16.6,
                ..Default::default()
            },
            true,
        );
        assert!((state.value() - 0.17).abs() < 1e-6);
        for _ in 0..20 {
            step(&mut state, FrameInput::default(), true);
        }
        assert!((state.value() - 0.17).abs() < 1e-6);
        step(
            &mut state,
            FrameInput {
                wheel: -100.,
                ..Default::default()
            },
            true,
        );
        assert_eq!(state.value(), 0.);
        step(
            &mut state,
            FrameInput {
                wheel: 100.,
                ..Default::default()
            },
            true,
        );
        assert_eq!(state.value(), 1.);
        state.restore(0.17);
        step(&mut state, FrameInput::default(), true);
        assert_eq!(state.value(), 0.17);
    }
    #[test]
    fn ordered_sensors_stop_growth_before_first_retained_contact() {
        let mut state = PostureState::new(0.2);
        let mut hits = [false; 10];
        hits[3] = true;
        state.contacts(hits, 3);
        step(
            &mut state,
            FrameInput {
                wheel: 100.,
                ..Default::default()
            },
            true,
        );
        assert_eq!(state.value(), 0.3);
        for _ in 0..3 {
            state.contacts([false; 10], 3);
            step(
                &mut state,
                FrameInput {
                    wheel: 100.,
                    ..Default::default()
                },
                true,
            );
            assert_eq!(state.value(), 0.3);
        }
        state.contacts([false; 10], 3);
        step(
            &mut state,
            FrameInput {
                wheel: 100.,
                ..Default::default()
            },
            true,
        );
        assert_eq!(state.value(), 1.);
    }
    #[test]
    fn duck_jump_force_release_and_landing_reset_follow_source() {
        let mut state = PostureState::new(1.);
        let jump = FrameInput {
            pressed: keys::SPACE,
            held: keys::SPACE,
            ..Default::default()
        };
        assert_eq!(step(&mut state, jump, true), 1.);
        assert_eq!(
            step(
                &mut state,
                FrameInput {
                    released: keys::SPACE,
                    ..Default::default()
                },
                false
            ),
            0.
        );
        assert_eq!(step(&mut state, jump, false), 1.25);
        assert_eq!(state.value(), 0.3);
        step(
            &mut state,
            FrameInput {
                held: keys::SPACE,
                wheel: 100.,
                ..Default::default()
            },
            false,
        );
        assert_eq!(state.value(), 1.);
        assert_eq!(step(&mut state, jump, false), 0.);
        assert_eq!(
            step(
                &mut state,
                FrameInput {
                    released: keys::SPACE,
                    ..Default::default()
                },
                false
            ),
            -0.5
        );
        assert_eq!(step(&mut state, FrameInput::default(), false), 0.);
        for _ in 0..10 {
            step(&mut state, FrameInput::default(), true);
        }
        assert_eq!(step(&mut state, jump, true), 1.);
        assert_eq!(step(&mut state, jump, false), 1.25);
    }
}

#[cfg(test)]
mod speed_tests {
    use super::*;
    #[test]
    fn air_posture_and_run_keep_the_ground_cap_and_source_takeoff_clamp() {
        let mut speed = MovementSpeed::default();
        assert_eq!(speed.limit(true, 1.75), 1.75);
        speed.grounded_velocity(1.5);
        assert_eq!(speed.limit(false, 4.375), 1.5);
        assert_eq!(speed.limit(false, 0.525), 1.5);
        assert_eq!(speed.limit(true, 1.05), 1.05);
        speed.grounded_velocity(0.8);
        assert_eq!(speed.limit(false, 4.375), 0.8);
    }
}
