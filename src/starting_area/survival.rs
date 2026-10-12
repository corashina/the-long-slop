use super::GarageRuntime;
use crate::fluids::{contact_drink_transfer, drink_transfer};
use crate::interaction::items::ItemLocation;
use crate::reference::{ConsumptionEffect, ItemId, ReferenceError, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, VehiclePose, collider_builder, to_rapier};
use rapier3d::prelude::{Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder};

pub(crate) fn spawn_consumable(
    reference: &ReferenceManifest,
    area: &mut super::StartingArea,
    physics: &mut PhysicsWorld,
    id: ItemId,
    prototype_index: usize,
    pose: VehiclePose,
) -> Result<(), ReferenceError> {
    let prototype = reference
        .survival
        .foods
        .get(prototype_index)
        .ok_or_else(|| ReferenceError("Missing food prototype".into()))?;
    if area.items.get(id).is_some() {
        return Err(ReferenceError("Duplicate food item".into()));
    }
    let root = to_rapier(super::node_pose(reference, prototype.root)?);
    let body = physics.rapier_mut().insert_body(
        RigidBodyBuilder::dynamic()
            .pose(to_rapier(pose))
            .ccd_enabled(true)
            .additional_mass(prototype.mass_kg)
            .linear_damping(prototype.linear_damping)
            .angular_damping(prototype.angular_damping),
    );
    let mut handles = Vec::with_capacity(prototype.colliders.len());
    for record in &prototype.colliders {
        let node = to_rapier(super::node_pose(reference, record.node)?);
        let mut collider = collider_builder(&record.geometry)?
            .sensor(record.sensor)
            .enabled(record.enabled)
            .density(0.)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(record.membership),
                Group::from_bits_truncate(record.filter),
                InteractionTestMode::And,
            ))
            .user_data(id.0 as u128 + 1)
            .build();
        collider.set_position(root.inverse() * node * *collider.position());
        let rapier = physics.rapier_mut();
        handles.push(
            rapier
                .colliders
                .insert_with_parent(collider, body, &mut rapier.bodies),
        );
    }
    area.items
        .insert_consumable(id, prototype_index, prototype, pose, body)?;
    area.interaction
        .register_consumable(id, prototype, &handles);
    area.item_render_poses.push((id, pose, pose));
    physics.refresh_queries();
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrinkResult {
    pub requested_liters: f32,
    pub debited_liters: f32,
    pub effect: ConsumptionEffect,
}

impl GarageRuntime {
    pub fn apply_toilet_output(
        &mut self,
        reference: &ReferenceManifest,
        output: crate::survival::ToiletOutput,
    ) -> Result<Option<ItemId>, ReferenceError> {
        if output.defecated {
            self.spawn_turd(reference).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn spawn_turd(&mut self, reference: &ReferenceManifest) -> Result<ItemId, ReferenceError> {
        let prototype_index = reference
            .survival
            .foods
            .iter()
            .position(|food| food.kind == crate::reference::FoodKind::Turd)
            .ok_or_else(|| ReferenceError("Missing turd prototype".into()))?;
        let id = self.stream.allocate_consumable_id(&self.area.items);
        let mut pose = self.area.player.body_pose(&self.physics);
        pose.rotation = [0., 0., 0., 1.];
        spawn_consumable(
            reference,
            &mut self.area,
            &mut self.physics,
            id,
            prototype_index,
            pose,
        )?;
        Ok(id)
    }
    pub fn consume(
        &mut self,
        reference: &ReferenceManifest,
        item: ItemId,
    ) -> Result<bool, ReferenceError> {
        if !self.survival.output().can_eat {
            return Ok(false);
        }
        let Some(state) = self.area.items.get(item) else {
            return Err(ReferenceError("Missing food item".into()));
        };
        let Some(food) = state.food else {
            return Ok(false);
        };
        if state.removed
            || state.body.is_none()
            || food.bite >= reference.survival.foods[food.prototype_index].bites
        {
            return Ok(false);
        }
        let prototype = &reference.survival.foods[food.prototype_index];
        let divisor = prototype.bites as f32;
        self.survival.queue(ConsumptionEffect {
            food: prototype.effect.food / divisor,
            water: prototype.effect.water / divisor,
            bowel: prototype.effect.bowel / divisor,
            hp: prototype.effect.hp / divisor,
        });
        self.survival.show_stats();
        let state = self.area.items.get_mut(item).unwrap();
        state.food.as_mut().unwrap().bite += 1;
        if state.food.unwrap().bite == prototype.bites && prototype.destroy_on_end {
            self.remove_consumable(item)?;
        }
        Ok(true)
    }

    pub fn drink(
        &mut self,
        reference: &ReferenceManifest,
        source: ItemId,
        requested_liters: f32,
    ) -> Result<DrinkResult, ReferenceError> {
        if !requested_liters.is_finite() || requested_liters < 0. {
            return Err(ReferenceError("Invalid drink volume".into()));
        }
        if !reference
            .tanks
            .iter()
            .any(|tank| tank.item == source && !tank.no_drink)
        {
            return Err(ReferenceError("Source cannot be drunk".into()));
        }
        self.apply_drink_request(reference, source, requested_liters)
    }

    pub fn drink_contact(
        &mut self,
        reference: &ReferenceManifest,
        source: ItemId,
        poured_liters: f32,
        fractions: [f32; 6],
    ) -> Result<DrinkResult, ReferenceError> {
        if !poured_liters.is_finite()
            || poured_liters < 0.
            || !fractions
                .iter()
                .all(|value| value.is_finite() && *value >= 0.)
        {
            return Err(ReferenceError("Invalid particle drink".into()));
        }
        if self
            .area
            .items
            .get(source)
            .and_then(|state| state.fluid.as_ref())
            .is_none()
        {
            return Err(ReferenceError("Missing particle source".into()));
        }
        if self.survival.bladder.mixture.total() >= self.survival.bladder.capacity_liters {
            return Ok(DrinkResult {
                requested_liters: poured_liters,
                debited_liters: poured_liters,
                effect: ConsumptionEffect::default(),
            });
        }
        let needs = self.survival.output().needs;
        let filter_water = needs.water < reference.survival.maximum.water;
        let filter_blood = needs.hp < reference.survival.maximum.hp;
        let before = self.survival.pending();
        self.survival
            .apply_drink(&reference.survival, fractions, poured_liters);
        let after = self.survival.pending();
        contact_drink_transfer(
            &mut self.survival.bladder,
            self.area.items.get(source).unwrap().fluid.as_ref().unwrap(),
            fractions,
            poured_liters,
            filter_water,
            filter_blood,
        );
        Ok(DrinkResult {
            requested_liters: poured_liters,
            debited_liters: poured_liters,
            effect: ConsumptionEffect {
                food: after.food - before.food,
                water: after.water - before.water,
                bowel: after.bowel - before.bowel,
                hp: after.hp - before.hp,
            },
        })
    }

    fn apply_drink_request(
        &mut self,
        reference: &ReferenceManifest,
        source: ItemId,
        requested_liters: f32,
    ) -> Result<DrinkResult, ReferenceError> {
        if !requested_liters.is_finite() || requested_liters < 0. {
            return Err(ReferenceError("Invalid drink volume".into()));
        }
        let Some(state) = self.area.items.get(source) else {
            return Err(ReferenceError("Missing drink source".into()));
        };
        let Some(container) = state.fluid.as_ref() else {
            return Err(ReferenceError("Drink source has no fluid".into()));
        };
        if state.removed
            || container.mixture.total() <= 0.
            || self.survival.bladder.mixture.total() >= self.survival.bladder.capacity_liters
        {
            return Ok(DrinkResult {
                requested_liters,
                debited_liters: 0.,
                effect: ConsumptionEffect::default(),
            });
        }
        let total = container.mixture.total();
        let fractions = container
            .mixture
            .quantities_liters
            .map(|amount| amount / total);
        let needs = self.survival.output().needs;
        let filter_water = needs.water < reference.survival.maximum.water;
        let filter_blood = needs.hp < reference.survival.maximum.hp;
        let before = self.survival.pending();
        self.survival
            .apply_drink(&reference.survival, fractions, requested_liters);
        let after = self.survival.pending();
        let debited_liters = drink_transfer(
            self.area
                .items
                .get_mut(source)
                .unwrap()
                .fluid
                .as_mut()
                .unwrap(),
            &mut self.survival.bladder,
            requested_liters,
            filter_water,
            filter_blood,
        );
        Ok(DrinkResult {
            requested_liters,
            debited_liters,
            effect: ConsumptionEffect {
                food: after.food - before.food,
                water: after.water - before.water,
                bowel: after.bowel - before.bowel,
                hp: after.hp - before.hp,
            },
        })
    }

    pub fn remove_consumable(&mut self, item: ItemId) -> Result<(), ReferenceError> {
        let state = self
            .area
            .items
            .get(item)
            .ok_or_else(|| ReferenceError("Missing food item".into()))?;
        if state.food.is_none() || state.removed {
            return Err(ReferenceError("Food item is not active".into()));
        }
        let body = state
            .body
            .ok_or_else(|| ReferenceError("Food body is missing".into()))?;
        if self.area.interaction.held() == Some(item) {
            self.area
                .interaction
                .drop_held(&mut self.area.items, &mut self.physics);
        }
        self.area.interaction.remove_consumable(item, body);
        let body_id = self.area.items.body_id(item).unwrap();
        self.physics.rapier_mut().remove_body(body);
        self.physics.refresh_queries();
        self.area
            .attachments
            .retain(|(handle, parent, _)| *handle != body && *parent != body);
        self.area
            .portable_mass
            .retain(|(handle, _, _, _)| *handle != body);
        if body_id.0 != u32::MAX {
            self.area.bodies.remove(&body_id);
            self.area.render_poses.retain(|(id, _, _)| *id != body_id);
            self.area
                .compound_colliders
                .retain(|collider| collider.body != body_id);
        }
        self.area.item_render_poses.retain(|(id, _, _)| *id != item);
        let state = self.area.items.get_mut(item).unwrap();
        state.body = None;
        state.location = ItemLocation::World;
        state.removed = true;
        Ok(())
    }
}
