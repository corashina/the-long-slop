//! Manifest loading and validation.

use super::*;

#[derive(Debug)]
pub struct ReferenceError(pub String);
impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ReferenceError {}

pub(crate) fn require(ok: bool, field: &str) -> Result<(), ReferenceError> {
    if ok {
        Ok(())
    } else {
        Err(ReferenceError(format!("Invalid reference field: {field}")))
    }
}
pub(crate) fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}
fn transform(t: &Transform) -> bool {
    finite(&t.translation)
        && finite(&t.rotation)
        && finite(&t.scale)
        && t.scale.iter().all(|x| *x != 0.0)
        && (t.rotation.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 0.002
}
pub(crate) fn ids<T: Eq + std::hash::Hash>(
    values: impl Iterator<Item = T>,
    field: &str,
) -> Result<HashSet<T>, ReferenceError> {
    let mut result = HashSet::new();
    for value in values {
        require(result.insert(value), field)?;
    }
    Ok(result)
}
pub(crate) fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

impl ReferenceManifest {
    pub fn from_json(bytes: &[u8]) -> Result<Self, ReferenceError> {
        require(bytes.len() <= 64 * 1024 * 1024, "manifest size")?;
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| ReferenceError(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ReferenceError> {
        require(
            self.app_id == APP_ID && self.build_id == BUILD_ID,
            "app/build",
        )?;
        let asset_ids = ids(self.assets.iter().map(|r| r.id), "duplicate asset ID")?;
        let node_ids = ids(self.nodes.iter().map(|r| r.id), "duplicate node ID")?;
        self.ui.validate()?;
        self.instruments.validate(&node_ids, &self.assets)?;
        let body_ids = ids(self.bodies.iter().map(|r| r.id), "duplicate body ID")?;
        let collider_ids = ids(self.colliders.iter().map(|r| r.id), "duplicate collider ID")?;
        let anchor_ids = ids(self.anchors.iter().map(|r| r.id), "duplicate anchor ID")?;
        let item_ids = ids(self.items.iter().map(|r| r.id), "duplicate item ID")?;
        let opening_ids = ids(self.openings.iter().map(|r| r.id), "duplicate opening ID")?;
        ids(self.seats.iter().map(|r| r.id), "duplicate seat ID")?;
        let wheel_ids = ids(self.wheel_slots.iter().map(|r| r.id), "duplicate wheel ID")?;
        ids(
            self.wheel_slots.iter().map(|r| r.rim),
            "duplicate mounted rim",
        )?;
        ids(
            self.wheel_slots.iter().map(|r| r.tire),
            "duplicate mounted tire",
        )?;
        ids(
            self.wheel_slots.iter().map(|r| r.wheel),
            "duplicate mounted wheel",
        )?;
        require(
            self.assets
                .iter()
                .enumerate()
                .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .nodes
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .bodies
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .colliders
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .anchors
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .items
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .seats
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u32 + 1)
                && self
                    .wheel_slots
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id.0 == i as u8),
            "dense runtime IDs",
        )?;
        ids(
            self.nodes.iter().map(|r| &r.source),
            "duplicate node source",
        )?;
        require(
            !self.assets.is_empty() && !self.source_hashes.is_empty(),
            "assets/source hashes",
        )?;
        for r in &self.source_hashes {
            require(!r.file.is_empty() && hash(&r.sha256), "source hash")?;
        }
        for r in &self.assets {
            require(
                r.path.starts_with("reference/") && !r.path.contains("..") && hash(&r.sha256),
                "asset path/hash",
            )?;
            let bindings = ids(
                self.nodes
                    .iter()
                    .filter(|n| n.asset == r.id)
                    .map(|n| n.gltf_node),
                "duplicate glTF node binding",
            )?;
            require(
                (0..bindings.len() as u32).all(|id| bindings.contains(&id)),
                "glTF node bindings",
            )?;
            ids(
                r.dependencies.iter().map(|dependency| &dependency.file),
                "duplicate asset dependency",
            )?;
            for dependency in &r.dependencies {
                require(
                    dependency.file.starts_with("reference/")
                        && !dependency.file.contains("..")
                        && hash(&dependency.sha256),
                    "asset dependency hash/path",
                )?;
            }
        }
        require(
            self.nodes.len() <= 100_000 && self.colliders.len() <= 100_000,
            "record count",
        )?;
        let node_map: HashMap<_, _> = self.nodes.iter().map(|r| (r.id, r)).collect();
        for r in &self.nodes {
            require(
                asset_ids.contains(&r.asset)
                    && r.parent
                        .is_none_or(|id| node_ids.contains(&id) && id != r.id),
                "node binding",
            )?;
            require(transform(&r.transform), "node transform")?;
            let mut parent = r.parent;
            for _ in 0..64 {
                if parent.is_none() {
                    break;
                }
                require(parent != Some(r.id), "node cycle")?;
                parent = parent
                    .and_then(|id| node_map.get(&id))
                    .and_then(|n| n.parent);
            }
            require(parent.is_none(), "node cycle")?;
        }
        for r in &self.bodies {
            require(
                r.parent_body
                    .is_none_or(|id| body_ids.contains(&id) && id != r.id),
                "body parent binding",
            )?;
            require(
                node_ids.contains(&r.node) && transform(&r.pose),
                "body binding/pose",
            )?;
            require(
                r.mass_kg.is_finite() && r.mass_kg >= 0.0 && (!r.dynamic || r.mass_kg > 0.0),
                "body mass",
            )?;
            require(
                finite(&[r.linear_damping, r.angular_damping])
                    && r.linear_damping >= 0.0
                    && r.angular_damping >= 0.0,
                "body damping",
            )?;
            require(r.pose.scale == [1.0; 3], "body scale")?;
            let mut parent = r.parent_body;
            for _ in 0..64 {
                if parent.is_none() {
                    break;
                }
                require(parent != Some(r.id), "body cycle")?;
                parent = self
                    .bodies
                    .iter()
                    .find(|body| Some(body.id) == parent)
                    .and_then(|body| body.parent_body);
            }
            require(parent.is_none(), "body cycle/depth")?;
        }
        for r in &self.colliders {
            require(
                body_ids.contains(&r.body) && node_ids.contains(&r.node),
                "collider binding",
            )?;
            require(
                r.friction.is_finite()
                    && r.friction >= 0.0
                    && r.restitution.is_finite()
                    && (0.0..=1.0).contains(&r.restitution),
                "collider material",
            )?;
            let valid = match &r.geometry {
                ColliderGeometry::Box {
                    center,
                    half_extents,
                    rotation,
                } => {
                    finite(center)
                        && finite(rotation)
                        && finite(half_extents)
                        && half_extents.iter().all(|x| *x > 0.0)
                }
                ColliderGeometry::Sphere { center, radius } => {
                    finite(center) && radius.is_finite() && *radius > 0.0
                }
                ColliderGeometry::Capsule { a, b, radius } => {
                    finite(a) && finite(b) && radius.is_finite() && *radius > 0.0
                }
                ColliderGeometry::Convex { points } => {
                    points.len() >= 4
                        && points.len() <= 1_000_000
                        && points.iter().all(|p| finite(p))
                }
                ColliderGeometry::Trimesh { points, triangles } => {
                    !points.is_empty()
                        && points.len() <= 1_000_000
                        && !triangles.is_empty()
                        && triangles.len() <= 2_000_000
                        && points.iter().all(|p| finite(p))
                        && triangles
                            .iter()
                            .flatten()
                            .all(|i| (*i as usize) < points.len())
                }
            };
            require(valid, "collider geometry")?;
        }
        for r in &self.anchors {
            require(
                node_ids.contains(&r.node) && body_ids.contains(&r.body) && transform(&r.transform),
                "anchor binding",
            )?;
        }
        for r in &self.items {
            require(
                node_ids.contains(&r.node)
                    && body_ids.contains(&r.body)
                    && r.attached_to
                        .is_none_or(|id| item_ids.contains(&id) && id != r.id),
                "item binding",
            )?;
            require(
                r.condition <= 4
                    && r.fall_off_power
                        .is_none_or(|power| power.is_finite() && power >= 0.0)
                    && r.mass_kg.is_finite()
                    && r.mass_kg >= 0.0,
                "item condition/mass",
            )?;
        }
        for r in &self.hinges {
            require(
                item_ids.contains(&r.item)
                    && anchor_ids.contains(&r.anchor)
                    && body_ids.contains(&r.body),
                "hinge binding",
            )?;
            require(
                finite(&r.axis)
                    && finite(&[
                        r.closed_degrees,
                        r.min_degrees,
                        r.max_degrees,
                        r.drag_sensitivity,
                    ])
                    && r.min_degrees <= r.max_degrees
                    && r.drag_sensitivity > 0.0
                    && r.drag_input_axis < 2
                    && finite(&r.drag_axis)
                    && r.rotation_axes.iter().all(|axis| finite(axis))
                    && anchor_ids.contains(&r.drag_anchor)
                    && r.motor_degrees_per_second
                        .is_none_or(|speed| speed.is_finite() && speed > 0.0),
                "hinge parameters",
            )?;
        }
        for r in &self.openings {
            let visuals = &r.visuals;
            require(
                r.valve.is_finite()
                    && (0.0..=1.0).contains(&r.valve)
                    && r.tune_rate.is_finite()
                    && r.tune_rate >= 0.0
                    && (!r.can_tune || r.tune_rate > 0.0),
                "opening valve/tuning",
            )?;
            require(
                visuals
                    .open_nodes
                    .iter()
                    .chain(&visuals.closed_nodes)
                    .all(|id| node_ids.contains(id))
                    && [visuals.cap_node, visuals.closed_node, visuals.open_node]
                        .iter()
                        .all(|id| id.is_none_or(|id| node_ids.contains(&id)))
                    && finite(&visuals.closed_euler_degrees)
                    && finite(&visuals.open_euler_degrees),
                "cap visual bindings",
            )?;
            require(
                visuals.closed_node.is_some() == visuals.open_node.is_some()
                    && (visuals.cap_node.is_some()
                        || (visuals.closed_node.is_none() && !visuals.euler_rotation)),
                "cap visual control",
            )?;
            require(
                item_ids.contains(&r.tank)
                    && item_ids.contains(&r.cap)
                    && anchor_ids.contains(&r.anchor),
                "opening binding",
            )?;
            require(
                finite(&[r.flow_liters_per_second, r.tilt_degrees])
                    && r.flow_liters_per_second > 0.0
                    && finite(&r.tilt_axis)
                    && r.flow_radius.is_finite()
                    && r.flow_radius > 0.0,
                "opening flow",
            )?;
        }
        ids(self.tanks.iter().map(|r| r.item), "duplicate tank")?;
        for r in &self.tanks {
            require(
                item_ids.contains(&r.item) && r.openings.iter().all(|id| opening_ids.contains(id)),
                "tank binding",
            )?;
            require(
                r.capacity_liters.is_finite()
                    && r.capacity_liters > 0.0
                    && finite(&r.quantities_liters)
                    && r.quantities_liters.iter().all(|v| *v >= 0.0)
                    && r.quantities_liters.iter().sum::<f32>() <= r.capacity_liters + 0.001,
                "tank quantities",
            )?;
        }
        let e = &self.engine;
        require(
            item_ids.contains(&e.item)
                && [e.fuel_tank, e.oil_tank, e.coolant_tank]
                    .iter()
                    .all(|id| self.tanks.iter().any(|t| t.item == *id)),
            "engine tank bindings",
        )?;
        require(
            finite(&[
                e.start_chance,
                e.starter_delay_seconds,
                e.starter_rpm,
                e.starter_rpm_lerp,
                e.idle_rpm,
                e.max_rpm,
                e.shutdown_rpm,
                e.max_torque_nm,
                e.starting_push_force,
                e.consumption_modifier,
                e.oil_consumption_modifier,
                e.engine_brake_modifier,
                e.rpm_change_modifier,
                e.engine_start_time_modifier,
                e.heat_gain_min,
                e.heat_gain_max,
                e.heat_loss,
            ]) && finite(&e.optimal_temperature)
                && finite(&e.fuel_consumption),
            "engine parameters",
        )?;
        require(
            (0.0..=1.0).contains(&e.start_chance)
                && e.shutdown_rpm > 0.0
                && e.max_rpm > e.idle_rpm
                && e.max_torque_nm > 0.0,
            "engine limits",
        )?;
        require(
            e.torque_curve.keys.len() >= 2 && e.torque_curve.keys.len() <= 1024,
            "torque curve",
        )?;
        for k in &e.torque_curve.keys {
            require(
                finite(&[
                    k.time,
                    k.value,
                    k.in_tangent,
                    k.out_tangent,
                    k.in_weight,
                    k.out_weight,
                ]) && k.weighted_mode <= 3,
                "curve key",
            )?;
        }
        require(
            e.torque_curve
                .keys
                .windows(2)
                .all(|k| k[0].time < k[1].time),
            "curve key order",
        )?;
        let p = &self.player;
        require(p.camera_culling_mask != 0, "camera culling mask")?;
        require(
            p.scroll_sensitivity.is_finite() && p.scroll_sensitivity > 0.0,
            "scroll sensitivity",
        )?;
        require(
            finite(&p.carry_anchor_offset)
                && finite(&p.look_sensitivity)
                && finite(&[
                    p.carry_offset_radius,
                    p.carry_drop_distance,
                    p.carry_force_multiplier,
                    p.carry_max_force,
                    p.carry_min_distance,
                    p.carry_max_distance,
                    p.obj_move_sensitivity,
                    p.obj_rotation_sensitivity,
                ])
                && p.carry_offset_radius > 0.0
                && p.carry_drop_distance > 0.0
                && p.carry_min_distance >= 0.0
                && p.carry_max_distance >= p.carry_min_distance,
            "carry controls",
        )?;
        require(
            finite(&[
                e.auto_shift_clutch_rpm,
                e.auto_shift_clutch_rpm_range,
                self.vehicle.auto_shift_stop_speed,
                self.vehicle.auto_shift_min_rpm,
                self.vehicle.auto_shift_max_rpm,
            ]) && self.vehicle.auto_shift_min_rpm < self.vehicle.auto_shift_max_rpm,
            "automatic shift thresholds",
        )?;
        require(
            e.oil_fluid < 6
                && finite(&[
                    e.max_rpm_difference,
                    e.torque_modifier,
                    e.start_chance_interval_seconds,
                    e.globals.min_steam_temperature,
                ])
                && e.max_rpm_difference > 0.0
                && e.start_chance_interval_seconds > 0.0
                && e.globals.temperature_difference_modifier > 0.0
                && e.globals.max_car_velocity > 0.0,
            "engine scalar bounds",
        )?;
        require(
            finite(&[
                p.standing_radius,
                p.standing_height,
                p.crouch_radius,
                p.exit_distance,
                p.exit_step_vertical,
                p.exit_step_horizontal,
            ]) && p.standing_radius > 0.0
                && p.standing_height >= p.standing_radius * 2.0
                && p.crouch_radius > 0.0
                && p.crouch_height >= p.crouch_radius * 2.0
                && p.exit_distance > 0.0
                && p.exit_steps_vertical <= 100
                && p.exit_steps_horizontal <= 100,
            "effective player geometry/exit scan",
        )?;
        require(
            finite(&[
                e.initial_temperature,
                e.heat_loss_speed_modifier,
                e.globals.fuel_consumption,
                e.globals.oil_consumption,
                e.globals.rpm_change,
                e.globals.coolant_evaporate_rate,
                e.globals.temperature_difference_modifier,
                e.globals.temperature_move_modifier,
                e.globals.max_car_velocity,
                e.globals.awake_heat_modifier,
            ]) && finite(&e.globals.car_temperature_bounds),
            "engine globals",
        )?;
        require(
            finite(&e.radiator.thermostat_range)
                && finite(&[
                    e.radiator.fan_temperature_on,
                    e.radiator.fan_temperature_off,
                    e.radiator.max_value,
                    e.radiator.max_fan_value,
                    e.radiator.max_fan_speed,
                    e.radiator.max_fluid_loss,
                ]),
            "radiator parameters",
        )?;
        require(
            finite(&[
                self.fluid_rules.pour_speed,
                self.fluid_rules.slow_oil_modifier,
                self.fluid_rules.fluid_tune_speed,
            ]) && self.fluid_rules.pour_speed > 0.0,
            "fluid rules",
        )?;
        require(
            finite(&[
                p.mass_kg,
                p.duck_multiplier,
                p.ctrl_duck_value,
                p.duck_lerp,
                p.head_lerp,
            ]) && p.mass_kg > 0.0
                && (0.0..=1.0).contains(&p.ctrl_duck_value),
            "player movement",
        )?;
        require(
            finite(&[
                p.walk_speed,
                p.run_multiplier,
                p.jump_force,
                p.crouch_height,
                p.capsule_radius,
                p.capsule_height,
                p.camera_height,
                p.field_of_view_degrees,
                p.aim_reach,
                p.exit_capsule_half_height,
                p.exit_capsule_radius,
            ]) && p.walk_speed > 0.0
                && p.capsule_radius > 0.0
                && p.capsule_height >= p.capsule_radius * 2.0
                && p.crouch_height >= p.capsule_radius * 2.0
                && p.crouch_height < p.capsule_height
                && p.aim_reach > 0.0
                && p.use_layer_mask != 0
                && p.get_out_layer_mask != 0
                && p.exit_capsule_half_height > 0.0
                && p.exit_capsule_radius > 0.0,
            "player parameters",
        )?;
        let mut seat_hit_nodes = HashSet::new();
        for r in &self.seats {
            require(
                item_ids.contains(&r.item)
                    && anchor_ids.contains(&r.anchor)
                    && !r.hit_nodes.is_empty()
                    && r.hit_nodes.iter().all(|node| {
                        node_ids.contains(node)
                            && seat_hit_nodes.insert(*node)
                            && self
                                .nodes
                                .iter()
                                .any(|record| record.id == *node && record.active)
                    })
                    && finite(&r.yaw_limits_degrees)
                    && r.yaw_limits_degrees[0] <= r.yaw_limits_degrees[1]
                    && r.exit_distance.is_finite()
                    && r.exit_distance > 0.0,
                "seat binding/limits",
            )?;
        }
        require(
            self.seats.iter().filter(|s| s.driver).count() == 1,
            "driver seat",
        )?;
        for r in &self.wheel_slots {
            require(
                anchor_ids.contains(&r.anchor)
                    && [r.wheel, r.tire, r.rim]
                        .iter()
                        .all(|id| item_ids.contains(id)),
                "wheel bindings",
            )?;
            require(
                finite(&[
                    r.radius,
                    r.suspension_distance,
                    r.spring,
                    r.damper,
                    r.target_position,
                ]) && finite(&r.center)
                    && finite(&r.forward_friction)
                    && finite(&r.sideways_friction)
                    && r.radius > 0.0
                    && r.suspension_distance > 0.0,
                "wheel parameters",
            )?;
        }
        ids(
            self.wheel_parts.iter().map(|part| part.item),
            "duplicate wheel part",
        )?;
        for part in &self.wheel_parts {
            require(
                item_ids.contains(&part.item)
                    && !part.colliders.is_empty()
                    && part.colliders.iter().all(|id| collider_ids.contains(id))
                    && part
                        .tire_slot_anchor
                        .is_none_or(|id| anchor_ids.contains(&id)),
                "wheel part bindings",
            )?;
            require(
                part.radius.is_finite()
                    && part.radius > 0.0
                    && finite(&part.local_rotation_after_mount)
                    && part.tire_mount.as_ref().is_none_or(transform)
                    && part.main_types.len() <= 16
                    && part.part_types.len() <= 16
                    && !part.part_types.is_empty(),
                "wheel part geometry/fit",
            )?;
        }
        ids(self.parts.iter().map(|part| part.item), "duplicate part")?;
        ids(
            self.headlights.iter().map(|light| light.item),
            "duplicate headlight",
        )?;
        for light in &self.headlights {
            require(
                item_ids.contains(&light.item)
                    && node_ids.contains(&light.lens_node)
                    && !light.states.is_empty()
                    && light.energy_rate.is_finite()
                    && light.energy_rate > 0.0
                    && light.states.iter().all(|state| {
                        state.intensity.is_finite()
                            && state.intensity >= 0.0
                            && state.lens_emission.as_ref().is_none_or(|c| finite(c))
                            && state.light.as_ref().is_none_or(|spot| {
                                node_ids.contains(&spot.node)
                                    && finite(&spot.color)
                                    && spot.range > 0.0
                                    && spot.spot_degrees > 0.0
                                    && spot.spot_degrees < 180.0
                            })
                    }),
                "headlight bindings",
            )?;
        }
        let d = &self.day_cycle;
        require(
            [
                d.day_seconds,
                d.night_seconds,
                d.dawn_seconds,
                d.dusk_seconds,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0)
                && d.dawn_seconds + d.dusk_seconds < d.day_seconds
                && (0.0..d.day_seconds).contains(&d.start_seconds)
                && finite(&[
                    d.day_intensity,
                    d.noon_intensity,
                    d.night_intensity,
                    d.day_ambient,
                    d.night_ambient,
                ])
                && finite(&d.day_angles)
                && finite(&d.midnight_angles)
                && finite(&d.moon_angles)
                && !d.sun_colors.is_empty()
                && d.sun_colors
                    .windows(2)
                    .all(|keys| keys[0].time <= keys[1].time)
                && d.sun_colors
                    .iter()
                    .all(|key| key.time.is_finite() && finite(&key.color))
                && d.day_fog_colors
                    .iter()
                    .chain(&d.night_fog_colors)
                    .all(|c| finite(c)),
            "day cycle",
        )?;
        for part in &self.parts {
            require(
                self.items
                    .iter()
                    .any(|item| item.id == part.item && item.attached_to.is_some())
                    && part.detach_node.is_none_or(|id| node_ids.contains(&id))
                    && finite(&part.local_rotation_after_mount),
                "part slot bindings",
            )?;
        }
        require(
            self.wheel_slots.iter().all(|slot| {
                self.wheel_parts.iter().any(|part| part.item == slot.rim)
                    && self.wheel_parts.iter().any(|part| part.item == slot.tire)
                    && anchor_ids.contains(&slot.mount_anchor)
            }),
            "wheel mount bindings",
        )?;
        let v = &self.vehicle;
        require(
            body_ids.contains(&v.body)
                && node_ids.contains(&v.root)
                && v.wheel_slots.iter().all(|id| wheel_ids.contains(id))
                && v.colliders.iter().all(|id| collider_ids.contains(id))
                && item_ids.contains(&v.item),
            "vehicle bindings",
        )?;
        let c = &v.crash;
        require(
            finite(&v.wheel_slip)
                && finite(&v.rim_slip)
                && finite(&[c.max_fall_off_speed, c.multiplier, c.fall_off_multiplier])
                && finite(&c.fall_off_speed)
                && c.fall_off_speed[0] <= c.fall_off_speed[1]
                && c.fall_off_speed[0] < c.max_fall_off_speed
                && c.fall_off_counts.iter().all(|[low, high]| low <= high),
            "crash and slip parameters",
        )?;
        require(
            wheel_ids.len() == 4
                && v.wheel_slots.iter().collect::<HashSet<_>>().len() == 4
                && !v.colliders.is_empty(),
            "four road wheels/colliders",
        )?;
        require(
            finite(&[
                v.mass_kg,
                v.serialized_mass_kg,
                v.final_drive_ratio,
                v.max_steer_degrees,
                v.brake_torque_nm,
                v.handbrake_torque_nm,
            ]) && finite(&v.gear_ratios)
                && v.gear_ratios.contains(&0.0)
                && v.gear_free_run.len() == v.gear_ratios.len()
                && v.mass_kg > 0.0,
            "vehicle parameters",
        )?;
        let a = &self.area;
        require(
            node_ids.contains(&a.home_root)
                && transform(&a.home_pose)
                && transform(&a.player_spawn)
                && transform(&a.car_pose),
            "area binding/poses",
        )?;
        for b in [&a.home_bounds, &a.driveway_bounds] {
            require(
                finite(&b.min) && finite(&b.max) && (0..3).all(|i| b.min[i] <= b.max[i]),
                "area bounds",
            )?;
        }
        require(a.driveway_height.is_finite(), "driveway height")?;
        self.world.validate()?;
        self.audio
            .validate(&item_ids, &collider_ids, &opening_ids)?;
        Ok(())
    }
}
