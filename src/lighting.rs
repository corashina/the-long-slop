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
    terrain_color: [f32; 4],
    next_biome_refresh: f64,
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
            output: evaluate(reference, cycle_seconds, 0., 0., 0., [0.; 4]),
            terrain_color: [0.; 4],
            next_biome_refresh: reference.biome_refresh_seconds as f64,
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
            self.terrain_color,
        );
        // The source changes startTime, then evaluates the old t in this update.
        self.cycle_seconds = if cycle as f32 > reference.day_seconds + reference.night_seconds {
            0.
        } else {
            cycle
        };
    }

    pub fn needs_terrain_color(&self, dt: f64) -> bool {
        self.elapsed_seconds + dt >= self.next_biome_refresh
    }
    pub fn set_terrain_color(&mut self, reference: &CycleReference, color: &[f32; 4], dt: f64) {
        if self.needs_terrain_color(dt) {
            self.terrain_color = *color;
            self.next_biome_refresh =
                self.elapsed_seconds + dt + reference.biome_refresh_seconds as f64;
        }
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
    biome: [f32; 4],
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

#[derive(Clone, Copy, Debug)]
pub struct LightOutput {
    pub id: crate::reference::LightId,
    pub enabled: bool,
    pub intensity: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct LampMaterialOutput {
    pub renderer: crate::reference::NodeId,
    pub asset: crate::reference::AssetId,
    pub material_index: u32,
}
#[derive(Clone, Debug)]
pub struct ControlPoseOutput {
    pub node: crate::reference::NodeId,
    pub local_pose: crate::reference::Transform,
}
#[derive(Clone, Debug)]
pub struct HeadlightState {
    pub current_state: usize,
    pub requested_state: usize,
    pub energy: f32,
    pub enabled: bool,
    pub on: bool,
    refreshed_state: usize,
    energy_move: f32,
    state_count: usize,
}
impl HeadlightState {
    pub fn new(reference: &crate::reference::LightControllerReference) -> Self {
        Self {
            current_state: reference.current_state,
            requested_state: reference.requested_state,
            energy: reference.energy,
            enabled: reference.enabled,
            on: reference.startup_on,
            refreshed_state: reference.current_state,
            energy_move: reference.energy_move,
            state_count: reference.states.len(),
        }
    }
    pub fn request(&mut self, state: u8) {
        let state = usize::from(state);
        if state < self.state_count && state != self.requested_state && state != self.current_state
        {
            self.requested_state = state;
            self.enabled = true;
        }
    }
    pub fn update(&mut self, dt: f32) {
        if !self.enabled || !dt.is_finite() || dt < 0. {
            return;
        }
        if self.requested_state == self.current_state {
            if self.energy < 1. {
                self.energy = (self.energy + dt * self.energy_move).clamp(0., 1.);
            } else {
                self.enabled = false;
            }
            self.refreshed_state = self.current_state;
        } else if self.energy > 0. {
            self.energy = (self.energy - dt * self.energy_move).clamp(0., 1.);
            self.refreshed_state = self.current_state;
        } else {
            // Source RefreshLights runs before the current-state assignment.
            self.refreshed_state = self.current_state;
            self.current_state = self.requested_state;
        }
    }
    pub fn current_state(&self) -> u8 {
        self.current_state as u8
    }
    pub fn energy(&self) -> f32 {
        self.energy
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct VehicleLightSample {
    pub local_ignition: bool,
    pub brake: f32,
    pub gear_ratio: f32,
    pub electronics_output: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct NodeVisibilityOutput {
    pub node: crate::reference::NodeId,
    pub visible: bool,
}
#[derive(Clone, Copy)]
enum SwitchMotion {
    Initial,
    Moving(f32),
}
pub struct LightingState {
    pub cycle: CycleState,
    requested_headlights: u8,
    mounted: Vec<bool>,
    vehicle: VehicleLightSample,
    high_beam: bool,
    dashboard_index: usize,
    pub(crate) controllers: Vec<HeadlightState>,
    pub(crate) lights: Vec<LightOutput>,
    pub(crate) materials: Vec<LampMaterialOutput>,
    pub(crate) poses: Vec<ControlPoseOutput>,
    visibility: Vec<NodeVisibilityOutput>,
    cabin_modes: Vec<usize>,
    pose_progress: Vec<Option<SwitchMotion>>,
}
impl LightingState {
    pub fn new(reference: &crate::reference::LightingRecord) -> Self {
        let mut state = Self {
            cycle: CycleState::new(&reference.cycle),
            visibility: reference
                .controllers
                .iter()
                .flat_map(|c| &c.states)
                .flat_map(|s| {
                    s.working_nodes
                        .iter()
                        .map(|node| NodeVisibilityOutput {
                            node: *node,
                            visible: !s.broken,
                        })
                        .chain(s.broken_nodes.iter().map(|node| NodeVisibilityOutput {
                            node: *node,
                            visible: s.broken,
                        }))
                })
                .collect(),
            cabin_modes: reference
                .controllers
                .iter()
                .map(|c| c.cabin_input.as_ref().map_or(0, |input| input.startup_mode))
                .collect(),
            pose_progress: vec![None; reference.controllers.len()],
            requested_headlights: 0,
            mounted: vec![true; reference.controllers.len()],
            vehicle: VehicleLightSample::default(),
            high_beam: false,
            dashboard_index: 0,
            controllers: reference
                .controllers
                .iter()
                .map(HeadlightState::new)
                .collect(),
            lights: reference
                .lights
                .iter()
                .map(|l| LightOutput {
                    id: l.id,
                    enabled: l.enabled,
                    intensity: l.startup_intensity,
                })
                .collect(),
            materials: Vec::with_capacity(
                reference
                    .controllers
                    .iter()
                    .flat_map(|c| &c.states)
                    .map(|s| s.renderers.len())
                    .sum::<usize>()
                    + 1,
            ),
            poses: Vec::with_capacity(
                reference
                    .controllers
                    .iter()
                    .map(|c| c.animated_poses.len())
                    .sum(),
            ),
        };
        state.reset(reference);
        state
    }
    pub fn reset(&mut self, reference: &crate::reference::LightingRecord) {
        self.cycle.reset(&reference.cycle);
        self.mounted.fill(true);
        self.materials.clear();
        self.poses.clear();
        for (index, c) in reference.controllers.iter().enumerate() {
            self.cabin_modes[index] = c.cabin_input.as_ref().map_or(0, |input| input.startup_mode);
            self.pose_progress[index] = None;
        }
        for (out, light) in self.lights.iter_mut().zip(&reference.lights) {
            *out = LightOutput {
                id: light.id,
                enabled: light.enabled,
                intensity: light.startup_intensity,
            };
        }
        for (state, control) in self.controllers.iter_mut().zip(&reference.controllers) {
            *state = HeadlightState::new(control);
            for lamp in &control.states {
                if let Some(material) = if control.startup_on {
                    &lamp.on_material
                } else {
                    &lamp.off_material
                } {
                    for &renderer in &lamp.renderers {
                        if let Some(out) = self
                            .materials
                            .iter_mut()
                            .find(|out| out.renderer == renderer)
                        {
                            *out = LampMaterialOutput {
                                renderer,
                                asset: material.asset,
                                material_index: material.gltf_material,
                            };
                        } else {
                            self.materials.push(LampMaterialOutput {
                                renderer,
                                asset: material.asset,
                                material_index: material.gltf_material,
                            });
                        }
                    }
                }
            }
            for pose in &control.animated_poses {
                if let Some(local_pose) = pose
                    .states
                    .get(usize::from(control.startup_on ^ control.reverse_switch))
                {
                    self.poses.push(ControlPoseOutput {
                        node: pose.node,
                        local_pose: crate::reference::Transform {
                            translation: local_pose.translation,
                            rotation: local_pose.rotation,
                            scale: [1.; 3],
                        },
                    });
                }
            }
        }
        self.requested_headlights = reference
            .controllers
            .iter()
            .find(|c| c.kind == crate::reference::LightControllerKind::Headlight)
            .unwrap()
            .requested_state as u8;
        self.dashboard_index = self.materials.len();
        self.materials
            .push(crate::simulation::instruments::update_high_beam(
                &reference.high_beam,
                false,
            ));
        let electronics_output = reference
            .controllers
            .iter()
            .find(|c| c.kind == crate::reference::LightControllerKind::Rear)
            .unwrap()
            .electronics_output;
        for index in 0..reference.controllers.len() {
            if reference.controllers[index].kind == crate::reference::LightControllerKind::House {
                self.refresh_house(reference, index);
            }
        }
        self.update_vehicle(
            reference,
            VehicleLightSample {
                electronics_output,
                ..Default::default()
            },
        );
    }
    pub fn use_control(
        &mut self,
        reference: &crate::reference::LightingRecord,
        control: crate::reference::LightControlId,
    ) {
        let Some(index) = reference.controllers.iter().position(|c| c.id == control) else {
            return;
        };
        let c = &reference.controllers[index];
        match c.kind {
            crate::reference::LightControllerKind::House => {
                self.controllers[index].on = !self.controllers[index].on;
                if self.pose_progress[index].is_none() {
                    self.pose_progress[index] = Some(SwitchMotion::Initial);
                }
                self.refresh_house(reference, index);
            }
            crate::reference::LightControllerKind::Cabin => {
                let input = c.cabin_input.as_ref().unwrap();
                self.cabin_modes[index] = (self.cabin_modes[index] + 1) % input.modes.len();
            }
            crate::reference::LightControllerKind::Headlight => self.cycle_headlights(reference),
            _ => {}
        }
    }
    fn refresh_house(&mut self, reference: &crate::reference::LightingRecord, index: usize) {
        let c = &reference.controllers[index];
        let on = self.controllers[index].on
            && (c.electronics_source.is_none() || c.electronics_output > c.electronics_threshold);
        for lamp in &c.states {
            for id in &lamp.lights {
                let output = self.lights.iter_mut().find(|o| o.id == *id).unwrap();
                output.enabled = on && !lamp.broken;
                output.intensity = lamp.intensity;
            }
            select_material(&mut self.materials, lamp, on && !lamp.broken);
            for node in &lamp.working_nodes {
                self.visibility
                    .iter_mut()
                    .find(|o| o.node == *node)
                    .unwrap()
                    .visible = !lamp.broken;
            }
            for node in &lamp.broken_nodes {
                self.visibility
                    .iter_mut()
                    .find(|o| o.node == *node)
                    .unwrap()
                    .visible = lamp.broken;
            }
        }
    }
    pub fn update_cabin(
        &mut self,
        reference: &crate::reference::LightingRecord,
        door_values: [f32; 4],
    ) {
        for (index, c) in reference.controllers.iter().enumerate() {
            let Some(input) = &c.cabin_input else {
                continue;
            };
            let on = match input.modes[self.cabin_modes[index]] {
                1 => true,
                2 => false,
                _ => door_values.iter().any(|value| *value > input.threshold),
            };
            self.controllers[index].request(u8::from(on));
        }
    }
    pub fn visibility_outputs(&self) -> &[NodeVisibilityOutput] {
        &self.visibility
    }
    pub fn cycle_headlights(&mut self, reference: &crate::reference::LightingRecord) {
        self.requested_headlights = (self.requested_headlights + 1) % 3;
        for (state, control) in self.controllers.iter_mut().zip(&reference.controllers) {
            if control.kind == crate::reference::LightControllerKind::Headlight {
                state.request(self.requested_headlights);
            }
        }
        self.update_vehicle(reference, self.vehicle);
    }
    pub fn set_controller_mounted(&mut self, index: usize, mounted: bool) {
        self.mounted[index] = mounted;
    }
    pub fn headlights(&self) -> u8 {
        self.requested_headlights
    }
    pub fn high_beam(&self) -> bool {
        self.high_beam
    }
    pub fn update_vehicle(
        &mut self,
        reference: &crate::reference::LightingRecord,
        sample: VehicleLightSample,
    ) {
        self.vehicle = sample;
        let electronics = reference
            .controllers
            .iter()
            .find(|c| c.kind == crate::reference::LightControllerKind::Rear)
            .unwrap();
        self.high_beam = self.requested_headlights == 2
            && (electronics.electronics_source.is_none()
                || sample.electronics_output > electronics.electronics_threshold);
        let output =
            crate::simulation::instruments::update_high_beam(&reference.high_beam, self.high_beam);
        self.materials[self.dashboard_index] = output;
        for control in &reference.controllers {
            if control.kind != crate::reference::LightControllerKind::Rear {
                continue;
            }
            let mut lamps = control.states.iter();
            while let Some(lamp) = if sample.local_ignition {
                lamps.next()
            } else {
                lamps.next_back()
            } {
                let on = sample.local_ignition
                    && match lamp.signal {
                        crate::reference::LampSignal::Tail => self.requested_headlights != 0,
                        crate::reference::LampSignal::Brake => sample.brake > 0.01,
                        crate::reference::LampSignal::Reverse => sample.gear_ratio < 0.,
                        _ => false,
                    };
                for id in &lamp.lights {
                    let output = self.lights.iter_mut().find(|o| o.id == *id).unwrap();
                    output.enabled = on;
                    output.intensity = lamp.intensity;
                }
                if on || !sample.local_ignition || !lamp.dont_overwrite {
                    select_material(&mut self.materials, lamp, on);
                }
            }
        }
    }
    pub fn advance_fades(&mut self, reference: &crate::reference::LightingRecord, dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        for (index, control) in reference.controllers.iter().enumerate() {
            if control.kind == crate::reference::LightControllerKind::House {
                if let Some(motion) = self.pose_progress[index] {
                    let progress = match motion {
                        SwitchMotion::Initial => 0.,
                        SwitchMotion::Moving(progress) => progress + dt * 25.,
                    };
                    if progress >= 1. {
                        let target =
                            usize::from(self.controllers[index].on ^ control.reverse_switch);
                        for pose in &control.animated_poses {
                            let out = self.poses.iter_mut().find(|o| o.node == pose.node).unwrap();
                            out.local_pose.translation = pose.states[target].translation;
                            out.local_pose.rotation = pose.states[target].rotation;
                        }
                        self.pose_progress[index] = None;
                        continue;
                    }
                    let target = usize::from(self.controllers[index].on ^ control.reverse_switch);
                    for pose in &control.animated_poses {
                        let out = self.poses.iter_mut().find(|o| o.node == pose.node).unwrap();
                        let next = pose.states[target];
                        for i in 0..3 {
                            out.local_pose.translation[i] =
                                lerp(out.local_pose.translation[i], next.translation[i], progress);
                        }
                        let a = rapier3d::math::Rotation::from_array(out.local_pose.rotation);
                        let b = rapier3d::math::Rotation::from_array(next.rotation);
                        out.local_pose.rotation = a.lerp(b, progress).normalize().to_array();
                    }
                    self.pose_progress[index] = Some(SwitchMotion::Moving(progress));
                }
                continue;
            }
            if !matches!(
                control.kind,
                crate::reference::LightControllerKind::Headlight
                    | crate::reference::LightControllerKind::Cabin
            ) {
                continue;
            }
            let state = &mut self.controllers[index];
            if control.kind == crate::reference::LightControllerKind::Headlight
                && !self.mounted[index]
            {
                state.current_state = 0;
                state.refreshed_state = 0;
                state.energy = 0.;
                state.enabled = true;
                for lamp in &control.states {
                    for id in &lamp.lights {
                        let output = self.lights.iter_mut().find(|o| o.id == *id).unwrap();
                        output.enabled = false;
                        output.intensity = 0.;
                    }
                    select_material(&mut self.materials, lamp, false);
                }
                continue;
            }
            if control.kind == crate::reference::LightControllerKind::Headlight {
                state.request(self.requested_headlights);
            }
            state.update(dt);
            let energy = state.energy
                * if control.electronics_source.is_some() {
                    control.electronics_output
                } else {
                    1.
                };
            // Clear other states first. Shared source renderers then select the current state.
            for (index, lamp) in control.states.iter().enumerate() {
                if index == state.refreshed_state {
                    continue;
                }
                for id in &lamp.lights {
                    self.lights
                        .iter_mut()
                        .find(|o| o.id == *id)
                        .unwrap()
                        .enabled = false;
                }
                select_material(&mut self.materials, lamp, false);
            }
            let lamp = &control.states[state.refreshed_state];
            for id in &lamp.lights {
                let output = self.lights.iter_mut().find(|o| o.id == *id).unwrap();
                output.enabled = true;
                output.intensity = lamp.intensity * energy;
            }
            select_material(&mut self.materials, lamp, energy > 0.);
        }
    }
    /// New road instances use the current phase and electronics output.
    pub fn road_lamp_output(&self, reference: &crate::reference::LightingRecord) -> LightOutput {
        let road = &reference.road_lamp;
        let controller = reference
            .controllers
            .iter()
            .find(|c| c.id == road.controller)
            .unwrap();
        LightOutput {
            id: road.lights[0],
            enabled: road.global_enabled
                && self.cycle.output().roadside_enabled
                && controller.electronics_output > road.electronics_threshold,
            intensity: controller.electronics_output,
        }
    }
    pub fn light_outputs(&self) -> &[LightOutput] {
        &self.lights
    }
    pub fn material_outputs(&self) -> &[LampMaterialOutput] {
        &self.materials
    }
    pub fn control_poses(&self) -> &[ControlPoseOutput] {
        &self.poses
    }
}
fn select_material(
    outputs: &mut [LampMaterialOutput],
    lamp: &crate::reference::LampStateReference,
    on: bool,
) {
    let material = if on {
        lamp.on_material.as_ref().or(lamp.off_material.as_ref())
    } else {
        lamp.off_material.as_ref()
    };
    if let Some(material) = material {
        for renderer in &lamp.renderers {
            let output = outputs
                .iter_mut()
                .find(|o| o.renderer == *renderer)
                .expect("prepared lamp slot");
            output.asset = material.asset;
            output.material_index = material.gltf_material;
        }
    }
}
/// Unity 2021.3 gamma conversion. HDR channels use the native 2.2 exponent.
pub fn source_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else if value < 1. {
        ((value + 0.055) / 1.055).powf(2.4)
    } else {
        value.powf(2.2)
    }
}
/// Checked native byte attenuation table. The final texel is zero.
pub fn attenuation_table() -> [u8; 1024] {
    std::array::from_fn(|i| {
        if i == 1023 {
            return 0;
        }
        let s = i as f32 / 1024.;
        let mut value = 1. / (s * 25. + 1.);
        let start = f32::from_bits(0x3f23d70b);
        if s >= start {
            value *= 1. - (s - start) / f32::from_bits(0x3eb851ea);
        }
        (value * 255. + 0.5) as u8
    })
}

/// Keep the source repeat cookie phase bounded after a floating-origin shift.
pub fn directional_cookie_phase(
    basis: [[f32; 3]; 2],
    origin: [f64; 3],
    anchor: [f32; 3],
    size: f32,
) -> [f32; 2] {
    basis.map(|row| {
        let projected = (0..3)
            .map(|axis| f64::from(row[axis]) * (origin[axis] - f64::from(anchor[axis])))
            .sum::<f64>();
        (projected / f64::from(size)).rem_euclid(1.) as f32
    })
}

/// Native spotlight projection from the exported right-handed pose, without transform scale.
pub fn spot_cookie_matrix(
    pose: &crate::reference::Transform,
    range: f32,
    cot_half_angle: f32,
) -> [[f32; 4]; 4] {
    use rapier3d::math::{Rotation, Vector};
    let inverse = Rotation::from_array(pose.rotation).inverse();
    let scale = 1. / range;
    let projected_scale = (2. / cot_half_angle) * scale;
    [
        Vector::X,
        Vector::Y,
        Vector::Z,
        -Vector::from_array(pose.translation),
    ]
    .map(|column| {
        let local = inverse * column;
        // Export reflects Unity Z. X and Y keep their native cookie signs.
        [
            local.x * scale,
            local.y * scale,
            -local.z * scale,
            -local.z * projected_scale,
        ]
    })
}
