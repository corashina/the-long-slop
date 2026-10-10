//! Source cycle outputs retain Unity units.
use crate::reference::{CycleReference, GradientReference};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CyclePhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CloudOutput {
    pub color: [f32; 4],
    pub density: f32,
    pub speed: f32,
}

/// Clear-weather source signals. Angles use Unity Euler degrees; colors retain source channels.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleOutput {
    pub phase: CyclePhase,
    pub progress: f32,
    pub cycle_seconds: f64,
    pub directional_euler_degrees: [f32; 3],
    pub directional_color: [f32; 4],
    pub directional_intensity: f32,
    pub ambient: f32,
    pub reflection_intensity: f32,
    pub sky_exposure: f32,
    pub sky_tint: [f32; 4],
    pub ground_color: [f32; 4],
    pub atmosphere_thickness: f32,
    pub fog_density: f32,
    pub fog_color: [f32; 4],
    pub roadside_enabled: bool,
    pub biome_color: [f32; 4],
    /// This visual input controls cloud density. It does not control engine heat.
    pub visual_temperature: f32,
    /// Source order: low layer, high layer.
    pub clouds: [CloudOutput; 2],
    pub cloud_noise_coordinate: f32,
    pub cloud_noise: f32,
    pub star_color: [f32; 4],
    /// Add this local X rotation to each exported star pose.
    pub star_rotation_degrees: f32,
    pub flare_progress: f32,
    pub flare_brightness: f32,
    /// Active shader time. The cloud shader uses time / 20 with each layer's speed.
    pub shader_time_seconds: f64,
}

#[derive(Clone, Debug)]
pub struct CycleState {
    elapsed_seconds: f64,
    cycle_seconds: f64,
    cloud_coordinate: f32,
    star_rotation: f32,
    output: CycleOutput,
}

impl CycleState {
    /// Initialize from a validated cycle reference.
    pub fn new(reference: &CycleReference) -> Self {
        let cycle_seconds = f64::from(reference.startup_offset_seconds);
        Self {
            elapsed_seconds: 0.,
            cycle_seconds,
            cloud_coordinate: 0.,
            star_rotation: 0.,
            output: evaluate(reference, cycle_seconds, 0., 0., 0.),
        }
    }

    /// Advance by active play time. Reject invalid or unrepresentable source intervals.
    pub fn advance(&mut self, reference: &CycleReference, elapsed_seconds: f64) {
        let dt = elapsed_seconds as f32;
        let active = self.elapsed_seconds + elapsed_seconds;
        let cycle = self.cycle_seconds + elapsed_seconds;
        let cloud_step = dt * reference.cloud_perlin_speed;
        let star_step = reference.star_speed * dt;
        if elapsed_seconds < 0.
            || !elapsed_seconds.is_finite()
            || !dt.is_finite()
            || !active.is_finite()
            || !cycle.is_finite()
            || !cloud_step.is_finite()
            || !star_step.is_finite()
        {
            return;
        }
        // FixedUpdate rotates stars before it evaluates the cycle.
        self.star_rotation = (self.star_rotation + star_step) % 360.;
        self.cloud_coordinate += cloud_step;
        // Source repeatedly subtracts one while the coordinate exceeds zero.
        if self.cloud_coordinate > 0. {
            self.cloud_coordinate -= self.cloud_coordinate.ceil();
        }
        self.elapsed_seconds = active;
        self.output = evaluate(
            reference,
            cycle,
            active,
            self.cloud_coordinate,
            self.star_rotation,
        );
        // The source changes startTime, then evaluates the old t in this update.
        self.cycle_seconds = if cycle as f32 > reference.day_seconds + reference.night_seconds {
            0.
        } else {
            cycle
        };
    }

    pub fn reset(&mut self, reference: &CycleReference) {
        *self = Self::new(reference);
    }

    /// Return monotonic active play time, without the source startup offset.
    pub fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }

    pub fn output(&self) -> &CycleOutput {
        &self.output
    }
}

fn evaluate(
    r: &CycleReference,
    seconds: f64,
    active: f64,
    cloud_x: f32,
    star_rotation: f32,
) -> CycleOutput {
    let t = seconds as f32;
    let (phase, p) = if t > r.day_seconds {
        (
            CyclePhase::Night,
            inverse_lerp(r.day_seconds, r.day_seconds + r.night_seconds, t),
        )
    } else if t > r.day_seconds - r.dusk_seconds {
        (
            CyclePhase::Dusk,
            inverse_lerp(r.day_seconds - r.dusk_seconds, r.day_seconds, t),
        )
    } else if t > r.dawn_seconds {
        (
            CyclePhase::Day,
            inverse_lerp(r.dawn_seconds, r.day_seconds - r.dusk_seconds, t),
        )
    } else {
        (CyclePhase::Dawn, inverse_lerp(0., r.dawn_seconds, t))
    };
    let (pitch, yaw, intensity, ambient, exposure, gradient_time) = match phase {
        CyclePhase::Dawn => (
            lerp(r.midnight_angle_dawn, r.day_angle_min, p),
            lerp(0., r.day_yaw_degrees, p),
            lerp(0., r.day_intensity, p),
            lerp(r.night_ambient, r.day_ambient, p),
            lerp(r.min_exposure, r.max_exposure, p),
            1. - p,
        ),
        CyclePhase::Day => (
            lerp(r.day_angle_min, r.day_angle_max, p),
            r.day_yaw_degrees,
            lerp(
                r.noon_intensity,
                r.day_intensity,
                inverse_lerp(0.45, 0.35, p).max(inverse_lerp(0.55, 0.7, p)),
            ),
            r.day_ambient,
            r.max_exposure,
            0.,
        ),
        CyclePhase::Dusk => (
            lerp(r.day_angle_max, r.midnight_angle_dusk, p),
            lerp(r.day_yaw_degrees, 0., p),
            lerp(r.day_intensity, 0., p),
            lerp(r.day_ambient, r.night_ambient, p),
            lerp(r.max_exposure, r.min_exposure, p),
            p,
        ),
        CyclePhase::Night => (
            lerp(r.min_moon_angle, r.max_moon_angle, p),
            0.,
            r.night_intensity,
            r.night_ambient,
            r.max_exposure,
            1.,
        ),
    };
    // SetBiome waits one second before its first desert-color assignment.
    let biome = if active >= f64::from(r.biome_refresh_seconds) {
        r.desert_color
    } else {
        [0.; 4]
    };
    let day_temp = biome_float(&r.temp_days, biome);
    let night_temp = biome_float(&r.temp_nights, biome);
    let noon_temp = biome_float(&r.temp_add_noons, biome);
    let temp = match phase {
        CyclePhase::Dawn => lerp(night_temp, day_temp, p),
        CyclePhase::Day => {
            day_temp
                + lerp(0., noon_temp, inverse_lerp(0.3, 0.55, p))
                + (lerp(noon_temp, 0., inverse_lerp(0.7, 9., p)) - noon_temp)
        }
        CyclePhase::Dusk => lerp(day_temp, night_temp, p),
        CyclePhase::Night => night_temp,
    };
    let fog_density_day = biome_float(&r.day_fog_densities, biome);
    let fog_density_night = biome_float(&r.night_fog_densities, biome);
    let fog_day = biome_color(&r.day_fog_colors, biome);
    let fog_night = biome_color(&r.night_fog_colors, biome);
    let (fog_density, fog_color) = match phase {
        CyclePhase::Dawn => (
            lerp(fog_density_night, fog_density_day, inverse_lerp(0., 0.5, p)),
            color_lerp(fog_night, fog_day, inverse_lerp(0.5, 1., p)),
        ),
        CyclePhase::Day => (fog_density_day, fog_day),
        CyclePhase::Dusk => (
            lerp(fog_density_day, fog_density_night, inverse_lerp(0.5, 1., p)),
            color_lerp(fog_day, fog_night, inverse_lerp(0., 0.5, p)),
        ),
        CyclePhase::Night => (fog_density_night, fog_night),
    };
    let cloud_noise = (native_cloud_noise(cloud_x) - 0.5) * 2. * r.cloud_perlin_spread;
    let cloud_density = inverse_lerp(r.temp_days[0] + r.temp_add_noons[0], r.temp_nights[0], temp);
    let clouds = std::array::from_fn(|i| {
        let cloud = &r.clouds[i];
        let color = match phase {
            CyclePhase::Dawn if p < 0.5 => color_lerp(
                cloud.night_color,
                cloud.transition_color,
                inverse_lerp(0.3, 0.5, p),
            ),
            CyclePhase::Dawn => color_lerp(cloud.transition_color, cloud.day_color, (p - 0.5) * 2.),
            CyclePhase::Day => cloud.day_color,
            CyclePhase::Dusk if p < 0.5 => {
                color_lerp(cloud.day_color, cloud.transition_color, p * 2.)
            }
            CyclePhase::Dusk => color_lerp(
                cloud.transition_color,
                cloud.night_color,
                inverse_lerp(0.5, 0.7, p),
            ),
            CyclePhase::Night => cloud.night_color,
        };
        CloudOutput {
            color,
            density: lerp(
                cloud.min_density,
                cloud.max_density,
                cloud_density + cloud_noise,
            ),
            // Start calls Felho(1). FixedUpdate does not change _Speed.
            speed: lerp(cloud.min_speed, cloud.mid_speed, inverse_lerp(0.5, 1., 1.)),
        }
    });
    let flare_night = biome_float(&r.flare_night, biome);
    let flare_day = biome_float(&r.flare_day, biome);
    let mut flare_progress = p;
    let flare_brightness = match phase {
        CyclePhase::Dawn => {
            if flare_progress < r.flare_lerp_start {
                flare_progress = lerp(
                    0.,
                    r.flare_lerp_end,
                    inverse_lerp(r.flare_lerp_start, r.flare_lerp_end, flare_progress),
                );
            }
            lerp3(
                flare_night,
                biome_float(&r.flare_dawn, biome),
                flare_day,
                flare_progress,
            )
        }
        CyclePhase::Day => flare_day,
        CyclePhase::Dusk => {
            if flare_progress > r.flare_lerp_start {
                flare_progress = lerp(
                    r.flare_lerp_start,
                    1.,
                    inverse_lerp(r.flare_lerp_start, r.flare_lerp_end, flare_progress),
                );
            }
            lerp3(
                flare_day,
                biome_float(&r.flare_dusk, biome),
                flare_night,
                flare_progress,
            )
        }
        CyclePhase::Night => flare_night,
    };
    CycleOutput {
        phase,
        progress: p,
        cycle_seconds: seconds,
        directional_euler_degrees: [pitch, yaw, 0.],
        directional_color: gradient(&r.color, gradient_time),
        directional_intensity: intensity,
        ambient,
        reflection_intensity: lerp(
            r.min_reflection,
            r.max_reflection,
            inverse_lerp(0., r.day_ambient, ambient),
        ),
        sky_exposure: exposure,
        sky_tint: biome_color(&r.sky_colors, biome),
        ground_color: match phase {
            CyclePhase::Dawn => color_lerp(
                biome_color(&r.night_ground_colors, biome),
                biome_color(&r.day_ground_colors, biome),
                p,
            ),
            CyclePhase::Day => biome_color(&r.day_ground_colors, biome),
            CyclePhase::Dusk => color_lerp(
                biome_color(&r.day_ground_colors, biome),
                biome_color(&r.night_ground_colors, biome),
                p,
            ),
            CyclePhase::Night => biome_color(&r.night_ground_colors, biome),
        },
        atmosphere_thickness: match phase {
            CyclePhase::Night => 0.,
            CyclePhase::Day => 1.,
            _ => gradient(&r.atmosphere_thickness, gradient_time)[0] * 255.,
        },
        fog_density: fog_density * r.fog_modifier,
        fog_color,
        roadside_enabled: match phase {
            CyclePhase::Dawn => p < 0.5,
            CyclePhase::Day => false,
            CyclePhase::Dusk => p > 0.5,
            CyclePhase::Night => true,
        },
        biome_color: biome,
        visual_temperature: temp,
        clouds,
        cloud_noise_coordinate: cloud_x,
        cloud_noise,
        star_color: match phase {
            CyclePhase::Dawn => color_lerp(r.star_color_night, r.star_color_day, p),
            CyclePhase::Day => r.star_color_day,
            CyclePhase::Dusk => color_lerp(r.star_color_day, r.star_color_night, p),
            CyclePhase::Night => r.star_color_night,
        },
        star_rotation_degrees: star_rotation,
        flare_progress,
        flare_brightness,
        shader_time_seconds: active,
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0., 1.)
}
fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.
    } else {
        ((value - a) / (b - a)).clamp(0., 1.)
    }
}
fn color_lerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    std::array::from_fn(|i| lerp(a[i], b[i], t))
}
fn biome_color(colors: &[[f32; 4]; 3], biome: [f32; 4]) -> [f32; 4] {
    color_lerp(
        color_lerp(colors[2], colors[0], biome[0]),
        colors[1],
        biome[2],
    )
}
fn biome_float(values: &[f32; 3], biome: [f32; 4]) -> f32 {
    lerp(lerp(values[2], values[0], biome[0]), values[1], biome[2])
}
fn lerp3(a: f32, b: f32, c: f32, t: f32) -> f32 {
    if t < 0.5 {
        lerp(a, b, inverse_lerp(0., 0.5, t))
    } else {
        lerp(b, c, inverse_lerp(0.5, 1., t))
    }
}
fn gradient(g: &GradientReference, time: f32) -> [f32; 4] {
    let time = time.clamp(0., 1.);
    let mut color = g.colors[0].value;
    for keys in g.colors.windows(2) {
        if time < keys[1].time {
            color = if g.mode == 1 {
                keys[0].value
            } else {
                color_lerp(
                    keys[0].value,
                    keys[1].value,
                    inverse_lerp(keys[0].time, keys[1].time, time),
                )
            };
            break;
        }
        color = keys[1].value;
    }
    let mut alpha = g.alphas[0].value;
    for keys in g.alphas.windows(2) {
        if time < keys[1].time {
            alpha = if g.mode == 1 {
                keys[0].value
            } else {
                lerp(
                    keys[0].value,
                    keys[1].value,
                    inverse_lerp(keys[0].time, keys[1].time, time),
                )
            };
            break;
        }
        alpha = keys[1].value;
    }
    color[3] = alpha;
    color
}

/// Installed UnityPlayer's checked slice: x in [-1, 0], y = 0. Keep each f32 operation separate.
fn native_cloud_noise(x: f32) -> f32 {
    let t = x.abs();
    if t == 1. {
        return 0.69_f32 / 1.483_f32;
    }
    let t2 = t * t;
    let t3 = t2 * t;
    let a = t * 6.;
    let b = a - 15.;
    let c = b * t;
    let d = c + 10.;
    let fade = d * t3;
    let g1 = t - 1.;
    let delta = g1 - t;
    let raw = delta * fade;
    let raw = raw + t;
    let value = raw + 0.69_f32;
    value / 1.483_f32
}

#[cfg(test)]
mod tests {
    use super::native_cloud_noise;

    #[test]
    fn native_cloud_noise_matches_checked_engine_bits() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/common/native_cloud_noise.json")).unwrap();
        for sample in fixture["samples"].as_array().unwrap() {
            let bits = |field: &str| {
                u32::from_str_radix(sample[field].as_str().unwrap().trim_start_matches("0x"), 16)
                    .unwrap()
            };
            let x = f32::from_bits(bits("x_bits"));
            assert_eq!(
                native_cloud_noise(x).to_bits(),
                bits("native_bits"),
                "x={x}"
            );
        }
    }
}
