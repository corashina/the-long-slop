use super::{
    NodeId, ReferenceError, ReferenceManifest, SourceId, Transform, finite, require, transform,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrivingAction {
    Clutch,
    ShiftUp,
    ShiftDown,
    Reverse,
    Neutral,
    Gear1,
    Gear2,
    Gear3,
    Gear4,
    Gear5,
    Gear6,
    /// Source `IN.togglemousedrive`.
    MouseDrive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivingAssists {
    pub automatic_clutch: bool,
    pub automatic_gears: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivingBinding {
    pub action: DrivingAction,
    pub codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivingReference {
    pub source: SourceId,
    pub pedal_rate: f32,
    pub force_manual: bool,
    pub min_motor_torque_nm: f32,
    pub default_assists: DrivingAssists,
    pub bindings: Vec<DrivingBinding>,
    pub mouse_drive: MouseDriveReference,
}

/// Source `slidersettingscript`: `GetValue` range and default, multiplier applied.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderRange {
    pub default: f32,
    pub minimum: f32,
    pub maximum: f32,
}

impl SliderRange {
    pub fn contains(&self, value: f32) -> bool {
        (self.minimum..=self.maximum).contains(&value)
    }
}

/// Source `tabControls.sliders[8]`, `[9]` (`FMouseDriveSensX`, `FMouseDriveSensY`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MouseDriveReference {
    pub sensitivity_x: SliderRange,
    pub sensitivity_y: SliderRange,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivingVehicleReference {
    pub initial_gear: usize,
    pub force_automatic: bool,
    pub play_shift_sounds_in_automatic: bool,
}

/// Source `carscript.gears[i]` lever poses, in the lever parent space.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeverGear {
    pub path: Transform,
    pub pos: Transform,
}

/// Source `carscript.Transmission`. `gears` follow `VehicleReference.gear_ratios`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeverReference {
    pub node: NodeId,
    pub gears: Vec<LeverGear>,
    pub seconds: f32,
    pub stages: [f32; 2],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelFrictionCurve {
    pub extremum_slip: f32,
    pub extremum_value: f32,
    pub asymptote_slip: f32,
    pub asymptote_value: f32,
    pub stiffness: f32,
}

impl WheelFrictionCurve {
    pub(super) fn valid(&self) -> bool {
        finite(&[
            self.extremum_slip,
            self.extremum_value,
            self.asymptote_slip,
            self.asymptote_value,
            self.stiffness,
        ]) && self.extremum_slip > 0.0
            && self.asymptote_slip > self.extremum_slip
            && self.extremum_value > 0.0
            && self.asymptote_value >= 0.0
            && self.asymptote_value <= self.extremum_value
            && self.stiffness >= 0.0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelHubReference {
    pub radius_m: f32,
    pub mass_kg: f32,
    pub forward_stiffness: f32,
    pub sideways_stiffness: f32,
}

impl DrivingReference {
    pub(super) fn validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.source.file == "level0" && self.source.path_id == 16865,
            "driving settings source",
        )?;
        require(
            finite(&[self.pedal_rate, self.min_motor_torque_nm])
                && self.pedal_rate > 0.0
                && self.min_motor_torque_nm >= 0.0,
            "driving parameters",
        )?;
        require(
            !self.default_assists.automatic_gears || self.default_assists.automatic_clutch,
            "driving assist dependency",
        )?;
        let allowed = [
            "ControlLeft",
            "ControlRight",
            "Numpad0",
            "ArrowUp",
            "ArrowDown",
            "KeyR",
            "KeyN",
            "NumpadAdd",
            "NumpadSubtract",
            "Digit0",
            "Digit1",
            "Digit2",
            "Digit3",
            "Digit4",
            "Digit5",
            "Digit6",
            "Numpad1",
            "Numpad2",
            "Numpad3",
            "Numpad4",
            "Numpad5",
            "Numpad6",
            "KeyU",
        ];
        let mut actions = HashSet::new();
        require(
            self.bindings.len() == 12
                && self.bindings.iter().all(|binding| {
                    actions.insert(binding.action)
                        && !binding.codes.is_empty()
                        && binding
                            .codes
                            .iter()
                            .all(|code| allowed.contains(&code.as_str()))
                        && binding.codes.iter().collect::<HashSet<_>>().len() == binding.codes.len()
                }),
            "driving bindings",
        )?;
        require(
            [self.mouse_drive.sensitivity_x, self.mouse_drive.sensitivity_y]
                .iter()
                .all(|range| {
                    finite(&[range.default, range.minimum, range.maximum])
                        && range.contains(range.default)
                }),
            "mouse driving sensitivity",
        )?;
        for vehicle in &manifest.vehicles {
            require(
                vehicle.driving.initial_gear < vehicle.gear_ratios.len(),
                "driving startup gear",
            )?;
            require(
                vehicle.key_steer_multiplier.is_finite() && vehicle.key_steer_multiplier >= 0.0,
                "key steer multiplier",
            )?;
            if let Some(lever) = &vehicle.lever {
                require(
                    manifest.nodes.iter().any(|node| node.id == lever.node)
                        && lever.gears.len() == vehicle.gear_ratios.len()
                        && lever
                            .gears
                            .iter()
                            .all(|gear| transform(&gear.path) && transform(&gear.pos))
                        && finite(&[lever.seconds, lever.stages[0], lever.stages[1]])
                        && lever.seconds > 0.0
                        && 0.0 < lever.stages[0]
                        && lever.stages[0] < lever.stages[1]
                        && lever.stages[1] < 1.0,
                    "driving lever",
                )?;
            }
        }
        Ok(())
    }
}
