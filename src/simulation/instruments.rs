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
    pub distance_m: f64,
}
pub struct InstrumentState {
    signals: [f32; 3],
    values: [f32; 3],
    digit_values: [f32; 5],
    pub needle_angles: [f32; 3],
    pub digit_angles: [f32; 5],
    pub lamp_bits: u32,
    pub distance_km: f32,
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0., 1.)
}
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut delta = (b - a).rem_euclid(360.);
    if delta > 180. {
        delta -= 360.;
    }
    a + delta * t.clamp(0., 1.)
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
        Self {
            signals: [0.; 3],
            values: [0.; 3],
            digit_values: std::array::from_fn(|i| r.odometer.digits[i].initial_value),
            needle_angles: std::array::from_fn(|i| needle_angle(&r.needles[i], 0.)),
            digit_angles: std::array::from_fn(|i| r.odometer.digits[i].initial_angle),
            lamp_bits: 0,
            distance_km: r.odometer.initial_value,
        }
    }
    pub fn reset(&mut self, r: &InstrumentReference) {
        *self = Self::new(r);
    }
    pub fn update(&mut self, r: &InstrumentReference, s: InstrumentSample, dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
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
            self.signals[i] = if n.signal_smoothing > 0. {
                lerp(self.signals[i], target, dt * n.signal_smoothing)
            } else {
                target
            };
            self.values[i] = if n.no_value_lerp {
                self.signals[i] + n.offset
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
                _ => false,
            };
            let v = if on { 1. } else { 0. };
            if v >= l.min_value && (v <= l.max_value || l.stay_on) {
                self.lamp_bits |= 1 << i;
            }
        }
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
            let angle = if i == 0 || d.continuous {
                self.digit_values[i] * 360. / base
            } else if self.digit_values[..i].iter().all(|v| *v >= base - 1.) {
                let floor = self.digit_values[i].trunc();
                lerp_angle(
                    floor * 360. / base,
                    (floor + 1.) * 360. / base,
                    ((self.digit_values[0] - (base - 1.)) / (1.)).clamp(0., 1.),
                )
            } else {
                self.digit_values[i].trunc() * 360. / base
            };
            self.digit_angles[i] = lerp_angle(self.digit_angles[i], angle, dt * 20.);
        }
    }
}
