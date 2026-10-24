//! Source `ladderscript` contact countdown and `fpscontroller.LadderMove`.
use crate::simulation::{FrameInput, keys};
use rapier3d::math::Vector;
use rapier3d::prelude::ColliderHandle;

const CONTACT_STEPS: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LadderContact {
    pub collider: ColliderHandle,
    pub speed: f32,
    /// World space.
    pub up: Vector,
}

/// Source `ladderV`, `TLadder` and `releasedJump`.
#[derive(Clone, Copy, Debug)]
pub(super) struct LadderState {
    countdown: u8,
    ladder: Option<LadderContact>,
    active: bool,
    jump_released: bool,
    climbed: bool,
}
impl Default for LadderState {
    fn default() -> Self {
        Self {
            countdown: 0,
            ladder: None,
            active: false,
            jump_released: true,
            climbed: false,
        }
    }
}
impl LadderState {
    /// Source `OnTriggerStay`. Keeps the first ladder and refreshes its pose.
    pub(super) fn touch(&mut self, contact: LadderContact) {
        self.countdown = CONTACT_STEPS;
        if self.ladder.is_none_or(|l| l.collider == contact.collider) {
            self.ladder = Some(contact);
        }
    }
    /// One player step on foot.
    pub(super) fn tick(&mut self, input: FrameInput) {
        if input.released & keys::SPACE != 0 {
            self.jump_released = true;
        }
        self.active = self.countdown > 0;
        if self.active {
            self.countdown -= 1;
        } else {
            self.ladder = None;
        }
    }
    pub(super) fn ladder(&self) -> Option<LadderContact> {
        self.ladder.filter(|_| self.active)
    }
    /// The last `climb_velocity` call climbed.
    pub(super) fn climbed(&self) -> bool {
        self.climbed
    }
    /// Source `useGravity`.
    pub(super) fn gravity(&self) -> bool {
        self.ladder().is_none() || !self.jump_released
    }
    /// Source `LadderMove`. `None` walks normally. `speed` is `FladderSpeed`.
    pub(super) fn climb_velocity(
        &mut self,
        input: FrameInput,
        forward: Vector,
        right: Vector,
        grounded: bool,
        speed: f32,
        run_multiplier: Option<f32>,
    ) -> Option<Vector> {
        self.climbed = false;
        let ladder = self.ladder()?;
        let angle = forward.angle_between(ladder.up).to_degrees();
        if (angle > 70.0 && grounded) || !self.jump_released {
            return None;
        }
        let up = if angle < 120.0 { ladder.up } else { -ladder.up };
        let mut sum = Vector::ZERO;
        let mut vertical = Vector::ZERO;
        if input.held & keys::FORWARD != 0 {
            sum += forward;
            vertical = up;
        }
        if input.held & keys::BACK != 0 {
            vertical = -up;
        }
        if input.held & keys::LEFT != 0 {
            sum -= right;
        }
        if input.held & keys::RIGHT != 0 {
            sum += right;
        }
        sum += vertical.normalize_or_zero();
        if input.pressed & keys::SPACE != 0 {
            self.jump_released = false;
            return None;
        }
        self.climbed = true;
        Some(sum * speed * ladder.speed * run_multiplier.unwrap_or(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> LadderState {
        let mut state = LadderState::default();
        state.touch(LadderContact {
            collider: ColliderHandle::from_raw_parts(0, 0),
            speed: 2.0,
            up: Vector::Y,
        });
        state.tick(FrameInput::default());
        state
    }
    fn held(keys: u32) -> FrameInput {
        FrameInput {
            held: keys,
            ..FrameInput::default()
        }
    }
    fn climb(
        state: &mut LadderState,
        input: FrameInput,
        forward: Vector,
        grounded: bool,
    ) -> Option<Vector> {
        state.climb_velocity(input, forward, Vector::X, grounded, 3.0, None)
    }

    #[test]
    fn climb_velocity_follows_source_ladder_move() {
        let level = -Vector::Z;
        let mut state = state();
        // Forward adds the head forward and the ladder up, unnormalised.
        let up = climb(&mut state, held(keys::FORWARD), level, false).unwrap();
        assert!((up - (level + Vector::Y) * 6.0).length() < 1e-5);
        let down = climb(&mut state, held(keys::BACK), level, false).unwrap();
        assert!((down - -Vector::Y * 6.0).length() < 1e-5);
        let side = climb(&mut state, held(keys::LEFT), level, false).unwrap();
        assert!((side - -Vector::X * 6.0).length() < 1e-5);
        // Above 120 degrees the vertical keys swap.
        let low = (-Vector::Z - Vector::Y * 2.0).normalize();
        let swapped = climb(&mut state, held(keys::BACK), low, false).unwrap();
        assert!((swapped - Vector::Y * 6.0).length() < 1e-5);
        // Above 70 degrees on the ground walks normally.
        assert!(climb(&mut state, held(keys::FORWARD), level, true).is_none());
        let looking_up = (-Vector::Z + Vector::Y * 2.0).normalize();
        assert!(climb(&mut state, held(keys::FORWARD), looking_up, true).is_some());
        let run = state.climb_velocity(held(keys::BACK), level, Vector::X, false, 3.0, Some(1.5));
        assert!((run.unwrap() - -Vector::Y * 9.0).length() < 1e-5);
    }

    #[test]
    fn jump_leaves_ladder_until_release() {
        let mut state = state();
        assert!(!state.gravity());
        let jump = FrameInput {
            pressed: keys::SPACE,
            held: keys::SPACE,
            ..FrameInput::default()
        };
        assert!(climb(&mut state, jump, -Vector::Z, false).is_none());
        assert!(state.gravity());
        assert!(climb(&mut state, held(keys::FORWARD), -Vector::Z, false).is_none());
        state.tick(FrameInput {
            released: keys::SPACE,
            ..FrameInput::default()
        });
        assert!(!state.gravity());
        assert!(climb(&mut state, held(keys::FORWARD), -Vector::Z, false).is_some());
    }

    #[test]
    fn contact_expires_after_five_steps() {
        let mut state = state();
        for _ in 0..4 {
            state.tick(FrameInput::default());
            assert!(state.ladder().is_some());
        }
        state.tick(FrameInput::default());
        assert!(state.ladder().is_none());
        assert!(state.gravity());
    }
}
