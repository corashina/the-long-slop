use super::*;
use crate::{
    interaction::items::ItemLocation,
    inventory::{InventoryCommand, InventoryContext},
};
use rapier3d::math::{Pose, Rotation, Vector};

impl StartingArea {
    pub fn sync_inventory(&mut self) {
        for slot in &mut self.inventory.slots {
            if slot.is_some_and(|id| {
                self.items.get(id).is_none_or(|i| {
                    i.removed
                        || !matches!(
                            i.location,
                            ItemLocation::Stored(_) | ItemLocation::Held | ItemLocation::Equipped
                        )
                })
            }) {
                *slot = None;
            }
        }
        if let Some(id) = self.interaction.held() {
            if self.inventory.slots[self.inventory.selected as usize].is_none()
                && self.inventory_items.contains(&id)
                && !self.inventory.slots.contains(&Some(id))
            {
                self.inventory.slots[self.inventory.selected as usize] = Some(id);
            }
        }
    }
    pub fn inventory_context(&self, physics: &PhysicsWorld, input: FrameInput) -> InventoryContext {
        let carried = self.interaction.held();
        let ancestor_carry = self.player.seat_vehicle().is_some_and(|index| {
            carried
                .and_then(|id| self.items.get(id))
                .and_then(|i| i.body)
                == Some(physics.chassis_handle(index))
        });
        InventoryContext {
            carried,
            equipped: self.interaction.equipment.item(),
            ancestor_carry,
            seated_clutch: self.player.seat().is_some() && input.held & keys::CLUTCH != 0,
        }
    }
    pub fn apply_inventory_command(
        &mut self,
        command: InventoryCommand,
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
    ) -> Result<bool, ReferenceError> {
        let index = match command {
            InventoryCommand::Select(i) | InventoryCommand::Swap(i) => i as usize,
        };
        if index >= 4 {
            return Ok(false);
        }
        let selected = self.inventory.selected as usize;
        let outgoing = self.inventory.slots[selected];
        let incoming = self.inventory.slots[index];
        let held = self.interaction.held();
        let equipped = self.interaction.equipment.item();
        let drop_carry = matches!(command, InventoryCommand::Select(_))
            && incoming.is_some()
            && held.is_some_and(|id| {
                self.interaction
                    .pickup_rule(id)
                    .is_some_and(|p| !p.can_inventory)
            });
        let carried_after = held.filter(|id| Some(*id) != outgoing && !drop_carry);
        let equipped_after = equipped.filter(|id| Some(*id) != outgoing);
        if self
            .inventory_context(physics, FrameInput::default())
            .ancestor_carry
            || (held.is_some() && !drop_carry && equipped.is_some() && incoming.is_some())
        {
            return Ok(false);
        }
        // Validate both ends before releasing a joint, body or slot.
        for id in outgoing.into_iter().chain(incoming) {
            let Some(item) = self.items.get(id) else {
                return Ok(false);
            };
            let Some(body) = item.body.and_then(|h| physics.rapier().bodies.get(h)) else {
                return Ok(false);
            };
            if item.removed
                || !self.inventory_items.contains(&id)
                || !(body.is_dynamic() || item.location == ItemLocation::Equipped)
                || item.inactive_pose.is_some()
            {
                return Ok(false);
            }
            if Some(id) == outgoing {
                if !(held == Some(id) || equipped == Some(id)) {
                    return Ok(false);
                }
            } else if item.location != ItemLocation::Stored(index as u8) {
                return Ok(false);
            }
        }
        let to_hand = incoming.is_some_and(|id| {
            self.interaction
                .pickup_rule(id)
                .unwrap()
                .can_hold_right_hand
        }) && equipped_after.is_none();
        if incoming.is_some() && !to_hand && carried_after.is_some() {
            return Ok(false);
        }
        if drop_carry {
            self.interaction.drop_held(&mut self.items, physics);
        }
        if let Some(id) = outgoing {
            let storage = to_rapier(self.pose.inventory_pose(self.player.body_pose(physics)));
            let body = self.items.get(id).unwrap().body.unwrap();
            self.inventory_offsets[selected] = Some((
                id,
                crate::vehicle::from_rapier(
                    &(storage.inverse() * physics.rapier().bodies[body].position()),
                ),
            ));
            if self.interaction.held() == Some(id) {
                self.interaction.carry.drop(&mut self.items, physics);
            }
            if self.interaction.equipment.item() == Some(id) {
                self.interaction.equipment.release(&mut self.items, physics);
            }
            let item = self.items.get_mut(id).unwrap();
            item.location = ItemLocation::Stored(selected as u8);
            physics.edit_body(item.body.unwrap(), |body| body.set_enabled(false));
        }
        match command {
            InventoryCommand::Select(i) => {
                self.inventory.selected = i;
                self.inventory.swap_progress = 0.;
            }
            InventoryCommand::Swap(_) => {
                self.inventory.slots.swap(selected, index);
                self.inventory_offsets.swap(selected, index);
                if index != selected
                    && let Some(id) = self.inventory.slots[index]
                {
                    self.items.get_mut(id).unwrap().location = ItemLocation::Stored(index as u8);
                }
            }
        }
        if let Some(id) = self.inventory.slots[self.inventory.selected as usize] {
            self.inventory_offsets[self.inventory.selected as usize] = None;
            let body = self.items.get(id).unwrap().body.unwrap();
            physics.edit_body(body, |body| body.set_enabled(true));
            self.items.get_mut(id).unwrap().location = ItemLocation::World;
            if to_hand {
                self.interaction
                    .equipment
                    .retrieve(id, &mut self.items, physics);
            } else {
                let rule = self.interaction.pickup_rule(id).unwrap();
                let view = to_rapier(self.player.view_pose(physics));
                let offset = if rule.use_recenter_position {
                    Vector::from_array(rule.recenter_position)
                } else {
                    Vector::from_array(reference.player.carry_anchor_offset)
                        * rule.recenter_distance
                };
                let rotation = if rule.use_recenter_rotation {
                    Rotation::from_array(rule.recenter_rotation)
                } else {
                    Rotation::IDENTITY
                };
                let target = Pose::from_parts(
                    view.translation
                        + view.rotation
                            * (Vector::from_array(reference.player.carry_anchor_offset) + offset),
                    view.rotation * rotation,
                ) * to_rapier(self.items.get(id).unwrap().local_pose).inverse();
                physics.edit_body(body, |body| body.set_position(target, false));
                self.interaction
                    .pickup_item(
                        id,
                        self.player.view_pose(physics).position,
                        &self.player,
                        &mut self.items,
                        physics,
                    )
                    .expect("validated inventory retrieval");
            }
        }
        self.sync_inventory();
        self.sync_equipment(reference, physics);
        physics.refresh_queries();
        Ok(true)
    }
    pub fn sync_equipment(&mut self, reference: &ReferenceManifest, physics: &mut PhysicsWorld) {
        self.pose.set_frames(
            self.player.view_pose(physics),
            self.player.body_pose(physics),
        );
        self.pose.update(
            &self.player,
            &self.interaction,
            &self.items,
            &reference.clothing,
            FIXED_DT,
        );
        if let Some(id) = self.interaction.equipment.item() {
            self.interaction.equipment.sync(
                &self.pose,
                &self.items,
                self.interaction.pickup_rule(id).unwrap(),
                physics,
            );
        }
        let storage = to_rapier(self.pose.inventory_pose(self.player.body_pose(physics)));
        for (slot, id) in self.inventory.slots.iter().enumerate() {
            let Some(id) = id else { continue };
            let item = self.items.get(*id).unwrap();
            if matches!(item.location, ItemLocation::Stored(_))
                && let Some(h) = item.body
            {
                let frame = self.inventory_offsets[slot].get_or_insert_with(|| {
                    (
                        *id,
                        crate::vehicle::from_rapier(
                            &(storage.inverse() * physics.rapier().bodies[h].position()),
                        ),
                    )
                });
                physics.edit_body(h, |body| {
                    body.set_position(storage * to_rapier(frame.1), false);
                    body.set_enabled(false);
                });
            }
        }
    }
    pub fn inventory_body(&self, handle: RigidBodyHandle) -> Option<ItemLocation> {
        self.inventory
            .slots
            .iter()
            .flatten()
            .filter_map(|id| self.items.get(*id))
            .find(|i| i.body == Some(handle))
            .map(|i| i.location)
    }
}
impl GarageRuntime {
    pub(super) fn update_inventory(&mut self, reference: &ReferenceManifest, input: FrameInput) {
        self.area.sync_inventory();
        let context = self.area.inventory_context(&self.physics, input);
        if let Some(command) = self.area.inventory.step(input, FIXED_DT, context) {
            self.area
                .apply_inventory_command(command, reference, &mut self.physics)
                .expect("checked inventory bindings");
            // InventoryStuff processes key-up selection before the held-key phase.
            if matches!(command, InventoryCommand::Select(_)) {
                let context = self.area.inventory_context(&self.physics, input);
                if let Some(command) = self.area.inventory.step(
                    FrameInput {
                        released: 0,
                        ..input
                    },
                    FIXED_DT,
                    context,
                ) {
                    self.area
                        .apply_inventory_command(command, reference, &mut self.physics)
                        .expect("checked inventory bindings");
                }
            }
        }
    }
}

impl StartingArea {
    pub fn capture_inventory(
        &self,
        ids: &crate::save::IdentityMap,
    ) -> Result<crate::save::InventorySnapshot, crate::save::SaveError> {
        let mut slots = [None, None, None, None];
        for (saved, id) in slots.iter_mut().zip(self.inventory.slots) {
            *saved = id.map(|id| ids.item_instance(id)).transpose()?;
        }
        Ok(crate::save::InventorySnapshot {
            slots,
            selected: self.inventory.selected,
            equipped: self
                .interaction
                .equipment
                .item()
                .map(|id| ids.item_instance(id))
                .transpose()?,
        })
    }
}
