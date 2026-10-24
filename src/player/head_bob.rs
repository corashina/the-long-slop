//! Source `fpscontroller` walk head bob and `playeranimationscript.CarHeadBob`.
use crate::reference::{PlayerPoseReference, PlayerReference};
use rapier3d::math::{Rotation, Vector};

/// The offset holds its last value when the player stops.
#[derive(Clone, Copy, Debug)]
pub(super) struct WalkBob {
    speed: f32,
    speed_rb: f32,
    max: f32,
    threshold: f32,
    state: f32,
    pub(super) offset: f32,
    wdown: bool,
    /// True on the step a source footstep fires.
    pub(super) beat: bool,
}
impl WalkBob {
    pub(super) fn new(player: &PlayerReference) -> Self {
        Self {
            speed: player.walk_bob_speed,
            speed_rb: player.walk_bob_speed_rb,
            max: player.max_walk_bob,
            threshold: player.walk_bob_sound_threshold,
            state: 0.,
            offset: 0.,
            wdown: false,
            beat: false,
        }
    }
    /// `body_speed` is the body speed minus the support body speed.
    pub(super) fn step(&mut self, active: bool, body_speed: f32, dt: f32) {
        self.beat = false;
        if !active {
            return;
        }
        self.state += dt * self.speed + body_speed * self.speed_rb * dt;
        self.offset = self.state.sin() * self.max;
        if self.offset <= -self.threshold {
            self.beat = !self.wdown;
            self.wdown = true;
        } else {
            self.wdown = false;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CarHeadBob {
    back: f32,
    back_lerp: f32,
    forward_lerp: f32,
    max: [f32; 3],
    car_v: Vector,
    head_v: Vector,
}
impl CarHeadBob {
    pub(super) fn new(pose: &PlayerPoseReference) -> Self {
        let scalar = |name: &str| pose.animation_scalars[name];
        Self {
            back: scalar("headBackM"),
            back_lerp: scalar("headBackLerp"),
            forward_lerp: scalar("headForwardLerp"),
            max: [
                scalar("maxHeadBobX"),
                scalar("maxHeadBobY"),
                scalar("maxHeadBobZ"),
            ],
            car_v: Vector::ZERO,
            head_v: Vector::ZERO,
        }
    }
    pub(super) fn reset(&mut self) {
        self.car_v = Vector::ZERO;
        self.head_v = Vector::ZERO;
    }
    /// Returns a world offset. `head` gives the right, up, and forward clamp axes.
    pub(super) fn step(
        &mut self,
        dt: f32,
        parent_velocity: Vector,
        dead: bool,
        head: Rotation,
    ) -> Vector {
        if !dead {
            self.car_v = self.car_v.lerp(
                -parent_velocity * self.back,
                (self.back_lerp * dt).clamp(0., 1.),
            );
            self.head_v = self
                .head_v
                .lerp(-self.car_v, (self.forward_lerp * dt).clamp(0., 1.));
        }
        let bob = self.car_v + self.head_v;
        let mut offset = Vector::ZERO;
        for (axis, max) in [Vector::X, Vector::Y, Vector::Z].into_iter().zip(self.max) {
            let axis = head * axis;
            offset += axis * bob.dot(axis).clamp(-max, max);
        }
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bob() -> CarHeadBob {
        CarHeadBob {
            back: 0.1,
            back_lerp: 10.,
            forward_lerp: 4.,
            max: [0.2, 0.1, 0.3],
            car_v: Vector::ZERO,
            head_v: Vector::ZERO,
        }
    }

    #[test]
    fn walk_bob_phase_amplitude_and_one_beat_per_cycle() {
        let mut bob = WalkBob {
            speed: 10.,
            speed_rb: 2.,
            max: 0.05,
            threshold: 0.04,
            state: 0.,
            offset: 0.,
            wdown: false,
            beat: false,
        };
        // state = 0.01 * 10 + 3 * 2 * 0.01 = 0.16.
        bob.step(true, 3., 0.01);
        assert!((bob.state - 0.16).abs() < 1e-6);
        assert!((bob.offset - 0.16f32.sin() * 0.05).abs() < 1e-7);
        bob.step(false, 3., 0.01);
        assert!((bob.state - 0.16).abs() < 1e-6);
        let mut beats = 0;
        // 0.16 rad per step, so 1000 steps cover 25.46 cycles past the start.
        for _ in 0..1000 {
            bob.step(true, 3., 0.01);
            beats += bob.beat as u32;
        }
        assert_eq!(beats, 25);
    }

    #[test]
    fn car_bob_lags_clamps_and_freezes_when_dead() {
        let mut bob = bob();
        let v = Vector::new(1., 0., 0.);
        // car_v = -0.1 * 0.1 = -0.01; head_v = 0.01 * 0.04 = 0.0004.
        let offset = bob.step(0.01, v, false, Rotation::IDENTITY);
        assert!((offset.x - (-0.01 + 0.0004)).abs() < 1e-6);
        for _ in 0..1000 {
            bob.step(0.01, v, false, Rotation::IDENTITY);
        }
        assert!((bob.car_v - v * -0.1).length() < 1e-5);
        let offset = bob.step(
            0.01,
            Vector::new(100., 100., 100.),
            false,
            Rotation::IDENTITY,
        );
        assert_eq!(offset, Vector::new(-0.2, -0.1, -0.3));
        let frozen = bob.step(0.01, Vector::ZERO, true, Rotation::IDENTITY);
        assert_eq!(frozen, offset);
        // A turned head clamps along its own axes.
        let turned = Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let offset = bob.step(0.01, Vector::ZERO, true, turned);
        assert!((offset - Vector::new(-0.3, -0.1, -0.2)).length() < 1e-5);
    }
}
