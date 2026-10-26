//! Source `bikepedalscript.Update()`: pedal gas, pedal rpm and wheel torque.

use crate::reference::PedalsReference;
use crate::simulation::dials::{inverse_lerp, lerp};

/// Source `mainscript.minWheelMotorTorque`.
const MIN_WHEEL_MOTOR_TORQUE: f32 = 0.;

/// Source `multiGas` is multiplayer only. It is 0, so `gas < multiGas` never applies.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PedalState {
    pub gas: f32,
    pub debounce: f32,
    pub rpm: f32,
    pub output: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PedalStep {
    /// Motor torque for each driven wheel. `None` while disabled: carscript owns the torque.
    pub torque: Option<f32>,
    /// Pedal x rotation in degrees this step, for all `Axis` entries. Inverse pedals turn back.
    pub angle: f32,
}

/// `wheel_rpms`: Unity `WheelCollider.rpm` of each `WHDriven` wheel.
/// `mounted`: driven slots with a part (source `DrivenWheelsNum2`).
#[allow(clippy::too_many_arguments)]
pub fn step(
    reference: &PedalsReference,
    pedaling_nm: f32,
    pedaling_rpm: f32,
    state: &mut PedalState,
    seated: bool,
    throttle: f32,
    wheel_rpms: &[f32],
    mounted: usize,
    disabled: bool,
    dt: f32,
) -> PedalStep {
    let driven = mounted.max(1) as f32;
    let ratio = 1. / reference.ratio;
    let wrpm = wheel_rpms.iter().sum::<f32>() / driven * ratio;
    state.output = 0.;
    if state.debounce > 0. {
        state.debounce -= dt;
    } else {
        state.gas = 0.;
    }
    if !disabled && reference.player_use && seated && throttle > 0. {
        state.gas = reference.gas_tap * throttle;
        state.debounce = reference.input_debounce;
    }
    state.rpm = lerp(0., pedaling_rpm, state.gas);
    if state.rpm > wrpm {
        state.rpm = wrpm;
        state.output = pedaling_nm * state.gas * ratio / driven;
    }
    let t = inverse_lerp(reference.input_debounce, 0., state.debounce);
    if state.debounce > 0. {
        state.debounce -= dt;
        state.rpm = lerp(state.rpm, 0., t);
    } else {
        state.rpm = 0.;
    }
    // Source `UpdPedals`.
    if state.rpm < 0. {
        state.rpm = 0.;
    }
    PedalStep {
        torque: (!disabled).then(|| MIN_WHEEL_MOTOR_TORQUE.max(state.output)),
        angle: state.rpm * reference.speed_modifier * dt * reference.axes.len() as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::SourceId;

    const DT: f32 = 0.02;
    const NM: f32 = 40.;
    const RPM: f32 = 60.;

    fn reference() -> PedalsReference {
        PedalsReference {
            source: SourceId {
                file: String::new(),
                path_id: 0,
            },
            driven_wheels: vec![0],
            driven_slots: vec![0],
            seat: 0,
            pedals: vec![],
            pedals_inverse: vec![],
            feet_holders: vec![],
            axes: vec![0],
            axis_index: vec![],
            ratio: 2.,
            speed_modifier: 1.,
            input_debounce: 0.5,
            gas_tap: 1.,
            player_use: true,
        }
    }

    fn pedal(state: &mut PedalState, throttle: f32, rpms: &[f32], mounted: usize) -> PedalStep {
        step(
            &reference(),
            NM,
            RPM,
            state,
            true,
            throttle,
            rpms,
            mounted,
            false,
            DT,
        )
    }

    #[test]
    fn debounce_holds_gas_then_drops_it() {
        let mut state = PedalState::default();
        pedal(&mut state, 1., &[1000.], 1);
        assert_eq!(state.gas, 1.);
        for _ in 0..10 {
            pedal(&mut state, 0., &[1000.], 1);
        }
        assert_eq!(state.gas, 1.);
        for _ in 0..10 {
            pedal(&mut state, 0., &[1000.], 1);
        }
        assert_eq!(state.gas, 0.);
        assert_eq!(state.rpm, 0.);
    }

    #[test]
    fn rpm_caps_at_wheel_rpm_and_only_capped_rpm_gives_torque() {
        let mut state = PedalState::default();
        let free = pedal(&mut state, 1., &[1000.], 1);
        assert_eq!(free.torque, Some(0.));
        assert_eq!(state.rpm, RPM);

        let mut state = PedalState::default();
        let capped = pedal(&mut state, 1., &[20.], 1);
        // Wheel 20 rpm / ratio 2. A fresh tap has full debounce, so lerp t is 0.
        assert_eq!(state.rpm, 10.);
        assert_eq!(capped.torque, Some(NM / 2.));
        assert!((capped.angle - state.rpm * DT).abs() < 1e-6);
    }

    #[test]
    fn torque_splits_by_mounted_wheels() {
        let mut state = PedalState::default();
        assert_eq!(pedal(&mut state, 1., &[0., 0.], 2).torque, Some(NM / 4.));
        let mut state = PedalState::default();
        assert_eq!(pedal(&mut state, 1., &[0., 0.], 0).torque, Some(NM / 2.));
    }

    #[test]
    fn disabled_gives_no_torque() {
        let mut state = PedalState::default();
        let out = step(
            &reference(),
            NM,
            RPM,
            &mut state,
            true,
            1.,
            &[0.],
            1,
            true,
            DT,
        );
        assert_eq!(out.torque, None);
        assert_eq!(state.gas, 0.);
    }
}
