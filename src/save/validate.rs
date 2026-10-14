use super::*;
use crate::reference::{self as r, ReferenceManifest};
use std::collections::{HashMap, HashSet};

impl SaveSnapshot {
    pub fn validate(
        &self,
        reference: &ReferenceManifest,
        expected: &SourceStamp,
    ) -> Result<(), SaveError> {
        if self.schema_version != SAVE_SCHEMA_VERSION {
            return Err(SaveError::IncompatibleSources("Schema version".into()));
        }
        data(
            !self.driving_assists.automatic_gears || self.driving_assists.automatic_clutch,
            "Driving assist dependency",
        )?;
        validate_source(&self.source, expected, reference)?;
        data(timestamp(&self.saved_at), "Save timestamp")?;
        let ids = IdentityMap::new(reference)?;
        self.validate_world(reference)?;
        identity::unsigned(&self.progress.start_attempts)?;
        self.validate_vehicle(reference, &ids)?;
        let mut items = HashMap::new();
        for item in &self.items {
            let id = ids.item(&item.instance)?;
            links(items.insert(id, item).is_none(), "Duplicate item")?;
            absolute(&item.pose)?;
            local(&item.local_pose)?;
            data(unit(item.condition), "Item condition")?;
            data(
                item.temperature.is_none_or(f32::is_finite),
                "Item temperature",
            )?;
            links(
                item.temperature.is_some()
                    == reference.vehicles.iter().any(|v| id == v.engine.item),
                "Item temperature owner",
            )?;
            let tank = reference.tanks.iter().find(|tank| tank.item == id);
            match (&item.fluids, tank) {
                (Some(quantities), Some(tank)) => data(
                    quantities.iter().all(|v| v.is_finite() && *v >= 0.)
                        && quantities.iter().map(|v| f64::from(*v)).sum::<f64>()
                            <= f64::from(tank.capacity_liters) + 0.001,
                    "Fluid capacity",
                )?,
                (None, None) => (),
                _ => {
                    return Err(SaveError::InvalidLinks(
                        "Missing or unexpected fluid container".into(),
                    ));
                }
            }
        }
        links(items.len() == reference.items.len(), "Missing source items")?;
        let parents = self.validate_attachments(reference, &ids, &items)?;
        self.validate_player(reference, &ids, &items, &parents)?;
        self.validate_bodies(reference, &ids, &items)?;
        self.validate_mechanisms(reference, &ids)?;
        self.validate_lighting(reference, &ids)?;
        Ok(())
    }

    fn validate_vehicle(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
    ) -> Result<(), SaveError> {
        links(
            self.vehicles.len() == reference.vehicles.len(),
            "Missing vehicle state",
        )?;
        links(
            ids.vehicles.contains(&self.progress.selected_vehicle),
            "Selected vehicle identity",
        )?;
        let mut seen = HashSet::new();
        for vehicle in &self.vehicles {
            let index = ids
                .vehicles
                .iter()
                .position(|id| id == &vehicle.instance)
                .ok_or_else(|| invalid("Vehicle identity"))?;
            links(seen.insert(index), "Duplicate vehicle")?;
            let source = &reference.vehicles[index];
            data(
                vehicle.trip_m.is_finite() && vehicle.trip_m >= 0.,
                "Trip distance",
            )?;
            let odometer = &source.instruments.odometer;
            let distance = odometer.initial_value + (vehicle.trip_m / 1000.) as f32;
            let displayed = if odometer.use_multiplier {
                distance * odometer.multiplier
            } else {
                distance
            };
            data(
                distance.is_finite() && displayed.is_finite(),
                "Instrument distance",
            )?;
            data(vehicle.requested_headlights <= 2, "Requested headlights")?;
            let BodyIdentity::Source { source } = &vehicle.body.identity else {
                return Err(SaveError::InvalidLinks("Vehicle body identity".into()));
            };
            links(
                ids.bodies.get(source) == Some(&reference.vehicles[index].body),
                "Vehicle body",
            )?;
            body(&vehicle.body)?;
            data(
                vehicle.gear < reference.vehicles[index].gear_ratios.len(),
                "Vehicle gear",
            )?;
            let e = &vehicle.engine;
            data(
                finite(&[
                    e.rpm,
                    e.temperature,
                    e.elapsed,
                    e.last_run_time,
                    e.last_start_chance_time,
                    e.random_start,
                    e.supplied_fuel,
                ]) && e.elapsed >= 0.
                    && e.last_run_time >= -1.
                    && e.last_run_time <= e.elapsed
                    && e.last_start_chance_time >= 0.
                    && e.last_start_chance_time <= e.elapsed
                    && unit(e.random_start)
                    && e.supplied_fuel >= 0.,
                "Engine state",
            )?;
        }
        Ok(())
    }

    fn validate_attachments<'a>(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
        items: &HashMap<r::ItemId, &'a ItemSnapshot>,
    ) -> Result<HashMap<r::ItemId, r::ItemId>, SaveError> {
        let vehicle_item = |index: usize| {
            reference
                .items
                .iter()
                .find(|item| item.node == reference.vehicles[index].root)
                .map(|i| i.id)
                .ok_or_else(|| invalid("Missing vehicle item"))
        };
        let mut parents = HashMap::new();
        let mut part_slots = HashMap::new();
        let mut wheel_slots = HashMap::new();
        let mut tires = HashMap::new();
        let mut linked_parts = HashSet::new();
        for attachment in &self.attachments.parts {
            let id = *ids
                .part_slots
                .get(&attachment.slot)
                .ok_or_else(|| invalid("Unknown part slot"))?;
            links(
                part_slots.insert(id, attachment.part.as_ref()).is_none(),
                "Duplicate part slot",
            )?;
            let slot = reference
                .part_slots
                .iter()
                .find(|slot| slot.id == id)
                .unwrap();
            if let Some(instance) = &attachment.part {
                let item = ids.item(instance)?;
                links(linked_parts.insert(item), "Part has multiple owners")?;
                let part = reference
                    .parts
                    .iter()
                    .find(|part| part.item == item)
                    .ok_or_else(|| invalid("Unknown part"))?;
                links(
                    fits(
                        &slot.main_types,
                        &slot.part_types,
                        &part.main_types,
                        &part.part_types,
                    ),
                    "Incompatible part slot",
                )?;
                links(
                    items[&item].location
                        == ItemLocationSave::PartSlot {
                            slot: attachment.slot.clone(),
                        },
                    "Part location conflicts with slot",
                )?;
                parents.insert(
                    item,
                    slot.owner.map(Ok).unwrap_or_else(|| {
                        let mut node = slot.node;
                        let index = loop {
                            if let Some(index) =
                                reference.vehicles.iter().position(|v| v.root == node)
                            {
                                break index;
                            }
                            node = reference
                                .nodes
                                .iter()
                                .find(|n| n.id == node)
                                .and_then(|n| n.parent)
                                .ok_or_else(|| invalid("Part slot vehicle"))?;
                        };
                        vehicle_item(index)
                    })?,
                );
            }
        }
        links(
            part_slots.len() == reference.part_slots.len(),
            "Missing part slot",
        )?;
        for attachment in &self.attachments.wheels {
            let id = *ids
                .wheel_slots
                .get(&attachment.slot)
                .ok_or_else(|| invalid("Unknown wheel slot"))?;
            links(
                wheel_slots.insert(id, attachment.rim.as_ref()).is_none(),
                "Duplicate wheel slot",
            )?;
            let delta =
                (f64::from(attachment.rotation) - f64::from(attachment.previous_rotation)).abs();
            let vehicle_index = reference
                .vehicles
                .iter()
                .position(|v| v.wheel_slots.contains(&id))
                .ok_or_else(|| invalid("Wheel slot vehicle"))?;
            let vehicle = &reference.vehicles[vehicle_index];
            let largest_ratio = vehicle
                .gear_ratios
                .iter()
                .fold(0_f64, |m, r| m.max(f64::from(r.abs())))
                * f64::from(vehicle.final_drive_ratio.abs());
            data(
                delta * 60. / f64::from(crate::simulation::FIXED_DT) <= f64::from(f32::MAX)
                    && f64::from(attachment.rpm.abs()) * largest_ratio <= f64::from(f32::MAX),
                "Wheel derived RPM",
            )?;
            data(attachment.rotation.is_finite(), "Wheel rotation")?;
            data(attachment.rpm.is_finite(), "Wheel RPM")?;
            data(
                attachment.previous_rotation.is_finite(),
                "Previous wheel rotation",
            )?;
            if let Some(instance) = &attachment.rim {
                let item = ids.item(instance)?;
                links(linked_parts.insert(item), "Rim has multiple owners")?;
                let slot = reference
                    .wheel_slots
                    .iter()
                    .find(|slot| slot.id == id)
                    .unwrap();
                let part = reference
                    .wheel_parts
                    .iter()
                    .find(|part| part.item == item)
                    .ok_or_else(|| invalid("Unknown rim"))?;
                links(
                    matches!(
                        reference.items.iter().find(|i| i.id == item).unwrap().kind,
                        r::ItemKind::Rim
                    ) && fits(
                        &slot.rim_main_types,
                        &slot.rim_types,
                        &part.main_types,
                        &part.part_types,
                    ),
                    "Incompatible wheel slot",
                )?;
                links(
                    items[&item].location
                        == ItemLocationSave::WheelSlot {
                            slot: attachment.slot.clone(),
                        },
                    "Rim location conflicts with slot",
                )?;
                parents.insert(item, vehicle_item(vehicle_index)?);
            }
        }
        links(
            wheel_slots.len() == reference.wheel_slots.len(),
            "Missing wheel slot",
        )?;
        for attachment in &self.attachments.tires {
            let rim = ids.item(&attachment.rim)?;
            let tire = ids.item(&attachment.tire)?;
            links(
                tires.insert(rim, tire).is_none() && linked_parts.insert(tire),
                "Duplicate tire ownership",
            )?;
            let rim_part = reference
                .wheel_parts
                .iter()
                .find(|p| p.item == rim)
                .ok_or_else(|| invalid("Unknown rim"))?;
            let tire_part = reference
                .wheel_parts
                .iter()
                .find(|p| p.item == tire)
                .ok_or_else(|| invalid("Unknown tire"))?;
            links(
                matches!(
                    reference.items.iter().find(|i| i.id == rim).unwrap().kind,
                    r::ItemKind::Rim
                ) && matches!(
                    reference.items.iter().find(|i| i.id == tire).unwrap().kind,
                    r::ItemKind::Tire
                ) && fits(
                    &rim_part.tire_main_types,
                    &rim_part.tire_types,
                    &tire_part.main_types,
                    &tire_part.part_types,
                ),
                "Incompatible tire and rim",
            )?;
            links(
                items[&tire].location
                    == ItemLocationSave::Attached {
                        owner: attachment.rim.clone(),
                    },
                "Tire location conflicts with rim",
            )?;
            parents.insert(tire, rim);
        }
        for (&id, item) in items {
            let source_item = reference.items.iter().find(|i| i.id == id).unwrap();
            let detachable = reference.parts.iter().any(|part| part.item == id)
                || reference.wheel_parts.iter().any(|part| part.item == id);
            if let Some(owner) = source_item.attached_to.filter(|_| !detachable) {
                links(
                    matches!(&item.location, ItemLocationSave::Attached { owner: saved_owner }
                        if ids.item(saved_owner).ok() == Some(owner)),
                    "Fixed source attachment changed",
                )?;
            }
            match &item.location {
                ItemLocationSave::PartSlot { slot } => links(
                    ids.part_slots
                        .get(slot)
                        .and_then(|slot| part_slots.get(slot))
                        .copied()
                        .flatten()
                        == Some(&item.instance),
                    "Unlisted part attachment",
                )?,
                ItemLocationSave::WheelSlot { slot } => links(
                    ids.wheel_slots
                        .get(slot)
                        .and_then(|slot| wheel_slots.get(slot))
                        .copied()
                        .flatten()
                        == Some(&item.instance),
                    "Unlisted wheel attachment",
                )?,
                ItemLocationSave::Attached { owner } => {
                    let owner = ids.item(owner)?;
                    if matches!(source_item.kind, r::ItemKind::Tire) {
                        links(tires.get(&owner) == Some(&id), "Unlisted tire attachment")?;
                    } else {
                        links(
                            source_item.attached_to == Some(owner),
                            "Invalid source attachment",
                        )?;
                    }
                    parents.insert(id, owner);
                }
                ItemLocationSave::GeneratedAttachment { .. }
                | ItemLocationSave::World
                | ItemLocationSave::Held => (),
            }
        }
        for &start in items.keys() {
            let mut seen = HashSet::new();
            let mut next = Some(start);
            while let Some(id) = next {
                links(seen.insert(id), "Attachment cycle")?;
                next = parents.get(&id).copied();
            }
        }
        Ok(parents)
    }

    fn validate_player(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
        items: &HashMap<r::ItemId, &ItemSnapshot>,
        parents: &HashMap<r::ItemId, r::ItemId>,
    ) -> Result<(), SaveError> {
        let p = &self.player;
        absolute(&p.pose)?;
        data(
            finite(&[p.yaw, p.pitch, p.vertical_velocity])
                && (-89.0..=89.0).contains(&p.pitch)
                && bounded_local(&p.recoil_velocity)
                && bounded_local(&[p.vertical_velocity]),
            "Player motion",
        )?;
        if let Some(seat) = &p.seat {
            local(&seat.local_pose)?;
            let vehicle_index = ids
                .vehicles
                .iter()
                .position(|id| id == &seat.vehicle)
                .ok_or_else(|| invalid("Seat vehicle"))?;
            let vehicle = self
                .vehicles
                .iter()
                .find(|v| v.instance == seat.vehicle)
                .ok_or_else(|| invalid("Seat vehicle"))?;
            links(
                seat.vehicle == self.progress.selected_vehicle,
                "Selected seat vehicle",
            )?;
            data(
                p.pose
                    .position
                    .iter()
                    .zip(vehicle.body.pose.position)
                    .all(|(a, b)| (*a - b).abs() <= f64::from(crate::world::REBASE_LIMIT)),
                "Seat relative position",
            )?;
            let seat_id = ids
                .seats
                .get(&seat.source)
                .ok_or_else(|| invalid("Unknown seat"))?;
            let source = reference.seats.iter().find(|s| s.id == *seat_id).unwrap();
            let mut root = source.item;
            while let Some(parent) = parents.get(&root) {
                root = *parent;
            }
            links(
                reference.items.iter().any(|item| {
                    item.id == root && item.node == reference.vehicles[vehicle_index].root
                }),
                "Seat is detached from vehicle",
            )?;
        }
        let held: Vec<_> = items
            .values()
            .filter(|item| item.location == ItemLocationSave::Held)
            .collect();
        match &p.carry {
            Some(carry) => {
                if let InstanceId::Generated { id } = &carry.item {
                    links(held.is_empty(), "Multiple held owners")?;
                    let state = self
                        .world
                        .changes
                        .get(crate::world::ObjectKey::Generated(id.clone()))
                        .and_then(|c| c.snapshot.as_ref())
                        .ok_or_else(|| invalid("Missing generated carry state"))?;
                    links(
                        state.held && state.attachment.is_none(),
                        "Generated carry ownership",
                    )?;
                    data(
                        state
                            .pose
                            .position
                            .iter()
                            .zip(p.pose.position)
                            .all(|(a, b)| (*a - b).abs() <= f64::from(crate::world::REBASE_LIMIT)),
                        "Held relative position",
                    )?;
                    data(bounded_local(&carry.offset), "Carry offset")?;
                    quaternion(&carry.rotation)?;
                    local(&carry.local_grip)?;
                    return Ok(());
                }
                let id = ids.item(&carry.item)?;
                links(
                    held.len() == 1 && held[0].instance == carry.item,
                    "Held item ownership",
                )?;
                let pickup = reference
                    .items
                    .iter()
                    .find(|i| i.id == id)
                    .unwrap()
                    .pickup
                    .as_ref();
                links(
                    pickup.is_some_and(|rule| {
                        !rule.cant_pickup && !(p.seat.is_some() && rule.cant_pickup_while_seated)
                    }),
                    "Item cannot be held",
                )?;
                data(
                    items[&id]
                        .pose
                        .position
                        .iter()
                        .zip(p.pose.position)
                        .all(|(a, b)| (*a - b).abs() <= f64::from(crate::world::REBASE_LIMIT)),
                    "Held relative position",
                )?;
                let source_item = reference.items.iter().find(|i| i.id == id).unwrap();
                let assembly = reference.parts.iter().any(|p| p.item == id)
                    || reference.wheel_parts.iter().any(|p| p.item == id);
                let body_position = if assembly {
                    self.bodies.iter().find(|b|matches!(&b.identity,BodyIdentity::Assembly {root} if *root==carry.item)).map(|b|b.pose.position)
                } else {
                    let source = reference
                        .bodies
                        .iter()
                        .find(|b| b.id == source_item.body)
                        .unwrap();
                    Some(self.bodies.iter().find(|b|matches!(&b.identity,BodyIdentity::Source {source:key} if *key==SourceKey::from(&source.source))).map_or(source.pose.translation.map(f64::from),|b|b.pose.position))
                };
                links(body_position.is_some(), "Missing held body")?;
                data(
                    body_position
                        .unwrap()
                        .iter()
                        .zip(p.pose.position)
                        .all(|(a, b)| (*a - b).abs() <= f64::from(crate::world::REBASE_LIMIT)),
                    "Held body relative position",
                )?;
                data(bounded_local(&carry.offset), "Carry offset")?;
                quaternion(&carry.rotation)?;
                local(&carry.local_grip)?;
            }
            None => links(held.is_empty(), "Missing carry state")?,
        }
        Ok(())
    }

    fn validate_bodies(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
        items: &HashMap<r::ItemId, &ItemSnapshot>,
    ) -> Result<(), SaveError> {
        let mut identities = HashSet::new();
        for vehicle in &self.vehicles {
            identities.insert(&vehicle.body.identity);
        }
        let mut assemblies = HashSet::new();
        for record in &self.bodies {
            links(
                identities.insert(&record.identity),
                "Duplicate independent body",
            )?;
            body(record)?;
            match &record.identity {
                BodyIdentity::Source { source } => {
                    let id = ids
                        .bodies
                        .get(source)
                        .ok_or_else(|| invalid("Unknown source body"))?;
                    links(
                        !reference.vehicles.iter().any(|v| v.body == *id),
                        "Duplicate vehicle body",
                    )?;
                    let source_body = reference.bodies.iter().find(|body| body.id == *id).unwrap();
                    links(
                        source_body.parent_body.is_none(),
                        "Parented source body is not independent",
                    )?;
                }
                BodyIdentity::Assembly { root } => {
                    let id = ids.item(root)?;
                    links(
                        reference.parts.iter().any(|p| p.item == id)
                            || reference.wheel_parts.iter().any(|p| p.item == id),
                        "Unknown assembly root",
                    )?;
                    links(
                        matches!(
                            items[&id].location,
                            ItemLocationSave::World | ItemLocationSave::Held
                        ),
                        "Assembly root is attached",
                    )?;
                    assemblies.insert(id);
                }
            }
        }
        for part in reference
            .parts
            .iter()
            .map(|p| p.item)
            .chain(reference.wheel_parts.iter().map(|p| p.item))
        {
            if matches!(
                items[&part].location,
                ItemLocationSave::World | ItemLocationSave::Held
            ) {
                links(assemblies.contains(&part), "Missing detached assembly body")?;
            }
        }
        Ok(())
    }

    fn validate_mechanisms(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
    ) -> Result<(), SaveError> {
        let mut controls = HashSet::new();
        for saved in &self.mechanisms.controls {
            let id = ids
                .mechanisms
                .get(&saved.source)
                .ok_or_else(|| invalid("Unknown mechanism"))?;
            links(controls.insert(id), "Duplicate mechanism")?;
            let source = reference.mechanisms.iter().find(|m| m.id == *id).unwrap();
            data(
                finite(&saved.degrees) && unit(saved.slide),
                "Mechanism values",
            )?;
            match &source.rotation {
                Some(rotation) => data(
                    (0..2).all(|axis| {
                        saved.degrees[axis] == rotation.initial_degrees[axis]
                            || (rotation.limits_degrees[axis][0]..=rotation.limits_degrees[axis][1])
                                .contains(&saved.degrees[axis])
                    }),
                    "Mechanism rotation limits",
                )?,
                None => data(saved.degrees == [0.; 2], "Unexpected mechanism rotation")?,
            }
            if source.slide.is_none() {
                data(saved.slide == 0., "Unexpected slide value")?;
            }
            data(
                saved.turn_state <= source.turn.as_ref().map_or(0, |turn| turn.max_state),
                "Mechanism turn state",
            )?;
        }
        links(
            controls.len() == reference.mechanisms.len(),
            "Missing mechanism",
        )?;
        let mut openings = HashSet::new();
        for saved in &self.mechanisms.openings {
            let id = ids
                .openings
                .get(&saved.source)
                .ok_or_else(|| invalid("Unknown opening"))?;
            links(openings.insert(id), "Duplicate opening")?;
            data(unit(saved.valve), "Opening valve")?;
        }
        links(
            openings.len() == reference.openings.len(),
            "Missing opening",
        )
    }

    fn validate_lighting(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
    ) -> Result<(), SaveError> {
        let cycle = &self.lighting.cycle;
        let duration = f64::from(
            reference.lighting.cycle.day_seconds + reference.lighting.cycle.night_seconds,
        );
        data(
            cycle.elapsed_seconds.is_finite()
                && (cycle.elapsed_seconds as f32).is_finite()
                && cycle.elapsed_seconds >= 0.
                && cycle.cycle_seconds.is_finite()
                && cycle.cycle_seconds >= 0.
                && cycle.cycle_seconds as f32 <= duration as f32
                && cycle.evaluated_cycle_seconds.is_finite()
                && (cycle.evaluated_cycle_seconds as f32).is_finite()
                && cycle.evaluated_cycle_seconds >= 0.
                && cycle.cloud_coordinate.is_finite()
                && (-1.0..=0.0).contains(&cycle.cloud_coordinate)
                && cycle.star_rotation.is_finite()
                && (-360.0..360.0).contains(&cycle.star_rotation),
            "Cycle state",
        )?;
        let mut controllers = HashSet::new();
        for saved in &self.lighting.controllers {
            let id = ids
                .controllers
                .get(&saved.source)
                .ok_or_else(|| invalid("Unknown lamp controller"))?;
            links(controllers.insert(id), "Duplicate lamp controller")?;
            let source = reference
                .lighting
                .controllers
                .iter()
                .find(|c| c.id == *id)
                .unwrap();
            data(
                [
                    saved.current_state,
                    saved.requested_state,
                    saved.refreshed_state,
                ]
                .iter()
                .all(|state| *state < source.states.len())
                    && unit(saved.energy),
                "Lamp controller state",
            )?;
            match (&source.cabin_input, saved.cabin_mode) {
                (Some(input), Some(mode)) => data(mode < input.modes.len(), "Cabin mode")?,
                (None, None) => (),
                _ => return Err(invalid("Cabin mode owner")),
            }
            let mut poses = HashSet::new();
            for pose in &saved.switch_poses {
                let node = ids
                    .nodes
                    .get(&pose.node)
                    .ok_or_else(|| invalid("Unknown switch node"))?;
                links(
                    poses.insert(node) && source.animated_poses.iter().any(|p| p.node == *node),
                    "Duplicate or invalid switch node",
                )?;
                local(&pose.pose)?;
            }
            links(
                poses.len() == source.animated_poses.len(),
                "Missing switch pose",
            )?;
            if let Some(motion) = saved.switch_motion {
                links(
                    !source.animated_poses.is_empty(),
                    "Switch motion without animated poses",
                )?;
                if let SwitchMotionSave::Moving { progress } = motion {
                    data(unit(progress), "Switch motion")?;
                }
            }
        }
        links(
            controllers.len() == reference.lighting.controllers.len(),
            "Missing lamp controller",
        )
    }
}

fn validate_source(
    saved: &SourceStamp,
    expected: &SourceStamp,
    reference: &ReferenceManifest,
) -> Result<(), SaveError> {
    fn hashes<'a>(
        hashes: impl Iterator<Item = (&'a str, &'a str)>,
    ) -> Result<HashMap<&'a str, &'a str>, SaveError> {
        let mut values = HashMap::new();
        for (file, hash) in hashes {
            data(!file.is_empty() && sha256(hash), "Source hash")?;
            data(values.insert(file, hash).is_none(), "Duplicate source hash")?;
        }
        data(!values.is_empty(), "Missing source hashes")?;
        Ok(values)
    }
    data(
        sha256(&saved.manifest_sha256) && sha256(&expected.manifest_sha256),
        "Manifest hash",
    )?;
    let saved_hashes = hashes(
        saved
            .source_hashes
            .iter()
            .map(|h| (h.file.as_str(), h.sha256.as_str())),
    )?;
    let expected_hashes = hashes(
        expected
            .source_hashes
            .iter()
            .map(|h| (h.file.as_str(), h.sha256.as_str())),
    )?;
    let source_hashes = hashes(
        reference
            .source_hashes
            .iter()
            .map(|h| (h.file.as_str(), h.sha256.as_str())),
    )?;
    if saved.app_id != expected.app_id
        || saved.build_id != expected.build_id
        || expected.app_id != reference.app_id
        || expected.build_id != reference.build_id
        || saved.manifest_sha256 != expected.manifest_sha256
        || saved_hashes != expected_hashes
        || expected_hashes != source_hashes
    {
        return Err(SaveError::IncompatibleSources(
            "Build, manifest, or source-file hashes".into(),
        ));
    }
    Ok(())
}
fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}
fn bounded_local(values: &[f32]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && v.abs() <= crate::world::REBASE_LIMIT)
}
fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
fn quaternion(rotation: &[f32; 4]) -> Result<(), SaveError> {
    data(
        finite(rotation)
            && (rotation.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() - 1.).abs() <= 0.0001,
        "Normalized rotation",
    )
}
fn absolute(pose: &AbsolutePose) -> Result<(), SaveError> {
    data(
        pose.position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= crate::world::MAX_ABSOLUTE_COORDINATE),
        "Absolute position",
    )?;
    quaternion(&pose.rotation)
}
fn local(pose: &LocalPose) -> Result<(), SaveError> {
    data(bounded_local(&pose.position), "Local position")?;
    quaternion(&pose.rotation)
}
fn body(body: &BodySnapshot) -> Result<(), SaveError> {
    absolute(&body.pose)?;
    data(
        bounded_local(&body.linear_velocity) && bounded_local(&body.angular_velocity),
        "Body velocity",
    )
}
fn fits(
    slot_main: &[String],
    slot_types: &[String],
    part_main: &[String],
    part_types: &[String],
) -> bool {
    (slot_main.is_empty()
        || part_main.is_empty()
        || slot_main.iter().any(|tag| part_main.contains(tag)))
        && slot_types.iter().any(|tag| part_types.contains(tag))
}
fn invalid(detail: &str) -> SaveError {
    SaveError::InvalidLinks(detail.into())
}
fn links(valid: bool, detail: &str) -> Result<(), SaveError> {
    if valid { Ok(()) } else { Err(invalid(detail)) }
}
fn data(valid: bool, detail: &str) -> Result<(), SaveError> {
    if valid {
        Ok(())
    } else {
        Err(SaveError::CorruptData(detail.into()))
    }
}

// Browser Date.toISOString emits UTC, with optional fractional seconds accepted here.
fn timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    let number = |start: usize, end: usize| -> Option<u32> {
        let digits = bytes.get(start..end)?;
        if !digits.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(digits).ok()?.parse().ok()
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    day >= 1
        && day <= days
        && hour < 24
        && minute < 60
        && second < 60
        && (bytes.len() == 20
            || (bytes.len() > 21
                && bytes[19] == b'.'
                && bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)))
}

#[path = "validate_world.rs"]
mod world;
