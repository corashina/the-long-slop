//! Generated item bindings belong to their published source instance.
use super::WorldStream;
use crate::interaction::{
    InteractionState,
    items::{HitSound, ItemStore},
};
use crate::reference::{ActionBinding, PickupReference, ReferenceError, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, from_rapier, to_rapier};
use crate::world::spawn_rules::*;
use crate::world::{ObjectKey, changes};
use rapier3d::prelude::ActiveEvents;
impl WorldStream {
    pub fn initialize_imported_changes(
        &mut self,
        reference: &ReferenceManifest,
        items: &ItemStore,
        interaction: &InteractionState,
        physics: &PhysicsWorld,
    ) {
        if self.imported_baselines.is_empty() {
            self.imported_baselines
                .extend(items.iter().filter_map(|item| {
                    crate::world::changes::capture(item.id, items, interaction, physics, reference)
                        .map(|s| (item.id, s))
                }));
        }
    }
    pub fn capture_changes(
        &mut self,
        reference: &ReferenceManifest,
        items: &mut ItemStore,
        interaction: &InteractionState,
        physics: &PhysicsWorld,
    ) {
        while let Some(id) = items.take_dirty() {
            if self
                .contents
                .active
                .iter()
                .chain(&self.static_controls)
                .any(|c| {
                    !c.restored && (c.item == Some(id) || c.bindings.iter().any(|b| b.item == id))
                })
            {
                continue;
            }
            let Some(mut snapshot) =
                crate::world::changes::capture(id, items, interaction, physics, reference)
            else {
                continue;
            };
            snapshot.lamp = self
                .contents
                .active
                .iter()
                .find(|c| c.item == Some(id))
                .and_then(|c| c.content.portable_lamp());
            let key = items.object_key(id);
            // An unbound dial keeps its saved value.
            snapshot.dial = self.dial_value(id).unwrap_or_else(|| {
                self.changes
                    .get(&key)
                    .and_then(|c| c.snapshot.as_ref())
                    .and_then(|s| s.dial)
            });
            snapshot.radio = self.radio_value(id).or_else(|| {
                self.changes
                    .get(&key)
                    .and_then(|c| c.snapshot.as_ref())
                    .and_then(|s| s.radio)
            });
            if items.get(id).and_then(|i| i.body).is_some_and(|b| {
                physics.rapier().bodies[b].body_type()
                    == rapier3d::prelude::RigidBodyType::KinematicPositionBased
            }) && self
                .contents
                .active
                .iter()
                .any(|c| c.item == Some(id) && !c.content.buried)
            {
                if let Some(previous) = self.changes.get(&key).and_then(|c| c.snapshot.as_ref()) {
                    snapshot.linear_velocity = previous.linear_velocity;
                    snapshot.angular_velocity = previous.angular_velocity;
                    snapshot.sleeping = previous.sleeping;
                }
            }
            let baseline = match &key {
                crate::world::ObjectKey::Imported(_) => self
                    .imported_baselines
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, s)| s),
                crate::world::ObjectKey::Generated(_) => self
                    .contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .find_map(|c| {
                        if c.item == Some(id) {
                            c.baseline.as_ref()
                        } else {
                            c.bindings
                                .iter()
                                .find(|b| b.item == id)
                                .map(|b| &b.baseline)
                        }
                    })
                    .or_else(|| {
                        self.supplies.iter().find_map(|s| {
                            s.items
                                .zip(s.baselines.as_ref())
                                .and_then(|((tank, cap), b)| {
                                    if id == tank {
                                        Some(&b[0])
                                    } else if id == cap {
                                        Some(&b[1])
                                    } else {
                                        None
                                    }
                                })
                        })
                    }),
            };
            let Some(baseline) = baseline else {
                continue;
            };
            if items.get(id).is_some_and(|item| item.removed) {
                self.changes.remove(key.clone());
            } else {
                self.changes.record(key.clone(), snapshot, baseline);
            }
            if let crate::world::ObjectKey::Generated(_) = &key {
                if let Some(owner) = self
                    .contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .find(|c| c.item == Some(id) || c.bindings.iter().any(|b| b.item == id))
                {
                    self.changes
                        .set_replay(&key, &owner.origin, &owner.content.placement.id);
                } else if let Some(supply) = self
                    .supplies
                    .iter()
                    .find(|s| s.items.is_some_and(|(a, b)| a == id || b == id))
                {
                    if let Some(instance) = self
                        .structures
                        .iter()
                        .find(|s| s.placement.id == supply.key)
                    {
                        self.changes
                            .set_replay(&key, &instance.placement, &supply.key);
                    }
                }
            }
        }
    }

    /// Bounds new owner payload, type-fit pairs, and existing vector/map reallocation.
    pub(super) fn binding_reservation(
        &self,
        prototype: &crate::reference::WorldPrototypeReference,
        id: &crate::world::GeneratedId,
        fixed_decoration: bool,
    ) -> Result<(usize, usize), ReferenceError> {
        if fixed_decoration {
            return Ok((0, 0));
        }
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
                "ediblescript",
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
            if instance.fixed_decoration
                || self
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
                self.structures
                    .swap_remove(index)
                    .retire(physics, &mut self.trigger_roles);
                self.structure_revision = self.structure_revision.wrapping_add(1);
                return Err(error);
            }
            crate::world::content_mechanisms::sync(&mut content, items, interaction, physics);
            content.baseline = crate::world::changes::capture(
                content.item.unwrap(),
                items,
                interaction,
                physics,
                reference,
            );
            if let Some(baseline) = &mut content.baseline {
                baseline.lamp = content.content.portable_lamp();
            }
            for binding in &mut content.bindings {
                binding.baseline = crate::world::changes::capture(
                    binding.item,
                    items,
                    interaction,
                    physics,
                    reference,
                )
                .unwrap();
            }
            content.suspended_bodies.reserve(instance.bodies.len());
            self.static_controls.push(content);
            self.static_control_revision += 1;
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
                            physics.edit_body(body, |state| state.set_position(pose, false));
                        }
                    }
                }
            }
            if let Some(item) = content.item {
                if let (Some(state), Some((body, local))) = (items.get(item), content.render_body) {
                    if state.body != Some(body) {
                        let pose = to_rapier(state.pose) * to_rapier(local).inverse();
                        if physics.rapier().bodies[body].position() != &pose {
                            physics.edit_body(body, |state| state.set_position(pose, false));
                        }
                    }
                }
                continue;
            }
            let placement_id = &content.content.placement.id;
            let Some(instance) = self
                .structures
                .iter_mut()
                .find(|i| &i.placement.id == placement_id)
            else {
                continue;
            };
            if instance.fixed_decoration {
                content.restored = true;
                continue;
            }
            let placement = content.content.placement.clone();
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
                let hit = hit_sound(&self.reference, pickup.source);
                let state = items.set_body(id, Some(body)).unwrap();
                state.hit_sound = hit;
                state.fluid = content.content.fluid.clone();
                state.generated_condition = Some(content.content.condition);
                state.local_pose = from_rapier(
                    &(physics.rapier().bodies[body].position().inverse() * to_rapier(state.pose)),
                );
                let state = items.get(id).unwrap();
                let body_pose = to_rapier(state.pose) * to_rapier(state.local_pose).inverse();
                physics.edit_body(body, |state| state.set_position(body_pose, true));
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
                    physics.edit_collider(*handle, |collider| {
                        collider.user_data = ((collider_id.0 as u128) << 64) | (id.0 as u128 + 1);
                        collider.set_active_events(hit_events(hit));
                    });
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
                if let Some(edible) =
                    self.contents
                        .rules
                        .component(&self.reference, pickup_root, "ediblescript")?
                {
                    if let Some((prototype_index, food)) = reference
                        .survival
                        .foods
                        .iter()
                        .enumerate()
                        .find(|(_, food)| &food.source == edible.source)
                    {
                        items.get_mut(id).unwrap().food =
                            Some(crate::interaction::items::FoodState {
                                prototype_index,
                                bite: food.index,
                            });
                        let handles: Vec<_> = instance
                            .colliders
                            .iter()
                            .filter(|(source, _)| {
                                food.colliders.iter().any(|c| &c.source == source)
                            })
                            .map(|(_, handle)| *handle)
                            .collect();
                        interaction.register_consumable(id, food, &handles);
                    }
                }
                crate::world::content_containers::bind(
                    content,
                    instance,
                    prototype,
                    &self.contents.rules,
                    reference,
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
                crate::world::content_mechanisms::sync(content, items, interaction, physics);
                content.suspended_bodies.reserve(instance.bodies.len());
                content.baseline =
                    crate::world::changes::capture(id, items, interaction, physics, reference);
                if let Some(baseline) = &mut content.baseline {
                    baseline.lamp = content.content.portable_lamp();
                }
                for binding in &mut content.bindings {
                    binding.baseline = crate::world::changes::capture(
                        binding.item,
                        items,
                        interaction,
                        physics,
                        reference,
                    )
                    .unwrap();
                }
                let state = items.get(id).unwrap();
                physics.edit_body(body, |body| {
                    body.set_position(
                        to_rapier(state.pose) * to_rapier(state.local_pose).inverse(),
                        true,
                    )
                });
                Ok(())
            })();
            if let Err(error) = result {
                rollback(content, instance, items, interaction, physics, wheels);
                let index = self
                    .structures
                    .iter()
                    .position(|s| s.placement.id == placement.id)
                    .unwrap();
                self.structures
                    .swap_remove(index)
                    .retire(physics, &mut self.trigger_roles);
                self.structure_revision = self.structure_revision.wrapping_add(1);
                return Err(error);
            }
        }
        for content_index in 0..self.contents.active.len() {
            for slot_index in 0..self.contents.active[content_index].slots.len() {
                let content = &self.contents.active[content_index];
                let binding = &content.slots[slot_index];
                let source = &binding.source;
                let slot = binding.slot;
                if binding.initialized {
                    continue;
                }
                if interaction.parts.slot_item(slot).is_some() {
                    self.contents.active[content_index].slots[slot_index].initialized = true;
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
                    if let Some(mut snapshot) =
                        crate::world::changes::capture(item, items, interaction, physics, reference)
                    {
                        for owner in &mut self.contents.active {
                            if owner.item == Some(item) {
                                snapshot.lamp = owner.content.portable_lamp();
                                owner.baseline = Some(snapshot);
                                for binding in &mut owner.bindings {
                                    binding.baseline = crate::world::changes::capture(
                                        binding.item,
                                        items,
                                        interaction,
                                        physics,
                                        reference,
                                    )
                                    .unwrap();
                                }
                                owner.restored = false;
                                break;
                            }
                            if let Some(binding) =
                                owner.bindings.iter_mut().find(|b| b.item == item)
                            {
                                binding.baseline = snapshot;
                                owner.restored = false;
                                break;
                            }
                        }
                    }
                    self.contents.active[content_index].slots[slot_index].initialized = true;
                }
            }
        }
        self.restore_content_changes(reference, items, interaction, physics, wheels)?;
        Ok(())
    }
    fn restore_content_changes(
        &mut self,
        reference: &ReferenceManifest,
        items: &mut ItemStore,
        interaction: &mut InteractionState,
        physics: &mut PhysicsWorld,
        wheels: &mut crate::interaction::wheels::WheelAssemblies,
    ) -> Result<(), ReferenceError> {
        let count = self.contents.active.len() + self.static_controls.len();
        for index in 0..count {
            self.publish_content_visual(index, items);
            let pending_parent = {
                let content = self
                    .contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .nth(index)
                    .unwrap();
                if content.restored || content.item.is_none() {
                    continue;
                }
                content
                    .item
                    .into_iter()
                    .chain(content.bindings.iter().map(|b| b.item))
                    .any(|item| {
                        self.changes
                            .get(items.object_key(item))
                            .and_then(|c| c.snapshot.as_ref())
                            .and_then(|s| s.attachment.as_ref())
                            .and_then(|a| a.owner.as_ref())
                            .and_then(|k| items.resolve_key(k))
                            .is_some_and(|parent| {
                                self.contents
                                    .active
                                    .iter()
                                    .chain(&self.static_controls)
                                    .any(|owner| {
                                        owner.content.placement.id != content.content.placement.id
                                            && !owner.restored
                                            && (owner.item == Some(parent)
                                                || owner.bindings.iter().any(|b| b.item == parent))
                                    })
                            })
                    })
            };
            let content = self
                .contents
                .active
                .iter_mut()
                .chain(&mut self.static_controls)
                .nth(index)
                .unwrap();
            if content.restored || content.item.is_none() {
                continue;
            }
            for (body, enabled) in content.suspended_bodies.drain(..) {
                physics.edit_body(body, |state| state.set_enabled(enabled));
            }
            let mut complete = !pending_parent;
            let mut restore_error = None;
            for item in content
                .item
                .into_iter()
                .chain(content.bindings.iter().map(|b| b.item))
            {
                if pending_parent {
                    break;
                }
                if let Some(change) = self.changes.get(items.object_key(item)) {
                    if let Some(snapshot) = &change.snapshot {
                        match restore_item(
                            item,
                            snapshot,
                            reference,
                            items,
                            interaction,
                            physics,
                            wheels,
                        ) {
                            Ok(ready) => complete &= ready,
                            Err(error) => {
                                complete = false;
                                restore_error = Some(error);
                                break;
                            }
                        }
                    }
                }
            }
            content.restored = complete;
            if let Some(instance) = self
                .structures
                .iter()
                .find(|i| i.placement.id == content.content.placement.id)
            {
                let prototype = reference
                    .world
                    .prototypes
                    .iter()
                    .find(|p| p.source == instance.placement.prototype)
                    .unwrap();
                for (source, handle) in &instance.colliders {
                    let collider = prototype
                        .colliders
                        .iter()
                        .find(|c| &c.source == source)
                        .unwrap();
                    physics.edit_collider(*handle, |state| {
                        state.set_enabled(
                            complete
                                && collider.enabled
                                && crate::world::spawn_appearance::node_active(
                                    prototype,
                                    collider.node as usize,
                                    &content.content.appearance,
                                ),
                        )
                    });
                }
                if !complete {
                    for body in &instance.bodies {
                        physics.edit_body(*body, |body_state| {
                            content
                                .suspended_bodies
                                .push((*body, body_state.is_enabled()));
                            body_state.set_enabled(false);
                        });
                    }
                }
            }
            if complete {
                crate::world::content_mechanisms::sync(content, items, interaction, physics);
            }
            self.publish_content_visual(index, items);
            if let Some(error) = restore_error {
                physics.refresh_queries();
                return Err(error);
            }
        }
        Ok(())
    }
    fn publish_content_visual(&mut self, index: usize, items: &ItemStore) {
        let active_count = self.contents.active.len();
        let (content, slot) = if index < active_count {
            (
                &mut self.contents.active[index],
                super::content_index::ContentSlot::Active(index),
            )
        } else {
            (
                &mut self.static_controls[index - active_count],
                super::content_index::ContentSlot::Control(index - active_count),
            )
        };
        let food = content
            .item
            .and_then(|id| items.get(id))
            .and_then(|item| item.food.map(|food| (food, item.removed)));
        if content.take_visual_change(food) {
            self.presentation_changes.push(slot);
        }
    }

    pub fn bindings_ready(&self, id: &crate::world::GeneratedId) -> bool {
        self.contents
            .active
            .iter()
            .chain(&self.static_controls)
            .find(|c| &c.content.placement.id == id)
            .is_none_or(|c| c.item.is_none() || c.restored)
    }
    fn forget_expired_retired_item_change(&mut self, key: &ObjectKey, radius_squared: f32) {
        let Some(change) = self.changes.get(key) else {
            return;
        };
        if self.saved_item_protection.contains(key)
            || self.changes.has_attachment_dependents(key)
            || change
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.held)
            || changes::external_attachment(change, &self.changes)
            || change.instance.as_ref().is_some_and(|id| {
                self.contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .any(|content| &content.content.placement.id == id)
                    || self.supplies.iter().any(|supply| &supply.key == id)
                    || self.lamp_bindings.iter().any(|lamp| &lamp.parent == id)
            })
            || !changes::roadside_change_expired(
                change,
                [self.request.player, self.request.vehicle],
                radius_squared,
            )
        {
            return;
        }
        if self.changes.forget(key) {
            self.invalidate_changed_selection();
        }
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
            // Replay keeps its source pose until binding applies the destination snapshot.
            // Its caller owns cancellation, including intermediate deferred output.
            if content.replay_only
                && !content.restored
                && (content.item.is_none() || self.contents.replaying(&content.origin.id))
            {
                index += 1;
                continue;
            }
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
            protected |= content.has_external_dependents(items, &interaction.parts, wheels);
            protected |= self
                .saved_item_protection
                .iter()
                .any(|key| items.resolve_key(key).is_some_and(local_item));
            let removed = id
                .and_then(|id| items.get(id))
                .is_some_and(|item| item.removed);
            if (distance < remove * remove || protected) && !removed {
                if let Some(body) = id.and_then(|id| items.get(id)).and_then(|s| s.body) {
                    if !protected
                        && (content.content.buried
                            || distance >= freeze * freeze
                            || distance < unfreeze * unfreeze)
                    {
                        let kind = if content.content.buried || distance >= freeze * freeze {
                            rapier3d::prelude::RigidBodyType::KinematicPositionBased
                        } else {
                            rapier3d::prelude::RigidBodyType::Dynamic
                        };
                        if physics.rapier().bodies[body].body_type() != kind {
                            physics.edit_body(body, |state| state.set_body_type(kind, true));
                            if kind == rapier3d::prelude::RigidBodyType::Dynamic {
                                if let Some(item) = id {
                                    if let Some(snapshot) = self
                                        .changes
                                        .get(items.object_key(item))
                                        .and_then(|c| c.snapshot.as_ref())
                                    {
                                        physics.edit_body(body, |body| {
                                            body.set_linvel(snapshot.linear_velocity.into(), true)
                                        });
                                        physics.edit_body(body, |body| {
                                            body.set_angvel(snapshot.angular_velocity.into(), true)
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                index += 1;
                continue;
            }
            let content = self.contents.remove_active(index);
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
                let key = items.object_key(binding.item);
                wheels.remove_generated(binding.item);
                if let Some(instance) = self
                    .structures
                    .iter()
                    .find(|i| i.placement.id == content.content.placement.id)
                {
                    for (_, collider) in &instance.colliders {
                        interaction.unbind_generated(binding.item, std::slice::from_ref(collider));
                    }
                }
                items.set_body(binding.item, None);
                items.get_mut(binding.item).unwrap().location =
                    crate::interaction::items::ItemLocation::World;
                items.remove_generated(binding.item);
                self.forget_expired_retired_item_change(&key, remove * remove);
            }
            if let Some(id) = content.item {
                let key = items.object_key(id);
                if let Some(instance) = self
                    .structures
                    .iter()
                    .find(|i| i.placement.id == content.content.placement.id)
                {
                    for (_, collider) in &instance.colliders {
                        interaction.unbind_generated(id, std::slice::from_ref(collider));
                    }
                }
                items.set_body(id, None);
                items.get_mut(id).unwrap().location =
                    crate::interaction::items::ItemLocation::World;
                items.remove_generated(id);
                self.forget_expired_retired_item_change(&key, remove * remove);
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
        can_inventory: flag(f, "canInventory")?,
        can_hold_right_hand: flag(f, "canHoldRightHand")?,
        disable_rh_fix: flag(f, "disableRHFix")?,
        right_hand_position: {
            let mut p = vector(f, "RHP")?;
            p[2] = -p[2];
            p
        },
        right_hand_rotation: crate::world::spawn_building::source_euler(vector(f, "RHR")?),
        right_hand_grip: optional(f, "RightHandPos")?.cloned(),
        left_hand_grip: optional(f, "LeftHandPos")?.cloned(),
        disable_when_stored: optional(f, "disableThisWhenStored")?.cloned(),
        keep_when_stored: optional(f, "dontDisableWhenStored")?.cloned(),
        inventory_icon: optional(f, "invImg")?.cloned(),
    })
}

/// Source hit sound of a generated pickup.
pub(crate) fn hit_sound(
    world: &crate::reference::WorldReference,
    pickup: &crate::reference::SourceId,
) -> Option<HitSound> {
    world
        .pickup_hits
        .iter()
        .find(|h| &h.source == pickup)
        .map(|h| HitSound {
            clip: h.collision_clip,
            gain: h.collision_gain,
        })
}
/// Only pickups with a hit sound report contacts.
pub(crate) fn hit_events(sound: Option<HitSound>) -> ActiveEvents {
    if sound.is_some() {
        ActiveEvents::COLLISION_EVENTS
    } else {
        ActiveEvents::empty()
    }
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
        if let Some(state) = items.set_body(item, None) {
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
        physics.edit_collider(*collider, |collider| {
            collider.user_data = 0;
            collider.set_active_events(ActiveEvents::empty());
        });
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

pub(crate) fn restore_item(
    id: crate::reference::ItemId,
    snapshot: &crate::world::ObjectSnapshot,
    reference: &ReferenceManifest,
    items: &mut ItemStore,
    interaction: &mut InteractionState,
    physics: &mut PhysicsWorld,
    wheels: &mut crate::interaction::wheels::WheelAssemblies,
) -> Result<bool, ReferenceError> {
    use crate::interaction::items::ItemLocation;
    let owner = snapshot.attachment.as_ref().and_then(|a| a.owner.as_ref());
    let resolved = owner.and_then(|key| items.resolve_key(key));
    if owner.is_some() && resolved.is_none() {
        return Ok(false);
    }
    let slot = snapshot.attachment.as_ref().and_then(|a| a.slot.as_ref());
    let target_slot = slot.and_then(|source| interaction.parts.resolve_slot(resolved, source));
    if slot.is_some()
        && target_slot.is_none()
        && !slot.is_some_and(|source| reference.wheel_slots.iter().any(|s| &s.source == source))
    {
        return Ok(false);
    }
    let current = items.get(id).unwrap().location;
    if let ItemLocation::PartSlot(current_slot) = current {
        if target_slot != Some(current_slot) {
            interaction.parts.remove(id, items, physics)?;
            interaction
                .parts
                .detach_mechanism(id, &mut interaction.mechanisms, physics);
        }
    }
    if let ItemLocation::Attached(parent) = current {
        if resolved != Some(parent) && wheels.rules().iter().any(|r| r.item == id) {
            wheels.split(parent, items, physics)?;
        }
    }
    if let Some(slot) = target_slot {
        interaction.parts.validate_saved_slot(id, slot)?;
        interaction
            .parts
            .initialize_generated_slot(id, slot, items, physics)?;
        interaction
            .parts
            .mount_mechanism(id, &mut interaction.mechanisms);
    } else if let Some(wheel) =
        slot.and_then(|source| reference.wheel_slots.iter().find(|s| &s.source == source))
    {
        if items.get(id).unwrap().location != ItemLocation::WheelSlot(wheel.id) {
            wheels.install(id, wheel.id, items, physics)?;
        }
    } else if let Some(parent) = resolved {
        if slot.is_none()
            && items.get(id).unwrap().location != ItemLocation::Attached(parent)
            && wheels.rules().iter().any(|r| r.item == id)
        {
            wheels.assemble(parent, id, items, physics)?;
        }
    }
    if let (Some(parent_id), Some(attachment)) = (resolved, &snapshot.attachment) {
        if attachment.slot.is_none() && !wheels.rules().iter().any(|r| r.item == id) {
            let parent = items.get(parent_id).unwrap();
            let Some(parent_body) = parent.body else {
                return Ok(false);
            };
            let item = items.get(id).unwrap();
            let Some(old_body) = item.body else {
                return Ok(false);
            };
            if old_body != parent_body {
                if items
                    .iter()
                    .any(|other| other.body == Some(old_body) && !items.attached_to(other.id, id))
                {
                    return Err(ReferenceError(
                        "Saved attachment shares an unrelated body".into(),
                    ));
                }
                let local = crate::vehicle::VehiclePose {
                    position: attachment.local_pose.position.map(|v| v as f32),
                    rotation: attachment.local_pose.rotation,
                };
                let transform = to_rapier(parent.local_pose)
                    * to_rapier(local)
                    * to_rapier(item.local_pose).inverse();
                let colliders = physics.rapier().bodies[old_body].colliders().to_vec();
                for collider in colliders {
                    let local = *physics.rapier().colliders[collider]
                        .position_wrt_parent()
                        .unwrap();
                    physics.set_collider_parent(collider, Some(parent_body));
                    physics.edit_collider(collider, |state| {
                        state.set_position_wrt_parent(transform * local)
                    });
                }
                let pose = *physics.rapier().bodies[parent_body].position();
                let children: Vec<_> = items
                    .iter()
                    .filter(|i| i.body == Some(old_body))
                    .map(|i| i.id)
                    .collect();
                for child_id in children {
                    let child = items.set_body(child_id, Some(parent_body)).unwrap();
                    child.local_pose = from_rapier(&(transform * to_rapier(child.local_pose)));
                    child.pose = from_rapier(&(pose * to_rapier(child.local_pose)));
                    child.inactive_pose = None;
                }
                physics.edit_body(old_body, |body| body.set_enabled(false));
            }
        }
    }
    match (&snapshot.food, items.get(id).and_then(|i| i.food)) {
        (Some(saved), Some(food)) => {
            let recipe = &reference.survival.foods[food.prototype_index];
            if saved.source != recipe.source || saved.bite > recipe.bites {
                return Err(ReferenceError(
                    "Saved edible recipe or bite is invalid".into(),
                ));
            }
            items.get_mut(id).unwrap().food.as_mut().unwrap().bite = saved.bite;
        }
        (None, None) => (),
        _ => {
            return Err(ReferenceError(
                "Saved edible owner differs from source".into(),
            ));
        }
    }
    items.restore_generated(id, snapshot)?;
    items.get_mut(id).unwrap().pending_attachment = false;
    if snapshot.held {
        items.get_mut(id).unwrap().location = ItemLocation::Held;
        if let Some(handle) = items.get(id).and_then(|i| i.body) {
            physics.edit_body(handle, |body| {
                if body.is_kinematic() {
                    body.set_body_type(rapier3d::prelude::RigidBodyType::Dynamic, false);
                }
            });
        }
    }
    if let Some(valve) = snapshot.valve {
        interaction.restore_generated_valve(id, valve)?;
    }
    if let Some(mechanism) = snapshot.mechanism {
        interaction.mechanisms.restore(id, mechanism)?;
    }
    if let (Some(parent), Some(attachment)) = (resolved, &snapshot.attachment) {
        let parent = items.get(parent).unwrap().clone();
        let local = crate::vehicle::VehiclePose {
            position: attachment.local_pose.position.map(|v| v as f32),
            rotation: attachment.local_pose.rotation,
        };
        let item = items.get_mut(id).unwrap();
        if attachment.slot.is_none() {
            item.location = ItemLocation::Attached(parent.id);
        }
        item.pose = from_rapier(&(to_rapier(parent.pose) * to_rapier(local)));
        item.local_pose = from_rapier(&(to_rapier(parent.local_pose) * to_rapier(local)));
    } else {
        crate::world::changes::restore_motion(id, snapshot, items, physics);
    }
    Ok(true)
}
