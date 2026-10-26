//! Vehicle records of car world prototypes and engine, radiator and gauge records of part nodes.
//! Every `u32` id is local to its world prototype: node, body or collider index.
use super::render_lighting::{CaptureFace, ProbeRecord, ProbeRefresh};
use super::*;

/// Source `carscript` of a car world prototype root.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldVehicleReference {
    pub source: SourceId,
    /// Chassis body: the root Rigidbody (`carscript.RB`).
    pub body: u32,
    pub car_type: i32,
    /// Enabled chassis colliders.
    pub colliders: Vec<u32>,
    /// Source `massScript` masses of active nodes. Tank fluids add at runtime.
    pub mass_kg: f32,
    pub serialized_mass_kg: f32,
    /// Root `tankscript` node.
    pub fuel_tank: Option<u32>,
    /// Source `carscript.tankslot`. Its part holds the fuel tank when the root has none.
    pub fuel_tank_slot: Option<SourceId>,
    /// `carscript.electronics` node.
    pub electronics: Option<u32>,
    /// Source `carscript.startingTime`.
    pub starter_delay_seconds: f32,
    pub driving: DrivingVehicleReference,
    pub handbrake: bool,
    pub hub: WheelHubReference,
    pub wheels: Vec<WorldWheelReference>,
    pub gear_ratios: Vec<f32>,
    pub gear_free_run: Vec<bool>,
    pub final_drive_ratio: f32,
    pub max_steer_degrees: f32,
    pub brake_torque_nm: f32,
    pub handbrake_torque_nm: f32,
    pub wheel_slip: [f32; 4],
    pub rim_slip: [f32; 2],
    pub auto_shift_stop_speed: f32,
    pub auto_shift_min_rpm: f32,
    pub auto_shift_max_rpm: f32,
    pub dont_ignite_with_throttle_in_automatic: bool,
    pub auto_move_min: f32,
    pub collision_gain: f32,
    /// Source `carscript.psSmoke` systems, posed relative to the prototype root.
    pub exhaust: Vec<ParticleReference>,
    /// One `talajnezo` per wheel, in `wheels` order. Poses are relative to the prototype root.
    pub wheel_effects: Vec<WheelEffectsReference>,
    pub effect_gains: EffectGains,
    pub seats: Vec<WorldSeatReference>,
    pub lights: Vec<WorldLightReference>,
    pub light_switch: Option<WorldLightSwitchReference>,
    /// Node ids are prototype node indices.
    pub instruments: Option<InstrumentReference>,
    /// Lamp thresholds and electronics output of every gauge on the car.
    pub gauge_limits: GaugeLimits,
    /// Source `carscript.kmhmeters`: gauge part slots.
    pub kmh_meters: Vec<KmhMeterReference>,
    /// Indices of `headlight2` lamps in `instruments.lamps`.
    pub high_beams: Vec<u32>,
    /// Source `rendszamscript` labels of the license plates.
    pub plates: Vec<WorldPlateReference>,
    /// The realtime reflection probe that follows this car.
    pub probe: Option<WorldProbeReference>,
    pub audio: Option<CarAudioReference>,
}

/// Source `kmhslottype`: the meter of the part in each slot reads `types`, times `modifier`.
/// With `force`, every needle of that meter reads the speed (source behavior).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KmhMeterReference {
    pub force: bool,
    pub modifier: f32,
    pub types: Vec<String>,
    /// `partslotscript` components.
    pub slots: Vec<SourceId>,
}

/// A realtime `ReflectionProbe` of a car prototype. The fields are those of `ProbeRecord`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldProbeReference {
    pub source: SourceId,
    /// The probe node.
    pub node: u32,
    pub enabled: bool,
    pub bounds_size: [f32; 3],
    pub bounds_offset: [f32; 3],
    pub box_projection: bool,
    pub blend_distance: f32,
    pub intensity: f32,
    pub importance: i32,
    pub near: f32,
    pub far: f32,
    pub shadow_distance: f32,
    pub mask: u32,
    pub resolution: u32,
    pub refresh: ProbeRefresh,
    pub decode: [f32; 4],
    pub faces: Vec<CaptureFace>,
    pub mip_extents: Vec<u32>,
}
impl WorldProbeReference {
    /// The capture and influence values equal those of `shared`.
    pub fn matches(&self, shared: &ProbeRecord) -> bool {
        self.enabled == shared.enabled
            && self.bounds_size == shared.bounds_size
            && self.bounds_offset == shared.bounds_offset
            && self.box_projection == shared.box_projection
            && self.blend_distance == shared.blend_distance
            && self.intensity == shared.intensity
            && self.importance == shared.importance
            && self.near == shared.near
            && self.far == shared.far
            && self.shadow_distance == shared.shadow_distance
            && self.mask == shared.mask
            && self.resolution == shared.resolution
            && self.refresh == shared.refresh
            && self.decode == shared.decode
            && self.faces.len() == shared.faces.len()
            && self
                .faces
                .iter()
                .zip(&shared.faces)
                .all(|(a, b)| a.index == b.index && a.basis == b.basis)
            && self.mip_extents == shared.mip_extents
    }
}

/// A license plate `rendszamscript` label. It uses the shared plate font and material.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPlateReference {
    pub source: SourceId,
    /// The plate node with the script.
    pub node: u32,
    /// The TextMeshPro node.
    pub text_node: u32,
    /// `sameAsThis`: the node of the label that copies this label's text.
    pub same_as: Option<u32>,
    pub initial_text: String,
    pub initial_same: bool,
    pub font_source: SourceId,
    pub material_source: SourceId,
    /// Text renderer sorting layer and order.
    pub sorting: [i16; 2],
    pub layout: PlateLayoutReference,
}

/// A road WheelCollider of `carscript` and its wheel slot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldWheelReference {
    pub source: SourceId,
    pub node: u32,
    pub collider: WheelColliderReference,
    pub slot: u32,
    pub mount: u32,
    pub steer: bool,
    pub drive: bool,
    pub brake: bool,
    pub handbrake: bool,
    pub sideways_stiffness_modifier: f32,
}

/// The driver roles of a primary `seatscript`. The seat itself binds from the prototype.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSeatReference {
    pub source: SourceId,
    pub driver: bool,
    pub driving2: bool,
}

/// A `headlightscript` (headlight or cabin) or `rearlights` controller.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLightReference {
    pub source: SourceId,
    pub kind: LightControllerKind,
    pub node: u32,
    pub electronics: Option<u32>,
    pub startup_on: bool,
    pub current_state: usize,
    pub requested_state: usize,
    pub energy: f32,
    pub energy_move: f32,
    pub enabled: bool,
    pub reverse_switch: bool,
    pub states: Vec<WorldLampStateReference>,
    pub cabin: Option<WorldCabinInputReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLampStateReference {
    pub signal: LampSignal,
    /// Nodes of the source Light components.
    pub lights: Vec<u32>,
    pub intensity: f32,
    pub renderers: Vec<u32>,
    pub on_material: Option<MaterialLink>,
    pub off_material: Option<MaterialLink>,
    pub broken: bool,
    pub working_nodes: Vec<u32>,
    pub broken_nodes: Vec<u32>,
    pub dont_overwrite: bool,
}

/// Source `inputconnect` that routes a cabin lamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldCabinInputReference {
    pub source: SourceId,
    pub switch_node: u32,
    pub modes: Vec<u8>,
    pub startup_mode: usize,
    pub door_slots: Vec<u32>,
    pub threshold: f32,
    pub include_children: bool,
    pub missing_slot_is_same: bool,
}

/// The `usablescript` that calls `carscript.InteractSwitchLights`, and the switch pose nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldLightSwitchReference {
    pub usable: u32,
    pub nodes: Vec<u32>,
    /// `LightOff`, `LightLow`, `LightHigh` nodes.
    pub state_nodes: [Vec<u32>; 3],
}

/// Source `enginescript` of a part node.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnginePartReference {
    pub source: SourceId,
    pub node: u32,
    /// `enginescript.T` oil tank node.
    pub oil_tank: Option<u32>,
    pub start_chance: f32,
    pub starter_rpm: f32,
    pub starter_rpm_lerp: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
    pub shutdown_rpm: f32,
    pub max_torque_nm: f32,
    pub starting_push_force: f32,
    pub consumption_modifier: f32,
    pub oil_consumption_modifier: f32,
    pub engine_brake_modifier: f32,
    pub rpm_change_modifier: f32,
    pub engine_start_time_modifier: f32,
    pub heat_gain_min: f32,
    pub heat_gain_max: f32,
    pub heat_loss: f32,
    pub initial_temperature: f32,
    pub heat_loss_speed_modifier: f32,
    pub optimal_temperature: [f32; 4],
    /// Source `enginescript.FuelConsumption`, by fluid type.
    pub fuel_consumption: [f32; 6],
    pub two_stroke: bool,
    pub torque_curve: TorqueCurve,
    pub max_rpm_difference: f32,
    pub torque_modifier: f32,
    pub oil_fluid: u8,
    pub no_overheat: bool,
    pub kickstart: bool,
    pub start_chance_interval_seconds: f32,
    pub auto_shift_clutch_rpm: f32,
    pub auto_shift_clutch_rpm_range: f32,
    pub smoke_rate: f32,
    pub audio: Option<EngineAudioReference>,
}

/// A car voice template of `AudioReference::car_voices` and its position in the owner frame.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarSoundReference {
    pub voice: u32,
    pub position: [f32; 3],
}

/// A clip that a one-shot car voice template plays once.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarShotReference {
    pub voice: u32,
    pub clip: u32,
    pub gain: f32,
    pub position: [f32; 3],
}

/// Source `enginescript` sounds. Positions are in the engine part frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineAudioReference {
    /// `SAlap` loop.
    pub idle: CarSoundReference,
    /// `SMotor` loop.
    pub motor: CarSoundReference,
    /// `SStart`, repeated while the starter turns.
    pub starter: Option<CarShotReference>,
    /// `SStop`, when the engine stops.
    pub stop: Option<CarShotReference>,
    /// `SMinAlapRpm`, `SMaxAlapRpm`.
    pub idle_rpm: [f32; 2],
    /// `SMinMotorRpm`, `SMaxMotorRpm`.
    pub motor_rpm: [f32; 2],
    /// `SMaxAlapSound`.
    pub idle_gain: f32,
    /// `SMaxMotorSound`.
    pub motor_gain: f32,
    /// `minAlapPitch`, `maxAlapPitch`.
    pub idle_pitch: [f32; 2],
    /// `minMotorPitch`, `maxMotorPitch`.
    pub motor_pitch: [f32; 2],
}

/// Source `carscript` and `talajnezo` sounds of a car. Positions are in the chassis frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarAudioReference {
    pub horn: Option<HornReference>,
    /// `turnedRadioSOund` on `SSounds`: ignition and light switch.
    pub switch: Option<CarShotReference>,
    /// One entry per wheel, in wheel effect order.
    pub wheels: Vec<WheelSoundsReference>,
    pub route: CarRouteReference,
    /// `busdooropenscript` leaves of a bus, in hierarchy order.
    pub doors: Vec<BusDoorAudioReference>,
}

/// Source `busdooropenscript.sounds`: `Open` and `Close` play one at random at the leaf.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusDoorAudioReference {
    pub source: SourceId,
    pub sounds: Vec<CarShotReference>,
}

/// Source `carscript.SHornSound`. Pitch follows the ambient temperature.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HornReference {
    pub sound: CarSoundReference,
    pub pitch: [f32; 2],
    pub temperature: [f32; 2],
    pub attack_release_rate: f32,
}

/// Source `talajnezo` GroundSource, SpringSource and SlideSource of one wheel.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelSoundsReference {
    pub ground: CarSoundReference,
    pub spring: CarSoundReference,
    pub slide: CarSoundReference,
}

/// Source `audioBasedOnInteriorsensscript` of a car prototype.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarRouteReference {
    /// Indices in the prototype `interiors`.
    pub interiors: Vec<u32>,
    pub main_mixer: u32,
    pub outside_mixer: u32,
    pub car_mixer: Option<u32>,
}

/// Source `coolantTankscript` of a part node.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiatorPartReference {
    pub source: SourceId,
    pub node: u32,
    pub coolant_tank: Option<u32>,
    pub fan_node: Option<u32>,
    pub thermostat_range: [f32; 2],
    pub fan_temperature_on: f32,
    pub fan_temperature_off: f32,
    pub fan_on_when_engine_runs: bool,
    pub max_value: f32,
    pub max_fan_value: f32,
    pub max_fan_speed: f32,
    pub max_fluid_loss: f32,
}

/// Local id ranges of one world prototype.
#[derive(Clone, Copy)]
pub(super) struct PrototypeBounds<'a> {
    pub nodes: usize,
    pub bodies: usize,
    pub colliders: usize,
    pub asset: &'a AssetRecord,
}

impl PrototypeBounds<'_> {
    fn node(&self, id: u32) -> bool {
        (id as usize) < self.nodes
    }
    fn nodes<'b>(&self, ids: impl IntoIterator<Item = &'b u32>) -> bool {
        ids.into_iter().all(|id| self.node(*id))
    }
    fn material(&self, m: &MaterialLink) -> bool {
        m.asset == self.asset.id && m.gltf_material < self.asset.material_count
    }
}

fn check(source: &SourceId, name: &str, ok: bool) -> Result<(), ReferenceError> {
    require(ok, &format!("{}:{} {name}", source.file, source.path_id))
}

fn curve_valid(c: &TorqueCurve) -> bool {
    !c.keys.is_empty()
        && c.keys.iter().all(|k| {
            finite(&[
                k.time,
                k.value,
                k.in_tangent,
                k.out_tangent,
                k.in_weight,
                k.out_weight,
            ]) && k.weighted_mode <= 3
        })
        && c.keys.windows(2).all(|k| k[0].time < k[1].time)
}

impl WorldVehicleReference {
    /// Every material that the car can show on renderer node `node`: lamp states, gauge lamps,
    /// odometer digits and backlights.
    pub fn renderer_materials(&self, node: u32) -> impl Iterator<Item = &MaterialLink> {
        let lights = self
            .lights
            .iter()
            .flat_map(|l| &l.states)
            .filter(move |s| s.renderers.contains(&node))
            .flat_map(|s| s.on_material.iter().chain(&s.off_material));
        let instruments = self.instruments.iter().flat_map(move |r| {
            let lamps = r
                .lamps
                .iter()
                .filter(move |l| l.node.0 == node)
                .flat_map(|l| [&l.on, &l.off]);
            let digits = r
                .digits()
                .iter()
                .filter(move |d| d.node.0 == node)
                .flat_map(|d| [&d.on, &d.off]);
            let backlights = r
                .all_backlights()
                .filter_map(move |b| match b {
                    MaterialSwitchReference::Renderer { node: n, on, off } if n.0 == node => {
                        Some([on, off])
                    }
                    _ => None,
                })
                .flatten();
            lamps.chain(digits).chain(backlights)
        });
        lights.chain(instruments)
    }
    pub(super) fn validate(&self, b: PrototypeBounds) -> Result<(), ReferenceError> {
        let s = &self.source;
        check(
            s,
            "vehicle body/colliders/tanks",
            (self.body as usize) < b.bodies
                && self.colliders.iter().all(|i| (*i as usize) < b.colliders)
                && self.fuel_tank.is_none_or(|i| b.node(i))
                && self.electronics.is_none_or(|i| b.node(i)),
        )?;
        check(
            s,
            "vehicle scalars",
            finite(&[
                self.mass_kg,
                self.serialized_mass_kg,
                self.starter_delay_seconds,
                self.final_drive_ratio,
                self.max_steer_degrees,
                self.brake_torque_nm,
                self.handbrake_torque_nm,
                self.auto_shift_stop_speed,
                self.auto_shift_min_rpm,
                self.auto_shift_max_rpm,
                self.auto_move_min,
                self.collision_gain,
                self.hub.radius_m,
                self.hub.mass_kg,
                self.hub.forward_stiffness,
                self.hub.sideways_stiffness,
            ]) && finite(&self.wheel_slip)
                && finite(&self.rim_slip)
                && self.mass_kg > 0.
                && self.serialized_mass_kg > 0.
                && self.hub.radius_m > 0.
                && self.hub.mass_kg > 0.
                && self.auto_shift_min_rpm < self.auto_shift_max_rpm,
        )?;
        check(
            s,
            "vehicle gears",
            !self.gear_ratios.is_empty()
                && finite(&self.gear_ratios)
                && self.gear_free_run.len() == self.gear_ratios.len()
                && self.driving.initial_gear < self.gear_ratios.len(),
        )?;
        check(
            s,
            "vehicle plates",
            self.plates.iter().all(|p| {
                b.node(p.node)
                    && b.node(p.text_node)
                    && p.layout.valid()
                    && p.same_as.is_none_or(|n| {
                        n != p.node && self.plates.iter().any(|other| other.node == n)
                    })
            }) && self
                .plates
                .iter()
                .enumerate()
                .all(|(i, p)| self.plates[..i].iter().all(|o| o.node != p.node)),
        )?;
        check(
            s,
            "vehicle probe node",
            self.probe.as_ref().is_none_or(|p| b.node(p.node)),
        )?;
        check(s, "vehicle wheels", !self.wheels.is_empty())?;
        for (i, w) in self.wheels.iter().enumerate() {
            check(
                s,
                &format!("vehicle wheel {i}"),
                b.nodes([&w.node, &w.slot, &w.mount])
                    && w.collider.valid()
                    && w.sideways_stiffness_modifier.is_finite(),
            )?;
        }
        for (i, light) in self.lights.iter().enumerate() {
            let states_valid = light.states.iter().all(|st| {
                b.nodes(&st.lights)
                    && b.nodes(&st.renderers)
                    && b.nodes(&st.working_nodes)
                    && b.nodes(&st.broken_nodes)
                    && st.intensity.is_finite()
                    && [&st.on_material, &st.off_material]
                        .into_iter()
                        .flatten()
                        .all(|m| b.material(m))
            });
            let cabin_valid = light.cabin.as_ref().is_none_or(|c| {
                b.node(c.switch_node) && b.nodes(&c.door_slots) && c.threshold.is_finite()
            });
            check(
                s,
                &format!("vehicle light {i}"),
                b.node(light.node)
                    && light.electronics.is_none_or(|n| b.node(n))
                    && finite(&[light.energy, light.energy_move])
                    && states_valid
                    && cabin_valid,
            )?;
        }
        if let Some(sw) = &self.light_switch {
            check(
                s,
                "vehicle light switch",
                b.node(sw.usable)
                    && b.nodes(&sw.nodes)
                    && sw.state_nodes.iter().all(|n| b.nodes(n)),
            )?;
        }
        let lamps = self.instruments.as_ref().map_or(0, |r| r.lamps.len());
        check(
            s,
            "vehicle high beams",
            self.high_beams.iter().all(|i| (*i as usize) < lamps),
        )?;
        if let Some(r) = &self.instruments {
            check(s, "vehicle instruments", instruments_valid(r, b))?;
        }
        let l = &self.gauge_limits;
        check(
            s,
            "vehicle gauge limits",
            finite(&[
                l.electronics_output,
                l.electronics_off_threshold,
                l.low_fuel,
                l.low_oil,
                l.oil_test_rpm,
                l.battery_test_rpm,
            ]),
        )?;
        check(
            s,
            "vehicle kmh meters",
            self.kmh_meters.iter().all(|k| k.modifier.is_finite()),
        )?;
        for p in &self.exhaust {
            super::particles::validate_particle(p)?;
        }
        Ok(())
    }
}

fn instruments_valid(r: &InstrumentReference, b: PrototypeBounds) -> bool {
    // Gauge state keeps lamp bits in one u32.
    r.lamps.len() <= 32
        && r.all_backlights().all(|m| switch_valid(m, &b))
        && r.needles.iter().all(|n| {
            b.node(n.node.0)
                && n.axis < 3
                && finite(&[
                    n.min_value,
                    n.max_value,
                    n.min_angle,
                    n.max_angle,
                    n.offset,
                    n.value_smoothing,
                    n.signal_smoothing,
                ])
                && finite(&n.rest)
        })
        && r.lamps.iter().all(|l| {
            b.node(l.node.0)
                && b.material(&l.on)
                && b.material(&l.off)
                && finite(&[l.min_value, l.max_value])
        })
        && r.digits().iter().all(|d| {
            b.node(d.node.0)
                && d.axis < 3
                && b.material(&d.on)
                && b.material(&d.off)
                && finite(&[d.initial_value, d.initial_angle])
        })
        && r
            .odometer
            .as_ref()
            .is_none_or(|o| finite(&[o.initial_value, o.multiplier]))
}

fn switch_valid(m: &MaterialSwitchReference, b: &PrototypeBounds) -> bool {
    match m {
        MaterialSwitchReference::Absent => true,
        MaterialSwitchReference::Renderer { node, on, off } => {
            b.node(node.0) && b.material(on) && b.material(off)
        }
    }
}

impl GaugePartReference {
    pub(super) fn validate(&self, b: PrototypeBounds) -> Result<(), ReferenceError> {
        check(
            &self.source,
            "gauge part",
            b.node(self.node)
                && self.lamps.len() <= 32
                && std::iter::once(&self.backlight)
                    .chain(self.needles.iter().map(|n| &n.backlight))
                    .all(|m| switch_valid(m, &b))
                && self.needles.iter().all(|n| {
                    b.node(n.node.0)
                        && n.axis < 3
                        && finite(&[n.min_value, n.max_value, n.min_angle, n.max_angle])
                        && finite(&[n.offset, n.value_smoothing, n.signal_smoothing])
                        && finite(&n.rest)
                })
                && self.lamps.iter().all(|l| {
                    b.node(l.node.0) && b.material(&l.on) && b.material(&l.off)
                })
                && self.counters.iter().all(|c| {
                    finite(&[c.initial_value, c.multiplier])
                        && c.digits.iter().all(|d| {
                            b.node(d.node.0)
                                && d.axis < 3
                                && b.material(&d.on)
                                && b.material(&d.off)
                        })
                }),
        )
    }
}

impl EnginePartReference {
    pub(super) fn validate(&self, nodes: usize) -> Result<(), ReferenceError> {
        let s = &self.source;
        check(
            s,
            "engine part links",
            (self.node as usize) < nodes && self.oil_tank.is_none_or(|n| (n as usize) < nodes),
        )?;
        check(
            s,
            "engine part values",
            finite(&[
                self.start_chance,
                self.starter_rpm,
                self.starter_rpm_lerp,
                self.idle_rpm,
                self.max_rpm,
                self.shutdown_rpm,
                self.max_torque_nm,
                self.starting_push_force,
                self.consumption_modifier,
                self.oil_consumption_modifier,
                self.engine_brake_modifier,
                self.rpm_change_modifier,
                self.engine_start_time_modifier,
                self.heat_gain_min,
                self.heat_gain_max,
                self.heat_loss,
                self.initial_temperature,
                self.heat_loss_speed_modifier,
                self.max_rpm_difference,
                self.torque_modifier,
                self.start_chance_interval_seconds,
                self.auto_shift_clutch_rpm,
                self.auto_shift_clutch_rpm_range,
                self.smoke_rate,
            ]) && finite(&self.optimal_temperature)
                && finite(&self.fuel_consumption)
                && (0.0..=1.0).contains(&self.start_chance)
                && self.shutdown_rpm > 0.
                && self.max_rpm > self.idle_rpm
                && self.max_torque_nm > 0.
                && self.max_rpm_difference > 0.
                && self.start_chance_interval_seconds > 0.
                && self.smoke_rate >= 0.
                && self.oil_fluid < 6,
        )?;
        check(s, "engine torque curve", curve_valid(&self.torque_curve))
    }
}

impl RadiatorPartReference {
    pub(super) fn validate(&self, nodes: usize) -> Result<(), ReferenceError> {
        let s = &self.source;
        check(
            s,
            "radiator part links",
            [Some(self.node), self.coolant_tank, self.fan_node]
                .into_iter()
                .flatten()
                .all(|n| (n as usize) < nodes),
        )?;
        check(
            s,
            "radiator part values",
            finite(&self.thermostat_range)
                && finite(&[
                    self.fan_temperature_on,
                    self.fan_temperature_off,
                    self.max_value,
                    self.max_fan_value,
                    self.max_fan_speed,
                    self.max_fluid_loss,
                ]),
        )
    }
}

impl EngineGlobals {
    /// Shared `mainscript` engine values. The world record holds them once.
    pub(super) fn validate(&self) -> Result<(), ReferenceError> {
        check(
            &self.source,
            "vehicle globals",
            self.source.file == "level1"
                && self.source.path_id == 64919
                && finite(&[
                    self.fuel_consumption,
                    self.oil_consumption,
                    self.rpm_change,
                    self.coolant_evaporate_rate,
                    self.temperature_difference_modifier,
                    self.temperature_move_modifier,
                    self.max_car_velocity,
                    self.outside_temperature_modifier,
                    self.min_steam_temperature,
                ])
                && finite(&self.car_temperature_bounds)
                && self.temperature_difference_modifier > 0.
                && self.max_car_velocity > 0.,
        )
    }
}

/// A small engine part record for native tests: petrol, 120 Nm, oil fluid 3.
#[cfg(test)]
pub(crate) fn engine_part_fixture() -> serde_json::Value {
    serde_json::json!({"source": {"file": "a", "path_id": 14}, "node": 3, "oil_tank": 1, "start_chance": 0.9, "starter_rpm": 300.0, "starter_rpm_lerp": 2.0, "idle_rpm": 800.0, "max_rpm": 6000.0, "shutdown_rpm": 200.0, "max_torque_nm": 120.0, "starting_push_force": 1080.0, "consumption_modifier": 1.0, "oil_consumption_modifier": 1.0, "engine_brake_modifier": 1.0, "rpm_change_modifier": 1.0, "engine_start_time_modifier": 1.0, "heat_gain_min": 1.0, "heat_gain_max": 2.0, "heat_loss": 1.0, "initial_temperature": 20.0, "heat_loss_speed_modifier": 1.0, "optimal_temperature": [80.0, 90.0, 70.0, 100.0], "fuel_consumption": [0.0, 0.0, 1.0, 0.0, 0.0, 0.0], "two_stroke": false,
                "torque_curve": {"keys": [{"time": 0.0, "value": 0.5, "in_tangent": 0.0, "out_tangent": 0.0, "in_weight": 0.0, "out_weight": 0.0, "weighted_mode": 0}, {"time": 1.0, "value": 1.0, "in_tangent": 0.0, "out_tangent": 0.0, "in_weight": 0.0, "out_weight": 0.0, "weighted_mode": 0}], "pre_wrap_mode": 2, "post_wrap_mode": 2},
                "max_rpm_difference": 100.0, "torque_modifier": 1.0, "oil_fluid": 3, "no_overheat": false, "kickstart": false, "start_chance_interval_seconds": 1.0, "auto_shift_clutch_rpm": 1000.0, "auto_shift_clutch_rpm_range": 200.0, "smoke_rate": 5.0})
}

/// Gauge limits for native tests: low fuel 5 L, low oil 1 L, full electronics.
#[cfg(test)]
pub(crate) fn gauge_limits_fixture() -> serde_json::Value {
    serde_json::json!({"electronics_output": 1.0, "electronics_off_threshold": 0.2, "low_fuel": 5.0,
        "low_oil": 1.0, "oil_test_rpm": 500.0, "battery_test_rpm": 500.0})
}
