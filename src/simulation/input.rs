use crate::reference::{MouseDriveReference, SeatId};
use crate::simulation::{FIXED_DT, keys};
use serde::Deserialize;

/// Source `inputscript.DriveStuff` mouse axis scale.
const MOUSE_SCALE: f32 = 0.0125;
/// Source extra scale on `Mouse X`.
const MOUSE_STEER_SCALE: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrivingControls {
    /// Positive steers left.
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub handbrake: f32,
}

/// Player driving settings from the browser (source `settingsscript.S`).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivingSettings {
    /// Source `FMouseDriveSensX`, `FMouseDriveSensY`.
    pub mouse: [f32; 2],
}

impl DrivingSettings {
    pub fn defaults(reference: &MouseDriveReference) -> Self {
        Self {
            mouse: [
                reference.sensitivity_x.default,
                reference.sensitivity_y.default,
            ],
        }
    }

    pub fn valid(&self, reference: &MouseDriveReference) -> bool {
        reference.sensitivity_x.contains(self.mouse[0])
            && reference.sensitivity_y.contains(self.mouse[1])
    }
}

/// Source `inputscript.DriveStuff` keyboard and mouse controls at the fixed gameplay step.
pub struct DrivingInput {
    keyboard: DrivingControls,
    steering_increment: f32,
    pedal_increment: f32,
    seat: Option<SeatId>,
    /// Source `mouseDriveToggle`. It stays across seats and pauses.
    mouse_toggle: bool,
    /// Source `currentMouseSteer` (positive steers right) and `currentMouseGas`.
    mouse: [f32; 2],
    pub settings: DrivingSettings,
}

impl DrivingInput {
    pub fn new(steering_rate: f32, pedal_rate: f32, settings: DrivingSettings) -> Self {
        Self {
            keyboard: DrivingControls::default(),
            steering_increment: steering_rate * FIXED_DT,
            pedal_increment: pedal_rate * FIXED_DT,
            seat: None,
            mouse_toggle: false,
            mouse: [0.; 2],
            settings,
        }
    }

    pub fn clear(&mut self) {
        self.keyboard = DrivingControls::default();
        self.mouse = [0.; 2];
        self.seat = None;
    }

    pub fn toggle_mouse(&mut self) {
        self.mouse_toggle = !self.mouse_toggle;
    }

    /// Source `mouseDrive`. Mouse look reads zero while it is on.
    pub fn mouse_owned(&self) -> bool {
        self.mouse_toggle && self.seat.is_some()
    }

    /// `mouse` is the source `Mouse X`, `Mouse Y` sample. `steer_multiplier` is the seated car's
    /// `keySteerSensMultiplier`; zero means none.
    pub fn step(
        &mut self,
        seat: Option<SeatId>,
        held: u32,
        mouse: [f32; 2],
        steer_multiplier: f32,
    ) -> DrivingControls {
        if self.seat != seat {
            self.clear();
            self.seat = seat;
        }
        if seat.is_none() {
            return self.keyboard;
        }
        let steering_increment = if steer_multiplier != 0. {
            self.steering_increment * steer_multiplier
        } else {
            self.steering_increment
        };
        let steer =
            ((held & keys::LEFT != 0) as u8 as f32) - ((held & keys::RIGHT != 0) as u8 as f32);
        self.keyboard.steer +=
            (steer - self.keyboard.steer).clamp(-steering_increment, steering_increment);
        for (value, bit) in [
            (&mut self.keyboard.throttle, keys::FORWARD),
            (&mut self.keyboard.brake, keys::BACK),
            (&mut self.keyboard.clutch, keys::CLUTCH),
        ] {
            let target = (held & bit != 0) as u8 as f32;
            *value += (target - *value).clamp(-self.pedal_increment, self.pedal_increment);
        }
        if !self.mouse_toggle {
            self.mouse = [0.; 2];
            return self.keyboard;
        }
        let sensitivity = self.settings.mouse;
        if mouse.iter().all(|v| v.is_finite()) {
            self.mouse[0] = (self.mouse[0]
                + mouse[0] * MOUSE_SCALE * MOUSE_STEER_SCALE * sensitivity[0])
                .clamp(-1., 1.);
            self.mouse[1] =
                (self.mouse[1] + mouse[1] * MOUSE_SCALE * sensitivity[1]).clamp(-1., 1.);
        }
        DrivingControls {
            steer: (self.keyboard.steer - self.mouse[0]).clamp(-1., 1.),
            throttle: (self.keyboard.throttle + self.mouse[1].clamp(0., 1.)).clamp(0., 1.),
            brake: (self.keyboard.brake + (-self.mouse[1]).clamp(0., 1.)).clamp(0., 1.),
            ..self.keyboard
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEAT: Option<SeatId> = Some(SeatId(1));

    fn input() -> DrivingInput {
        DrivingInput::new(2., 5., DrivingSettings { mouse: [1., 1.] })
    }

    #[test]
    fn mouse_drives_after_toggle() {
        let mut driving = input();
        assert_eq!(
            driving.step(SEAT, 0, [0., 40.], 0.),
            DrivingControls::default()
        );
        driving.toggle_mouse();
        let up = driving.step(SEAT, 0, [0., 40.], 0.);
        assert!((up.throttle - 0.5).abs() < 1e-6 && up.brake == 0.);
        let down = driving.step(SEAT, 0, [0., -120.], 0.);
        assert!((down.brake - 1.).abs() < 1e-6 && down.throttle == 0.);
        // Mouse X has 0.25 of the Y scale. Right is negative steer.
        let right = driving.step(SEAT, 0, [40., 0.], 0.);
        assert!((right.steer + 0.125).abs() < 1e-6);
        assert!(driving.mouse_owned());
    }

    #[test]
    fn toggle_off_resets_and_seat_gates_mouse() {
        let mut driving = input();
        driving.toggle_mouse();
        assert_eq!(
            driving.step(None, 0, [40., 40.], 0.),
            DrivingControls::default()
        );
        assert!(!driving.mouse_owned());
        driving.step(SEAT, 0, [40., 40.], 0.);
        driving.toggle_mouse();
        assert_eq!(
            driving.step(SEAT, 0, [0., 0.], 0.),
            DrivingControls::default()
        );
        assert!(!driving.mouse_owned());
        driving.toggle_mouse();
        assert_eq!(
            driving.step(SEAT, 0, [0., 0.], 0.),
            DrivingControls::default()
        );
    }

    #[test]
    fn keyboard_steer_uses_car_multiplier() {
        let mut driving = input();
        let base = driving.step(SEAT, keys::LEFT, [0.; 2], 0.).steer;
        assert!((base - 2. * FIXED_DT).abs() < 1e-6);
        driving.clear();
        let scaled = driving.step(SEAT, keys::LEFT, [0.; 2], 3.).steer;
        assert!((scaled - 6. * FIXED_DT).abs() < 1e-6);
    }
}
