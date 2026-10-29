//! Generated general parts join the existing source slot and mechanism owners.
use super::contents::ContentInstance;
use super::spawn_rules::*;
use crate::interaction::{InteractionState, items::ItemStore};
use crate::reference::*;
use crate::vehicle::{PhysicsWorld, STEP_SURFACE_MASK};
pub(super) fn bind(
    content: &mut ContentInstance,
    instance: &super::StructureInstance,
    prototype: &WorldPrototypeReference,
    rules: &SourceRules,
    reference: &ReferenceManifest,
    pickup: Option<PickupReference>,
    layers: u32,
    items: &mut ItemStore,
    interaction: &mut InteractionState,
    physics: &mut PhysicsWorld,
) -> Result<(), ReferenceError> {
    let owner = content.item.unwrap();
    if !prototype.nodes.iter().any(|n| {
        rules
            .component(&reference.world, &n.source, "partscript")
            .ok()
            .flatten()
            .is_some()
            || rules
                .component(&reference.world, &n.source, "partslotscript")
                .ok()
                .flatten()
                .is_some()
    }) {
        return Ok(());
    }
    if content.nodes.is_empty() {
        let state = items.get(owner).unwrap();
        content.nodes = interaction.mechanisms.register_generated(
            prototype,
            state.pose,
            content.content.placement.scale,
            owner,
            0,
        )?;
    }
    for (part_node, node) in prototype.nodes.iter().enumerate() {
        if !super::spawn_appearance::node_active(prototype, part_node, &content.content.appearance)
        {
            continue;
        }
        let root = &node.source;
        let Some(part) = rules.component(&reference.world, root, "partscript")? else {
            continue;
        };
        if rules
            .component(&reference.world, root, "wheelscript")?
            .is_some()
            || rules
                .component(&reference.world, root, "gumiscript")?
                .is_some()
        {
            continue;
        }
        let strings = |name| {
            array(part.fields, name)?
                .iter()
                .map(|v| {
                    if let WorldValue::Text(s) = v {
                        Ok(s.clone())
                    } else {
                        Err(ReferenceError("Invalid source part type".into()))
                    }
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let main_types = strings("maintipus")?;
        let part_types = strings("tipus")?;
        if !reference.part_slots.iter().any(|slot| {
            (slot.main_types.is_empty()
                || main_types.is_empty()
                || slot.main_types.iter().any(|s| main_types.contains(s)))
                && slot.part_types.iter().any(|s| part_types.contains(s))
        }) {
            continue;
        }

        let pickup_record = rules.get(&reference.world, link(part.fields, "p")?)?;
        // Car roots have no pickupable. Only a root that is itself a part needs one.
        let pickup = if part_node == 0 {
            pickup
                .clone()
                .ok_or_else(|| ReferenceError("Generated part owner has no pickupable".into()))?
        } else {
            super::stream::content_bindings::pickup_rule(pickup_record)?
        };
        let pickup_root = rules.root(&reference.world, &pickup.source)?;
        let pickup_node = prototype
            .nodes
            .iter()
            .position(|n| &n.source == pickup_root)
            .ok_or_else(|| ReferenceError("Generated part pickup root missing".into()))?;
        let body_index = prototype
            .bodies
            .iter()
            .position(|b| b.node as usize == pickup_node)
            .ok_or_else(|| ReferenceError("Generated part body missing".into()))?;
        let body = instance.bodies[body_index];
        let item = if part_node == 0 {
            owner
        } else {
            let mut key = content.content.placement.id.clone();
            key.children.push((part.source.clone(), 0));
            let pose = super::placement::place_pose(
                content.content.placement.pose,
                content.content.placement.scale,
                super::placement::node_pose(prototype, pickup_node),
            );
            let item = items.insert_generated(key, prototype, pose)?;
            let hit = super::stream::content_bindings::hit_sound(&reference.world, &pickup.source);
            let state = items.set_body(item, Some(body)).unwrap();
            state.hit_sound = hit;
            state.local_pose = crate::vehicle::from_rapier(
                &(physics.rapier().bodies[body].position().inverse()
                    * crate::vehicle::to_rapier(state.pose)),
            );
            let local = state.local_pose;
            content.bindings.push(super::contents::ContentBinding {
                source: part.source.clone(),
                item,
                baseline: items.generated_snapshot(item, None).unwrap(),
                render_body: Some((body, local)),
            });
            let mut layers = 0;
            for (source, handle) in &instance.colliders {
                let c = prototype
                    .colliders
                    .iter()
                    .find(|c| c.source == *source)
                    .unwrap();
                if c.body != Some(body_index as u32) {
                    continue;
                }
                layers |= 1 << prototype.nodes[c.node as usize].layer;
                let id = interaction.allocate_generated_collider()?;
                physics.edit_collider(*handle, |collider| {
                    collider.user_data = (collider.user_data & STEP_SURFACE_MASK)
                        | ((id.0 as u128) << 64)
                        | (item.0 as u128 + 1);
                    collider.set_active_events(super::stream::content_bindings::hit_events(hit));
                });
                interaction.bind_surface(
                    *handle,
                    ActionBinding {
                        collider: id,
                        use_owner: None,
                        pickup_owner: Some(item),
                        removal_owner: Some(item),
                        push_owner: None,
                    },
                );
            }
            interaction.register_generated_pickup(item, pickup.clone(), layers, false);
            interaction
                .mechanisms
                .register_generated_item(item, content.nodes[pickup_node]);

            item
        };
        let linked_nodes = |name| {
            array(part.fields, name)?
                .iter()
                .map(|v| {
                    let source = rules.root(&reference.world, value_link(v)?)?;
                    prototype
                        .nodes
                        .iter()
                        .position(|n| &n.source == source)
                        .map(|i| content.nodes[i])
                        .ok_or_else(|| {
                            ReferenceError("Generated part activation target missing".into())
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let shape_handles: Vec<_> = instance
            .colliders
            .iter()
            .filter_map(|(_, handle)| {
                let c = &physics.rapier().colliders[*handle];
                (c.parent() == Some(body))
                    .then_some((ColliderId((c.user_data >> 64) as u32), *handle))
            })
            .collect();
        let rigid = &physics.rapier().bodies[body];
        let mass = rigid.mass();
        let recipe = PartReference {
            scan_radius: number(part.fields, "scanRadius")?,
            usable: None,
            item,
            source: part.source.clone(),
            node: content.nodes[part_node],
            main_types,
            part_types,
            cover_slot: None,
            local_rotation_after_mount: super::spawn_building::source_euler(vector(
                part.fields,
                "localRotationAfterMount",
            )?),
            colliders: shape_handles.iter().map(|c| c.0).collect(),
            linear_damping: rigid.linear_damping(),
            angular_damping: rigid.angular_damping(),
            enable_if_mounted: linked_nodes("enableIfMounted")?,
            enable_if_dismounted: linked_nodes("enableIfDismounted")?,
            disable_if_mounted: linked_nodes("disableIfMounted")?,
            disable_if_dismounted: linked_nodes("disableIfDismounted")?,
            fall_off_category: fall_off_category(part.fields)?,
        };
        let pose = interaction.mechanisms.node_pose(recipe.node).unwrap();
        let mut members = vec![(item, content.nodes[pickup_node], mass)];
        for binding in &content.bindings {
            if binding.item == item || binding.render_body.is_some() {
                continue;
            }
            let root = rules.root(&reference.world, &binding.source)?;
            let node = prototype
                .nodes
                .iter()
                .position(|n| &n.source == root)
                .ok_or_else(|| ReferenceError("Generated part member missing".into()))?;
            if nearest_owner(content, prototype, rules, reference, node)? == Some(item) {
                members.push((binding.item, content.nodes[node], 0.));
            }
        }
        interaction.parts.register_generated(
            recipe,
            pickup,
            body,
            &shape_handles,
            pose,
            members,
            mass,
            items,
            physics,
        )?;
        interaction.register_generated_part_kind(item, layers);
    }
    Ok(())
}

pub(super) fn nearest_owner(
    content: &ContentInstance,
    prototype: &WorldPrototypeReference,
    rules: &SourceRules,
    reference: &ReferenceManifest,
    node: usize,
) -> Result<Option<ItemId>, ReferenceError> {
    let mut node = Some(node);
    while let Some(i) = node {
        if i == 0 {
            return Ok(content.item);
        }
        if let Some(part) =
            rules.component(&reference.world, &prototype.nodes[i].source, "partscript")?
        {
            if let Some(binding) = content.bindings.iter().find(|b| &b.source == part.source) {
                return Ok(Some(binding.item));
            }
        }
        node = prototype.nodes[i].parent.map(|n| n as usize);
    }
    Ok(content.item)
}

/// Source `partscript.fallOffCategory`: a `mainscript.defFallOffChance` index.
pub(super) fn fall_off_category<'a>(f: impl FieldAccess<'a>) -> Result<u8, ReferenceError> {
    u8::try_from(integer(f, "fallOffCategory")?)
        .ok()
        .filter(|c| usize::from(*c) < FALL_OFF_CATEGORIES)
        .ok_or_else(|| ReferenceError("Invalid source fallOffCategory".into()))
}
