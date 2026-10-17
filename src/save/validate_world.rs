use super::*;
use crate::world::{AttachmentSnapshot, GeneratedId, ObjectKey, SpawnContext};

impl SaveSnapshot {
    pub(super) fn validate_world(&self, reference: &ReferenceManifest) -> Result<(), SaveError> {
        self.world
            .generation
            .validate_identity(&reference.content_base)
            .map_err(|e| invalid(&e.0))?;
        let roads = crate::world::RoadIndex::new(
            crate::world::RoadSequence::new(&reference.world, self.world.seed)
                .map_err(|e| invalid(&e.0))?,
            crate::world::TerrainEvaluator::new(&reference.world, self.world.seed),
        );
        roads
            .validate_index(&self.world.generation.roads)
            .map_err(|e| invalid(&e.0))?;
        crate::world::PlacementIndex::new(&reference.world, self.world.seed)
            .validate_index(&self.world.generation.placements)
            .map_err(|e| invalid(&e.0))?;
        let sources: HashSet<_> =
            reference
                .world
                .records
                .iter()
                .map(|r| &r.source)
                .chain(
                    reference
                        .world
                        .initialization
                        .components
                        .iter()
                        .map(|r| &r.source),
                )
                .chain(
                    reference
                        .world
                        .initialization
                        .nodes
                        .iter()
                        .flat_map(|r| [&r.source, &r.owner]),
                )
                .chain(reference.world.prototypes.iter().flat_map(|p| {
                    std::iter::once(&p.source).chain(p.nodes.iter().map(|n| &n.source))
                }))
                .collect();
        let source = |s: &r::SourceId| links(sources.contains(s), "Unknown source-world identity");
        let generated = |id: &GeneratedId| -> Result<(), SaveError> {
            source(&id.generator)?;
            for (child, _) in &id.children {
                source(child)?;
            }
            data(
                id.unit
                    .iter()
                    .all(|v| v.unsigned_abs() <= crate::world::MAX_ABSOLUTE_COORDINATE as u64),
                "Generated unit",
            )
        };
        let object = |key: &ObjectKey| -> Result<(), SaveError> {
            match key {
                ObjectKey::Imported(id) => links(
                    reference.items.iter().any(|i| i.id == *id),
                    "Unknown imported world item",
                ),
                ObjectKey::Generated(id) => generated(id),
            }
        };
        let exists = |key: &ObjectKey| match key {
            ObjectKey::Imported(id) => reference.items.iter().any(|i| i.id == *id),
            ObjectKey::Generated(_) => self
                .world
                .changes
                .get(key)
                .is_some_and(|c| c.snapshot.is_some()),
        };
        let attachment = |a: &AttachmentSnapshot| -> Result<(), SaveError> {
            local(&LocalPose {
                position: a.local_pose.position.map(|v| v as f32),
                rotation: a.local_pose.rotation,
            })?;
            data(
                a.local_pose
                    .position
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= f64::from(crate::world::REBASE_LIMIT)),
                "Attachment local position",
            )?;
            if let Some(owner) = &a.owner {
                object(owner)?;
                links(exists(owner), "Removed or missing attachment owner")?;
            }
            if let Some(slot) = &a.slot {
                links(
                    sources.contains(slot)
                        || reference.part_slots.iter().any(|s| &s.source == slot)
                        || reference.wheel_slots.iter().any(|s| &s.source == slot),
                    "Unknown attachment slot",
                )?;
            }
            links(a.owner.is_some() || a.slot.is_some(), "Empty attachment")
        };
        let pose = |p: &crate::world::AbsolutePose| {
            absolute(&AbsolutePose {
                position: p.position,
                rotation: p.rotation,
            })
        };
        let mut edges = HashMap::new();
        let mut slots = HashSet::new();
        let mut held = None;
        for (key, change) in self.world.changes.iter() {
            let ObjectKey::Generated(id) = key else {
                return Err(invalid(
                    "Imported state has a separate canonical save owner",
                ));
            };
            generated(id)?;
            let replay = change
                .replay
                .as_ref()
                .ok_or_else(|| invalid("Missing original placement"))?;
            let instance = change
                .instance
                .as_ref()
                .ok_or_else(|| invalid("Missing generated instance"))?;
            generated(&replay.id)?;
            generated(instance)?;
            links(
                reference
                    .world
                    .prototypes
                    .iter()
                    .any(|p| p.source == replay.prototype),
                "Unknown replay prototype",
            )?;
            links(
                descendant(id, instance) && descendant(instance, &replay.id),
                "Replay identity ancestry",
            )?;
            pose(&replay.pose)?;
            data(
                replay.scale.iter().all(|v| v.is_finite() && *v != 0.),
                "Replay scale",
            )?;
            data(
                (0..3).all(|i| {
                    replay.bounds.min[i].is_finite()
                        && replay.bounds.max[i].is_finite()
                        && replay.bounds.min[i] <= replay.bounds.max[i]
                        && replay.bounds.min[i].abs() <= crate::world::MAX_ABSOLUTE_COORDINATE
                        && replay.bounds.max[i].abs() <= crate::world::MAX_ABSOLUTE_COORDINATE
                }),
                "Replay bounds",
            )?;
            let Some(s) = &change.snapshot else {
                links(
                    id == instance,
                    "Removal must identify a complete generated instance",
                )?;
                continue;
            };
            pose(&s.pose)?;
            data(
                unit(s.condition)
                    && s.temperature.is_none_or(f32::is_finite)
                    && s.valve.is_none_or(unit),
                "Generated scalar state",
            )?;
            data(
                bounded_local(&s.linear_velocity) && bounded_local(&s.angular_velocity),
                "Generated velocity",
            )?;
            data(
                s.fluid_quantities.is_none_or(|q| {
                    q.iter().all(|v| v.is_finite() && *v >= 0.)
                        && q.iter().map(|v| f64::from(*v)).sum::<f64>() <= f64::from(f32::MAX)
                }),
                "Generated fluids",
            )?;
            if let Some(food) = &s.food {
                let recipe = reference
                    .survival
                    .foods
                    .iter()
                    .find(|f| f.source == food.source)
                    .ok_or_else(|| invalid("Unknown edible recipe"))?;
                data(
                    food.bite <= recipe.bites
                        && !(food.bite == recipe.bites && recipe.destroy_on_end),
                    "Edible bite state",
                )?;
            }
            if let Some(m) = &s.mechanism {
                data(finite(&m.degrees) && unit(m.slide), "Generated mechanism")?;
            }
            if s.held {
                links(
                    held.replace(key).is_none() && s.attachment.is_none(),
                    "Generated held ownership",
                )?;
            }
            if let Some(a) = &s.attachment {
                attachment(a)?;
                if let Some(owner) = &a.owner {
                    edges.insert(key.clone(), owner.clone());
                }
                if let Some(slot) = &a.slot {
                    links(
                        slots.insert((a.owner.clone(), slot.clone())),
                        "Duplicate generated slot owner",
                    )?;
                }
            }
        }
        let ids = IdentityMap::new(reference)?;
        for item in &self.items {
            let key = ObjectKey::Imported(ids.item(&item.instance)?);
            match &item.location {
                ItemLocationSave::GeneratedAttachment { attachment: a } => {
                    attachment(a)?;
                    links(
                        matches!(a.owner, Some(ObjectKey::Generated(_))),
                        "Generated attachment owner",
                    )?;
                    if let Some(owner) = &a.owner {
                        edges.insert(key, owner.clone());
                    }
                    if let Some(slot) = &a.slot {
                        links(
                            slots.insert((a.owner.clone(), slot.clone())),
                            "Duplicate cross-source slot",
                        )?;
                    }
                }
                ItemLocationSave::Attached { owner } => {
                    edges.insert(key, ObjectKey::Imported(ids.item(owner)?));
                }
                _ => (),
            }
        }
        for start in edges.keys() {
            let mut seen = HashSet::new();
            let mut next = Some(start);
            while let Some(key) = next {
                links(seen.insert(key), "World attachment cycle")?;
                next = edges.get(key);
            }
        }
        links(
            held.map(|key| match key {
                ObjectKey::Generated(id) => InstanceId::Generated { id: id.clone() },
                _ => unreachable!(),
            }) == self.player.carry.as_ref().and_then(|c| {
                matches!(c.item, InstanceId::Generated { .. }).then(|| c.item.clone())
            }),
            "Generated carry record",
        )?;
        let vehicle_index = ids
            .vehicles
            .iter()
            .position(|v| v == &self.progress.selected_vehicle)
            .ok_or_else(|| invalid("Selected vehicle identity"))?;
        links(
            self.world.spawn.context.current_car == reference.vehicles[vehicle_index].spawn_car_id,
            "Selected source spawn car",
        )?;
        for record in &self.attachments.parts {
            if record.part.is_some() {
                let slot_id = ids
                    .part_slots
                    .get(&record.slot)
                    .ok_or_else(|| invalid("Unknown source part slot"))?;
                let slot = reference
                    .part_slots
                    .iter()
                    .find(|s| s.id == *slot_id)
                    .unwrap();
                links(
                    slots.insert((slot.owner.map(ObjectKey::Imported), slot.source.clone())),
                    "Duplicate cross-source part slot",
                )?;
            }
        }
        for record in &self.attachments.wheels {
            if record.rim.is_some() {
                let slot_id = ids
                    .wheel_slots
                    .get(&record.slot)
                    .ok_or_else(|| invalid("Unknown source wheel slot"))?;
                let vehicle = reference
                    .vehicles
                    .iter()
                    .find(|v| v.wheel_slots.contains(slot_id))
                    .unwrap();
                let owner = reference
                    .items
                    .iter()
                    .find(|i| i.node == vehicle.root)
                    .map(|i| ObjectKey::Imported(i.id));
                links(
                    slots.insert((owner, record.slot.source_id()?)),
                    "Duplicate cross-source wheel slot",
                )?;
            }
        }
        let context = |c: SpawnContext| -> Result<(), SaveError> {
            data(
                c.date == reference.world.spawn_context.date
                    && c.chance_bits.is_none_or(|v| f32::from_bits(v).is_finite()),
                "Source first-call context",
            )
        };
        context(self.world.spawn.context)?;
        data(
            self.world.spawn.live.words != [0; 4],
            "Source live random state",
        )?;
        let mut boundaries = HashSet::new();
        for b in &self.world.spawn.boundaries {
            generated(&b.parent)?;
            source(&b.caller)?;
            context(b.context)?;
            data(b.random.words != [0; 4], "Source boundary random state")?;
            links(
                boundaries.insert((&b.parent, &b.caller, b.ordinal, b.phase as u8)),
                "Duplicate source first-call boundary",
            )?;
            for p in &b.painting_pools {
                source(&p.source)?;
            }
            let init = &b.initialization;
            for (id, s, _) in &init.paintings {
                generated(id)?;
                source(s)?;
            }
            for (id, s, scale) in &init.scales {
                generated(id)?;
                source(s)?;
                data(finite(scale), "Source initialization scale")?;
            }
            for (id, s, _) in &init.lamps {
                generated(id)?;
                source(s)?;
            }
            for (id, s) in &init.initialized {
                generated(id)?;
                source(s)?;
            }
            for (id, s, _) in &init.selectors {
                generated(id)?;
                source(s)?;
            }
            for (id, s, _) in &init.activations {
                generated(id)?;
                source(s)?;
            }
            for (id, s, part) in &init.slot_parts {
                generated(id)?;
                source(s)?;
                source(part)?;
            }
            for c in &init.conditions {
                generated(&c.parent)?;
                source(&c.source)?;
                data(finite(&c.color), "Source condition color")?;
            }
        }
        for p in &self.world.spawn.painting_pools {
            source(&p.source)?;
        }
        crate::world::SourceSpawnState::restore_history(self.world.spawn.clone())?;
        Ok(())
    }
}
fn descendant(child: &GeneratedId, parent: &GeneratedId) -> bool {
    child.layer == parent.layer
        && child.generator == parent.generator
        && child.unit == parent.unit
        && child.ordinal == parent.ordinal
        && child.children.starts_with(&parent.children)
}
