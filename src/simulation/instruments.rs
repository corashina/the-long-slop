use super::dials::{digit_angle, lerp, lerp_angle};
use crate::reference::{InstrumentReference, NeedleReference};
use rapier3d::math::Rotation;
#[derive(Clone, Copy, Default)]
pub struct InstrumentSample {
    pub speed_kmh: f32,
    pub fuel_liters: f32,
    pub oil_liters: f32,
    pub temperature: f32,
    pub rpm: f32,
    pub local_ignition: bool,
    pub headlights: u8,
    pub distance_m: f64,
}
/// Gauge state of one car, for any needle, lamp and digit count. Lamp bits hold up to 32 lamps.
pub struct InstrumentState {
    signals: Vec<f32>,
    values: Vec<f32>,
    digit_values: Vec<f32>,
    pub needle_angles: Vec<f32>,
    pub digit_angles: Vec<f32>,
    pub lamp_bits: u32,
    pub digits_lit: bool,
    pub distance_km: f32,
}
pub fn needle_angle(r: &NeedleReference, value: f32) -> f32 {
    let mut t = ((value + r.offset - r.min_value) / (r.max_value - r.min_value)).clamp(0., 1.);
    if r.inv {
        t = 1. - t;
    }
    lerp(r.min_angle, r.max_angle, t)
}
pub fn source_rotation(e: [f32; 3]) -> [f32; 4] {
    let q = (Rotation::from_rotation_y(e[1].to_radians())
        * Rotation::from_rotation_x(e[0].to_radians())
        * Rotation::from_rotation_z(e[2].to_radians()))
    .to_array();
    [-q[0], -q[1], q[2], q[3]]
}
impl InstrumentState {
    pub fn new(r: &InstrumentReference) -> Self {
        let mut state = Self {
            signals: vec![0.; r.needles.len()],
            values: vec![0.; r.needles.len()],
            digit_values: vec![0.; r.odometer.digits.len()],
            needle_angles: vec![0.; r.needles.len()],
            digit_angles: vec![0.; r.odometer.digits.len()],
            lamp_bits: 0,
            digits_lit: false,
            distance_km: 0.,
        };
        state.reset(r);
        state
    }
    /// Startup values. Keeps the buffers.
    pub fn reset(&mut self, r: &InstrumentReference) {
        self.signals.fill(0.);
        self.values.fill(0.);
        for (i, d) in r.odometer.digits.iter().enumerate() {
            self.digit_values[i] = d.initial_value;
            self.digit_angles[i] = d.initial_angle;
        }
        for (i, n) in r.needles.iter().enumerate() {
            self.needle_angles[i] = needle_angle(n, 0.);
        }
        self.lamp_bits = 0;
        self.digits_lit = false;
        self.distance_km = r.odometer.initial_value;
    }
    pub fn initialize_from_sample(&mut self, r: &InstrumentReference, s: InstrumentSample) {
        self.reset(r);
        self.evaluate(r, s, 0., true);
    }
    pub fn update(&mut self, r: &InstrumentReference, s: InstrumentSample, dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        self.evaluate(r, s, dt, false);
    }
    fn evaluate(
        &mut self,
        r: &InstrumentReference,
        s: InstrumentSample,
        dt: f32,
        initialize: bool,
    ) {
        for (i, n) in r.needles.iter().enumerate() {
            let target = match n.signal.as_str() {
                "speed" => s.speed_kmh.abs(),
                "fuel" => {
                    if s.local_ignition {
                        s.fuel_liters
                    } else {
                        0.
                    }
                }
                "temp" => {
                    if s.local_ignition {
                        s.temperature
                    } else {
                        0.
                    }
                }
                _ => 0.,
            };
            self.signals[i] = if !initialize && n.signal_smoothing > 0. {
                lerp(self.signals[i], target, dt * n.signal_smoothing)
            } else {
                target
            };
            self.values[i] = if n.no_value_lerp {
                self.signals[i] + n.offset
            } else if initialize {
                self.signals[i]
            } else if n.lerp_angle {
                lerp_angle(self.values[i], self.signals[i], dt * n.value_smoothing)
            } else {
                lerp(self.values[i], self.signals[i], dt * n.value_smoothing)
            };
            self.needle_angles[i] = needle_angle(n, self.values[i]);
        }
        self.lamp_bits = 0;
        for (i, l) in r.lamps.iter().enumerate() {
            let on = match l.signal.as_str() {
                "fuel" => s.local_ignition && s.fuel_liters < r.low_fuel,
                "oil" => s.local_ignition && (s.oil_liters < r.low_oil || s.rpm < r.oil_test_rpm),
                "battery" => s.local_ignition && s.rpm < r.battery_test_rpm,
                "headlight2" => {
                    s.headlights == 2 && r.electronics_output > r.electronics_off_threshold
                }
                _ => false,
            };
            let v = if on { 1. } else { 0. };
            if v >= l.min_value && (v <= l.max_value || l.stay_on) {
                self.lamp_bits |= 1 << i;
            }
        }
        self.digits_lit = s.local_ignition && matches!(s.headlights, 1 | 2);
        self.distance_km = r.odometer.initial_value + (s.distance_m / 1000.) as f32;
        let o = &r.odometer;
        let value = if o.use_multiplier {
            self.distance_km * o.multiplier
        } else {
            self.distance_km
        };
        let base = o.base as f32;
        for (i, d) in o.digits.iter().enumerate() {
            if !d.direct {
                let place = base.powf(d.place as f32 - o.decimals + 1.);
                self.digit_values[i] = if place == 0. {
                    0.
                } else {
                    value % place / place * base
                };
            }
        }
        for (i, d) in o.digits.iter().enumerate() {
            let angle = digit_angle(&self.digit_values, i, d.continuous, base);
            self.digit_angles[i] = if initialize {
                angle
            } else {
                lerp_angle(self.digit_angles[i], angle, dt * 20.)
            };
        }
    }
}

pub fn update_high_beam(
    lamp: &crate::reference::HighBeamLampReference,
    high_beam: bool,
) -> crate::lighting::LampMaterialOutput {
    let value = high_beam as u8 as f32;
    let on = value >= lamp.min_value && (value <= lamp.max_value || lamp.stay_on);
    let material = if on { &lamp.on } else { &lamp.off };
    crate::lighting::LampMaterialOutput {
        renderer: lamp.node,
        asset: material.asset,
        material_index: material.gltf_material,
    }
}
