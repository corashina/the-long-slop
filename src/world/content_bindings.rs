//! Generated item bindings belong to their published source instance.
use super::WorldStream;
use crate::interaction::{InteractionState, items::ItemStore};
use crate::reference::{ActionBinding, PickupReference, ReferenceError, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, from_rapier, to_rapier};
use crate::world::spawn_rules::*;
impl WorldStream {
    /// Bounds new owner payload, type-fit pairs, and existing vector/map reallocation.
    pub(super) fn binding_reservation(
        &self,
        prototype: &crate::reference::WorldPrototypeReference,
        id: &crate::world::GeneratedId,
    ) -> Result<(usize, usize), ReferenceError> {
        let mut components = 0usize;
        let mut parts = 0usize;
        let mut fields_bytes = 0usize;
        for node in &prototype.nodes {
            for class in [
                "pickupable",
                "partscript",
                "partslotscript",
                "tankcapscript",
                "tankscript",
                "wheelscript",
                "gumiscript",
                "flashlightscript",
            ] {
                if let Some(record) =
                    self.contents
                        .rules
                        .component(&self.reference, &node.source, class)?
                {
                    components += 1;
                    parts += usize::from(matches!(class, "partscript" | "partslotscript"));
                    fields_bytes += match record.fields {
                        Fields::Record(fields) => {
                            fields.values().map(|v| v.input_heap_bytes()).sum::<usize>()
                        }
                        Fields::Compact(fields) => fields
                            .iter()
                            .map(|f| f.value.input_heap_bytes())
                            .sum::<usize>(),
                    };
                }
            }
        }
        components += self
            .reference
            .mechanisms
            .iter()
            .filter(|r| r.prototype == prototype.source)
            .count();
        if components == 0 {
            return Ok((0, 0));
        }
        let identities = components.saturating_mul(
            super::super::contents::identity_heap(id)
                + prototype
                    .nodes
                    .iter()
                    .map(|n| n.name.len())
                    .max()
                    .unwrap_or(0)
                + 128,
        );
        // Each component covers its typed owner, maps, copied recipes and auxiliary item.
        // Fits can add a pair to both directions; 64 bytes covers the hash/table slot.
        let additions = components.saturating_mul(8192)
            + prototype.nodes.len().saturating_mul(1024)
            + prototype.colliders.len().saturating_mul(2048)
            + fields_bytes.saturating_mul(8)
            + identities.saturating_mul(8)
            + parts
                .saturating_mul(self.binding_objects + components)
                .saturating_mul(128);
        Ok((
            self.binding_owner_bytes
                .saturating_mul(2)
                .saturating_add(additions.saturating_mul(2)),
            components,
        ))
    }
    pub(super) fn sync_static_controls(
        &mut self,
        reference: &ReferenceManifest,
        items: &mut ItemStore,
        interaction: &mut InteractionState,
        physics: &mut PhysicsWorld,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) -> Result<(), ReferenceError> {
        for content in &mut self.static_controls {
            crate::world::content_mechanisms::sync(content, items, interaction, physics);
        }
        for index in 0..self.structures.len() {
            let instance = &mut self.structures[index];
            if self
                .contents
                .active
                .iter()
                .any(|c| c.content.placement.id == instance.placement.id)
                || self
                    .static_controls
                    .iter()
                    .any(|c| c.content.placement.id == instance.placement.id)
                || !reference
                    .world
                    .mechanisms
                    .iter()
                    .any(|r| r.prototype == instance.placement.prototype)
            {
                continue;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .unwrap();
            let mut content =
                crate::world::contents::ContentInstance::for_structure(instance.placement.clone());
            content.item = Some(items.insert_generated(
                instance.placement.id.clone(),
                prototype,
                instance.placement.pose,
            )?);
            if let Err(error) = crate::world::content_mechanisms::bind(
                &mut content,
                instance,
                prototype,
                reference,
                &self.contents.rules,
                items,
                interaction,
                physics,
            ) {
                rollback(&mut content, instance, items, interaction, physics, wheels);
                self.structures.swap_remove(index).retire(physics);
                return Err(error);
            }
            self.static_controls.push(content);
        }
        Ok(())
    }
    /// Borrowed source baselines for active generated item bindings.
    pub fn generated_baselines(
        &self,
    ) -> impl Iterator<Item = (crate::reference::ItemId, &crate::world::ObjectSnapshot)> {
        self.contents
            .active
            .iter()
            .chain(&self.static_controls)
            .flat_map(|c| {
                c.item
                    .zip(c.baseline.as_ref())
                    .into_iter()
                    .chain(c.bindings.iter().map(|b| (b.item, &b.baseline)))
            })
    }
    pub fn generated_cap_visuals(
        &self,
        id: &crate::world::GeneratedId,
    ) -> &[crate::world::GeneratedCapVisual] {
        self.contents
            .active
            .iter()
            .find(|c| &c.content.placement.id == id)
            .map_or(&[], |c| c.caps.as_slice())
    }
    pub fn generated_node_pose(
        &self,
        id: &crate::world::GeneratedId,
        node: usize,
    ) -> Option<crate::vehicle::VehiclePose> {
        self.contents
            .active
            .iter()
            .chain(&self.static_controls)
            .find(|c| &c.content.placement.id == id)
            .and_then(|c| c.poses.get(node))
            .copied()
    }
    pub(super) fn sync_contents(
        &mut self,
        reference: &ReferenceManifest,
        items: &mut ItemStore,
        interaction: &mut InteractionState,
        physics: &mut PhysicsWorld,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) -> Result<(), ReferenceError> {
        for content in &mut self.contents.active {
            crate::world::content_mechanisms::sync(content, items, interaction, physics);
            for binding in &content.bindings {
                if let (Some(state), Some((body, local))) =
                    (items.get(binding.item), binding.render_body)
                {
                    if state.body != Some(body) {
                        let pose = to_rapier(state.pose) * to_rapier(local).inverse();
                        if physics.rapier().bodies[body].position() != &pose {
                            physics.rapier_mut().bodies[body].set_position(pose, false);
                        }
                    }
                }
            }
            if let Some(item) = content.item {
                if let (Some(state), Some((body, local))) = (items.get(item), content.render_body) {
                    if state.body != Some(body) {
                        let pose = to_rapier(state.pose) * to_rapier(local).inverse();
                        if physics.rapier().bodies[body].position() != &pose {
                            physics.rapier_mut().bodies[body].set_position(pose, false);
                        }
                    }
                }
                continue;
            }
            let placement = content.content.placement.clone();
            let Some(instance) = self
                .structures
                .iter_mut()
                .find(|i| i.placement.id == placement.id)
            else {
                continue;
            };
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == placement.prototype)
                .unwrap();
            let pickup_root = if let Some(unparent) = self.contents.rules.component(
                &self.reference,
                &placement.prototype,
                "childunparent",
            )? {
                link(unparent.fields, "g")?
            } else {
                &placement.prototype
            };
            let Some(pickup) =
                self.contents
                    .rules
                    .component(&self.reference, pickup_root, "pickupable")?
            else {
                continue;
            };
            let result = (|| -> Result<(), ReferenceError> {
                let rule = pickup_rule(pickup)?;
                let root = self.contents.rules.root(&self.reference, pickup.source)?;
                let body_index = prototype
                    .bodies
                    .iter()
                    .position(|b| prototype.nodes[b.node as usize].source == *root)
                    .ok_or_else(|| ReferenceError("Generated pickup has no source body".into()))?;
                let body = instance.bodies[body_index];
                let pickup_node = prototype
                    .nodes
                    .iter()
                    .position(|n| &n.source == root)
                    .ok_or_else(|| ReferenceError("Generated pickup transform missing".into()))?;
                let pickup_pose = crate::world::placement::place_pose(
                    placement.pose,
                    placement.scale,
                    crate::world::placement::node_pose(prototype, pickup_node),
                );
                let id = items.insert_generated(placement.id.clone(), prototype, pickup_pose)?;
                content.item = Some(id);
                let state = items.get_mut(id).unwrap();
                state.body = Some(body);
                state.fluid = content.content.fluid.clone();
                state.condition = content.content.condition;
                state.local_pose = from_rapier(
                    &(physics.rapier().bodies[body].position().inverse() * to_rapier(state.pose)),
                );
                let state = items.get(id).unwrap();
                let body_pose = to_rapier(state.pose) * to_rapier(state.local_pose).inverse();
                physics.rapier_mut().bodies[body].set_position(body_pose, true);
                let mut layers = 0u32;
                for (source, handle) in &instance.colliders {
                    let collider = prototype
                        .colliders
                        .iter()
                        .find(|c| c.source == *source)
                        .unwrap();
                    if collider.body != Some(body_index as u32) {
                        continue;
                    }
                    layers |= 1u32 << prototype.nodes[collider.node as usize].layer;
                    let collider_id = interaction.allocate_generated_collider()?;
                    physics.rapier_mut().colliders[*handle].user_data =
                        ((collider_id.0 as u128) << 64) | (id.0 as u128 + 1);
                    interaction.bind_surface(
                        *handle,
                        ActionBinding {
                            collider: collider_id,
                            use_owner: None,
                            pickup_owner: Some(id),
                            removal_owner: None,
                            push_owner: None,
                        },
                    );
                }
                interaction.register_generated_pickup(
                    id,
                    rule.clone(),
                    layers,
                    state.fluid.is_some(),
                );
                content.render_body = Some((body, state.local_pose));
                content.item = Some(id);
                crate::world::content_containers::bind(
                    content,
                    instance,
                    prototype,
                    &self.contents.rules,
                    reference,
                    &self.changes,
                    self.origin,
                    items,
                    interaction,
                    physics,
                )?;
                crate::world::content_parts::bind(
                    content,
                    instance,
                    prototype,
                    &self.contents.rules,
                    reference,
                    rule,
                    layers,
                    items,
                    interaction,
                    physics,
                )?;
                crate::world::content_wheels::bind(
                    content,
                    instance,
                    prototype,
                    &self.contents.rules,
                    reference,
                    items,
                    interaction,
                    physics,
                    wheels,
                )?;
                crate::world::content_slots::bind(
                    content,
                    instance,
                    prototype,
                    &self.contents.rules,
                    reference,
                    items,
                    interaction,
                    physics,
                    wheels,
                )?;
                crate::world::content_mechanisms::bind(
                    content,
                    instance,
                    prototype,
                    reference,
                    &self.contents.rules,
                    items,
                    interaction,
                    physics,
                )?;
                if interaction.parts.contains(id) {
                    wheels.register_generated_general(
                        id,
                        physics.rapier().bodies[body].mass(),
                        items.get(id).unwrap().fluid.is_some(),
                    );
                }
                content.baseline = items.generated_snapshot(id, None);
                if let Some(change) = self
                    .changes
                    .get(crate::world::ObjectKey::Generated(placement.id.clone()))
                {
                    items.restore_generated(id, &change.snapshot)?;
                }
                let state = items.get(id).unwrap();
                physics.rapier_mut().bodies[body].set_position(
                    to_rapier(state.pose) * to_rapier(state.local_pose).inverse(),
                    true,
                );
                Ok(())
            })();
            if let Err(error) = result {
                rollback(content, instance, items, interaction, physics, wheels);
                let index = self
                    .structures
                    .iter()
                    .position(|s| s.placement.id == placement.id)
                    .unwrap();
                self.structures.swap_remove(index).retire(physics);
                return Err(error);
            }
        }
        for content_index in 0..self.contents.active.len() {
            for slot_index in 0..self.contents.active[content_index].slots.len() {
                let content = &self.contents.active[content_index];
                let binding = &content.slots[slot_index];
                let source = &binding.source;
                let slot = binding.slot;
                if binding.initialized || interaction.parts.slot_item(slot).is_some() {
                    continue;
                }
                let embedded = self
                    .contents
                    .selected_slot_part(&content.content.placement.id, source, &self.reference)?
                    .and_then(|part| {
                        content
                            .bindings
                            .iter()
                            .find(|b| &b.source == part)
                            .map(|b| b.item)
                    });
                let spawned = self
                    .contents
                    .active
                    .iter()
                    .find(|child| {
                        let mut iter = child.content.placement.id.children.iter();
                        child.content.placement.id.layer == content.content.placement.id.layer
                            && child.content.placement.id.generator
                                == content.content.placement.id.generator
                            && child.content.placement.id.unit == content.content.placement.id.unit
                            && child.content.placement.id.ordinal
                                == content.content.placement.id.ordinal
                            && child.content.placement.id.children.len()
                                == content.content.placement.id.children.len() + 1
                            && iter
                                .by_ref()
                                .take(content.content.placement.id.children.len())
                                .eq(content.content.placement.id.children.iter())
                            && iter.next().is_some_and(|(s, n)| s == source && *n == 0)
                    })
                    .and_then(|c| c.item);
                if let Some(item) = embedded
                    .or(spawned)
                    .filter(|i| interaction.parts.contains(*i))
                {
                    interaction
                        .parts
                        .initialize_generated_slot(item, slot, items, physics)?;
                    interaction
                        .parts
                        .mount_mechanism(item, &mut interaction.mechanisms);
                    if let Some(snapshot) =
                        items.generated_snapshot(item, interaction.generated_valve(item))
                    {
                        for owner in &mut self.contents.active {
                            if owner.item == Some(item) {
                                owner.baseline = Some(snapshot);
                                break;
                            }
                            if let Some(binding) =
                                owner.bindings.iter_mut().find(|b| b.item == item)
                            {
                                binding.baseline = snapshot;
                                break;
                            }
                        }
                    }
                    self.contents.active[content_index].slots[slot_index].initialized = true;
                }
            }
        }
        Ok(())
    }
    pub(super) fn retire_contents(
        &mut self,
        physics: &mut PhysicsWorld,
        items: &mut ItemStore,
        interaction: &mut InteractionState,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) {
        let remove = self.reference.distances["ItemRemoveDist"];
        let freeze = self.reference.distances["ItemFreezeDist"];
        let unfreeze = self.reference.distances["ItemUnFreezeDist"];
        let mut index = 0;
        while index < self.contents.active.len() {
            let content = &self.contents.active[index];
            let id = content.item;
            let pose = id
                .and_then(|id| items.get(id))
                .map(|s| crate::world::AbsolutePose {
                    position: self.origin.absolute(s.pose.position),
                    rotation: s.pose.rotation,
                })
                .unwrap_or(content.content.placement.pose);
            let offset =
                crate::world::placement::rotate(pose.rotation, content.content.distance_offset);
            let position = std::array::from_fn(|i| pose.position[i] + offset[i] as f64);
            let mut distance = [self.request.player, self.request.vehicle]
                .into_iter()
                .map(|p| crate::world::contents::distance(p, position))
                .fold(f32::INFINITY, f32::min);
            let local_item = |item| {
                content.item == Some(item) || content.bindings.iter().any(|b| b.item == item)
            };
            let local_slot = |slot| {
                content.slots.iter().any(|s| s.slot == slot)
                    || self.contents.active.iter().any(|owner| {
                        let parent = &owner.content.placement.id;
                        let child = &content.content.placement.id;
                        parent.layer == child.layer
                            && parent.generator == child.generator
                            && parent.unit == child.unit
                            && parent.ordinal == child.ordinal
                            && child.children.len() > parent.children.len()
                            && child.children.starts_with(&parent.children)
                            && owner.slots.iter().any(|s| s.slot == slot)
                    })
            };
            let protected_location = |location| match location {
                crate::interaction::items::ItemLocation::World => false,
                crate::interaction::items::ItemLocation::Attached(owner) => !local_item(owner),
                crate::interaction::items::ItemLocation::PartSlot(slot) => !local_slot(slot),
                _ => true,
            };
            let mut protected = id
                .and_then(|id| items.get(id))
                .is_some_and(|item| protected_location(item.location));
            for binding in &content.bindings {
                if let Some(item) = items.get(binding.item) {
                    protected |= protected_location(item.location);
                    let position = self.origin.absolute(item.pose.position);
                    distance = distance.min(
                        [self.request.player, self.request.vehicle]
                            .into_iter()
                            .map(|p| crate::world::contents::distance(p, position))
                            .fold(f32::INFINITY, f32::min),
                    );
                }
            }
            protected |= self.contents.active.iter().any(|child| child.item.and_then(|i| items.get(i)).is_some_and(|item| {
                matches!(item.location, crate::interaction::items::ItemLocation::PartSlot(slot) if content.slots.iter().any(|s| s.slot == slot))
            }));
            if distance < remove * remove || protected {
                if let Some(body) = id.and_then(|id| items.get(id)).and_then(|s| s.body) {
                    if !protected
                        && (content.content.buried
                            || distance >= freeze * freeze
                            || distance < unfreeze * unfreeze)
                    {
                        physics.rapier_mut().bodies[body].set_body_type(
                            if content.content.buried || distance >= freeze * freeze {
                                rapier3d::prelude::RigidBodyType::KinematicPositionBased
                            } else {
                                rapier3d::prelude::RigidBodyType::Dynamic
                            },
                            true,
                        );
                    }
                }
                index += 1;
                continue;
            }
            let content = self.contents.active.swap_remove(index);
            for binding in &content.bindings {
                interaction.parts.remove_generated(binding.item);
            }
            for slot in &content.slots {
                interaction.parts.remove_generated_slot(slot.slot);
                items.remove_generated_slot(slot.slot);
            }
            if let Some(item) = content.item {
                interaction.parts.remove_generated(item);
                wheels.remove_generated(item);
                interaction
                    .mechanisms
                    .remove_generated(item, &content.nodes);
            }
            for binding in &content.bindings {
                wheels.remove_generated(binding.item);
                let mut key = content.content.placement.id.clone();
                key.children.push((binding.source.clone(), 0));
                if let Some(snapshot) = items
                    .generated_snapshot(binding.item, interaction.generated_valve(binding.item))
                {
                    self.changes.record(
                        crate::world::ObjectKey::Generated(key),
                        snapshot,
                        &binding.baseline,
                    );
                }
                if let Some(instance) = self
                    .structures
                    .iter()
                    .find(|i| i.placement.id == content.content.placement.id)
                {
                    for (_, collider) in &instance.colliders {
                        interaction.unbind_generated(binding.item, std::slice::from_ref(collider));
                    }
                }
                items.get_mut(binding.item).unwrap().body = None;
                items.get_mut(binding.item).unwrap().location =
                    crate::interaction::items::ItemLocation::World;
                items.remove_generated(binding.item);
            }
            if let (Some(id), Some(baseline)) = (content.item, content.baseline) {
                if let Some(snapshot) = items.generated_snapshot(id, None) {
                    self.changes.record(
                        crate::world::ObjectKey::Generated(content.content.placement.id.clone()),
                        snapshot,
                        &baseline,
                    );
                }
                if let Some(instance) = self
                    .structures
                    .iter()
                    .find(|i| i.placement.id == content.content.placement.id)
                {
                    for (_, collider) in &instance.colliders {
                        interaction.unbind_generated(id, std::slice::from_ref(collider));
                    }
                }
                items.get_mut(id).unwrap().body = None;
                items.remove_generated(id);
            }
            self.wanted_structures
                .retain(|p| p.id != content.content.placement.id);
            self.ready_structures
                .retain(|id| *id != content.content.placement.id);
        }
    }
}

pub(crate) fn pickup_rule(pickup: SourceRecord<'_>) -> Result<PickupReference, ReferenceError> {
    let f = pickup.fields;
    let mut position = vector(f, "recenterPos")?;
    position[2] = -position[2];
    Ok(PickupReference {
        source: pickup.source.clone(),
        cant_pickup: flag(f, "cantPickup")?,
        cant_pickup_while_mounted: flag(f, "cantPickupWhileMount")?,
        cant_pickup_while_seated: flag(f, "cantPickupWhileSitting")?,
        no_rotation: flag(f, "noRot")?,
        move_with_rotation: flag(f, "moveWithRot")?,
        use_recenter_position: flag(f, "useRecenter")?,
        recenter_position: position,
        use_recenter_rotation: flag(f, "useRecenterRot")?,
        recenter_rotation: crate::world::spawn_building::source_euler(vector(f, "recenterRot")?),
        recenter_distance: number(f, "recenterDist")?,
    })
}

// Failed unpublished bindings cannot leave item identities or attachment owners behind.
pub(super) fn rollback(
    content: &mut super::super::contents::ContentInstance,
    instance: &super::super::StructureInstance,
    items: &mut ItemStore,
    interaction: &mut InteractionState,
    physics: &mut PhysicsWorld,
    wheels: &mut crate::interaction::wheels::WheelAssemblies,
) {
    for item in content.bindings.iter().map(|b| b.item).chain(content.item) {
        interaction.parts.remove_generated(item);
        wheels.remove_generated(item);
        for (_, collider) in &instance.colliders {
            interaction.unbind_generated(item, std::slice::from_ref(collider));
        }
        if let Some(state) = items.get_mut(item) {
            state.body = None;
            state.location = crate::interaction::items::ItemLocation::World;
        }
        items.remove_generated(item);
    }
    for slot in &content.slots {
        interaction.parts.remove_generated_slot(slot.slot);
        items.remove_generated_slot(slot.slot);
    }
    if let Some(item) = content.item {
        interaction
            .mechanisms
            .remove_generated(item, &content.nodes);
    }
    for (_, collider) in &instance.colliders {
        physics.rapier_mut().colliders[*collider].user_data = 0;
    }
    content.item = None;
    content.baseline = None;
    content.render_body = None;
    content.bindings.clear();
    content.slots.clear();
    content.caps.clear();
    content.nodes.clear();
    content.poses.clear();
    content.mechanism_colliders.clear();
}
