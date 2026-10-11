//! Collider identity and source action owners.
use super::items::{ItemLocation, ItemStore};
use super::{AimTarget, InteractionState, SLOT_SENSOR_FLAG, SlotTarget};
use crate::reference::{ActionBinding, ColliderId, ItemId, ItemKind, WheelSlotId};
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use rapier3d::math::Vector;
use rapier3d::prelude::{ColliderHandle, QueryFilter, Ray};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionBlock {
    Covered,
    Incompatible,
    Occupied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedAim {
    pub use_target: Option<AimTarget>,
    pub grab_target: Option<AimTarget>,
    pub installation_target: Option<AimTarget>,
    pub filling_target: Option<AimTarget>,
    pub push_target: Option<AimTarget>,
    pub focus_target: Option<AimTarget>,
    pub grab_block: Option<InteractionBlock>,
    pub installation_block: Option<InteractionBlock>,
}

impl ResolvedAim {
    pub fn same_actions(&self, other: &Self) -> bool {
        let identity = |target: Option<AimTarget>| {
            target.map(|target| (target.collider, target.item, target.slot))
        };
        identity(self.use_target) == identity(other.use_target)
            && identity(self.grab_target) == identity(other.grab_target)
            && identity(self.installation_target) == identity(other.installation_target)
            && identity(self.filling_target) == identity(other.filling_target)
            && identity(self.push_target) == identity(other.push_target)
            && identity(self.focus_target) == identity(other.focus_target)
            && self.grab_block == other.grab_block
            && self.installation_block == other.installation_block
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        for target in [
            &mut self.use_target,
            &mut self.grab_target,
            &mut self.installation_target,
            &mut self.filling_target,
            &mut self.push_target,
            &mut self.focus_target,
        ]
        .into_iter()
        .flatten()
        {
            for (point, shift) in target.point.iter_mut().zip(shift) {
                *point -= shift;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum QueryAction {
    Use,
    Grab,
    Removal,
    Install,
    Fill,
    Push,
    Focus,
}

impl InteractionState {
    pub fn bind_collider(&mut self, id: ColliderId, handle: ColliderHandle) {
        if let Some(binding) = self.source_bindings.get(&id) {
            self.collider_bindings.insert(handle, *binding);
        }
    }

    pub fn bind_surface(&mut self, handle: ColliderHandle, binding: ActionBinding) {
        if self.collider_bindings.get(&handle) == Some(&binding) {
            return;
        }
        if binding.collider.0 == 0
            && binding
                .use_owner
                .is_some_and(|item| self.supplies.contains(&item))
        {
            self.collider_bindings.retain(|_, previous| {
                previous.collider.0 != 0
                    || previous.use_owner != binding.use_owner
                    || previous.pickup_owner != binding.pickup_owner
            });
        }
        self.collider_bindings.insert(handle, binding);
    }

    pub fn bind_wheel_sensor(
        &mut self,
        handle: ColliderHandle,
        rim: ItemId,
        physics: &mut PhysicsWorld,
    ) {
        let layers = self.removal_layers.get(&rim).copied().unwrap_or(0);
        let binding = ActionBinding {
            collider: ColliderId(0),
            use_owner: None,
            pickup_owner: None,
            removal_owner: (layers != 0).then_some(rim),
            push_owner: None,
        };
        if self.collider_bindings.get(&handle) == Some(&binding) {
            return;
        }
        let collider = &mut physics.rapier_mut().colliders[handle];
        let mut groups = collider.collision_groups();
        groups.memberships = rapier3d::prelude::Group::from_bits_truncate(layers);
        collider.set_collision_groups(groups);
        self.bind_surface(handle, binding);
    }

    pub fn resolve(
        &self,
        physics: &PhysicsWorld,
        view: VehiclePose,
        items: &ItemStore,
    ) -> ResolvedAim {
        if !self.reach.is_finite() || self.reach <= 0.0 {
            return ResolvedAim::default();
        }
        let view = to_rapier(view);
        let ray = Ray::new(view.translation, view.rotation * -Vector::Z);
        let mut aim = ResolvedAim {
            use_target: self.query_action(physics, &ray, items, QueryAction::Use),
            grab_target: self.query_action(physics, &ray, items, QueryAction::Grab),
            installation_target: self.query_action(physics, &ray, items, QueryAction::Install),
            filling_target: self.query_action(physics, &ray, items, QueryAction::Fill),
            push_target: self.query_action(physics, &ray, items, QueryAction::Push),
            focus_target: self.query_action(physics, &ray, items, QueryAction::Focus),
            ..ResolvedAim::default()
        };
        if let Some(removal) = self.query_action(physics, &ray, items, QueryAction::Removal)
            && aim
                .grab_target
                .is_none_or(|pickup| removal.distance <= pickup.distance)
        {
            aim.grab_target = Some(removal);
        }
        if let Some(target) = aim.grab_target {
            let available = items
                .get(target.item)
                .is_some_and(|item| match item.location {
                    ItemLocation::WheelSlot(_) => {
                        self.is_wheel(target.item)
                            && self.carry.allows_pickup(target.item, false, true)
                    }
                    ItemLocation::PartSlot(_) => self.carry.allows_pickup(target.item, false, true),
                    ItemLocation::World => {
                        self.carry.allows_pickup(target.item, false, false)
                            && item.body.is_some_and(|body| {
                                body != physics.chassis_handle()
                                    && physics
                                        .rapier()
                                        .bodies
                                        .get(body)
                                        .is_some_and(|body| body.is_dynamic())
                            })
                    }
                    _ => false,
                });
            if self.parts.covered(target.item) {
                aim.grab_target = None;
                aim.grab_block = Some(InteractionBlock::Covered);
            } else if !available {
                aim.grab_target = None;
                aim.grab_block = Some(InteractionBlock::Unavailable);
            }
        }
        if let Some(item) = self.held().filter(|item| self.parts.contains(*item)) {
            (aim.installation_target, aim.installation_block) =
                self.parts.scan(item, items, physics);
        } else {
            aim.installation_block = self.wheel_installation_block(&aim);
        }
        if let Some(SlotTarget::Part(slot)) = aim.installation_target.and_then(|t| t.slot)
            && let Some(item) = self.held()
        {
            aim.installation_block = self.parts.can_install(item, slot, items).err();
        }
        if aim.installation_block.is_some() {
            aim.installation_target = None;
        }
        if self.held().is_some()
            || aim.use_target.is_some()
            || aim.grab_target.is_some()
            || aim.installation_target.is_some()
            || aim.filling_target.is_some()
        {
            aim.push_target = None;
        }
        aim
    }

    pub(super) fn wheel_installation_block(&self, aim: &ResolvedAim) -> Option<InteractionBlock> {
        aim.installation_block.or_else(|| {
            (aim.installation_target
                .is_some_and(|target| matches!(target.slot, Some(SlotTarget::Wheel(_))))
                && self.held().is_some_and(|item| !self.is_wheel(item)))
            .then_some(InteractionBlock::Incompatible)
        })
    }

    fn is_wheel(&self, item: ItemId) -> bool {
        matches!(self.kinds.get(&item), Some(ItemKind::Wheel | ItemKind::Rim))
    }

    fn supports_use(&self, item: ItemId) -> bool {
        self.usable_items.contains(&item) || self.mechanisms.contains(item)
    }

    fn query_action(
        &self,
        physics: &PhysicsWorld,
        ray: &Ray,
        items: &ItemStore,
        action: QueryAction,
    ) -> Option<AimTarget> {
        let masks = self.action_masks;
        let mask = match action {
            QueryAction::Grab => masks.pickup_mask,
            QueryAction::Install => masks.attach_mask,
            QueryAction::Push => masks.push_mask,
            _ => masks.use_mask,
        };
        let visible = masks.use_mask | masks.pickup_mask | masks.attach_mask | masks.push_mask;
        let held_body = self.carry.body();
        let owner = |handle: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            let binding = self.collider_bindings.get(&handle);
            match action {
                QueryAction::Use => binding
                    .and_then(|binding| binding.use_owner)
                    .filter(|id| self.supports_use(*id)),
                QueryAction::Fill => binding.and_then(|binding| binding.use_owner).filter(|id| {
                    self.filling_items.contains(id)
                        && self.held().is_some()
                        && self
                            .openings
                            .iter()
                            .any(|opening| opening.reference.cap == *id && opening.valve > 0.0)
                }),
                QueryAction::Grab | QueryAction::Removal => binding
                    .and_then(|binding| {
                        if matches!(action, QueryAction::Removal) {
                            self.parts
                                .slot_target(binding.collider)
                                .map(|s| self.parts.slot_item(s))
                                .unwrap_or(binding.removal_owner)
                        } else {
                            binding.pickup_owner
                        }
                    })
                    .map(|id| {
                        if matches!(self.kinds.get(&id), Some(ItemKind::Tire))
                            && let Some(ItemLocation::Attached(parent)) =
                                items.get(id).map(|item| item.location)
                            && self.is_wheel(parent)
                        {
                            parent
                        } else {
                            self.parts.covering_item(id).unwrap_or(id)
                        }
                    })
                    .filter(|id| {
                        !matches!(action, QueryAction::Removal)
                            || (self.parts.contains(*id)
                                && matches!(
                                    items.get(*id).map(|i| i.location),
                                    Some(ItemLocation::PartSlot(_))
                                ))
                            || (self.is_wheel(*id)
                                && items.get(*id).is_some_and(|item| {
                                    matches!(item.location, ItemLocation::WheelSlot(_))
                                }))
                    }),
                QueryAction::Push => binding.and_then(|binding| binding.push_owner).filter(|id| {
                    self.push.supports(*id)
                        && self.parts.slot_target(binding.unwrap().collider).is_none()
                        && collider.user_data & SLOT_SENSOR_FLAG == 0
                }),
                QueryAction::Install if collider.user_data & SLOT_SENSOR_FLAG != 0 => {
                    (collider.user_data as u32).checked_sub(1).map(ItemId)
                }
                QueryAction::Focus => binding
                    .and_then(|binding| binding.use_owner)
                    .or_else(|| (collider.user_data as u32).checked_sub(1).map(ItemId)),
                _ => None,
            }
        };
        let eligible = |handle: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.collision_groups().memberships.bits() & mask != 0
                || (matches!(action, QueryAction::Grab)
                    && self
                        .collider_bindings
                        .get(&handle)
                        .and_then(|binding| binding.pickup_owner)
                        .is_some_and(|id| {
                            self.pickup_layers
                                .get(&id)
                                .is_some_and(|layers| layers & mask != 0)
                        }))
        };
        let surfaces = |handle: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.is_enabled()
                && (held_body.is_none() || collider.parent() != held_body)
                && (self.player_body.is_none() || collider.parent() != self.player_body)
                && !collider
                    .shape()
                    .contains_point(collider.position(), ray.origin)
                && ((!collider.is_sensor()
                    && collider.collision_groups().memberships.bits() & visible != 0)
                    || (eligible(handle, collider) && owner(handle, collider).is_some()))
        };
        let (handle, hit) = physics
            .queries(QueryFilter::default().predicate(&surfaces))
            .cast_ray_and_get_normal(ray, self.reach, true)?;
        let collider = &physics.rapier().colliders[handle];
        if !eligible(handle, collider) {
            return None;
        }
        let item = owner(handle, collider)?;
        Some(AimTarget {
            collider: handle,
            item,
            distance: hit.time_of_impact,
            point: ray.point_at(hit.time_of_impact).to_array(),
            slot: (collider.user_data & SLOT_SENSOR_FLAG != 0)
                .then_some(SlotTarget::Wheel(WheelSlotId(
                    (collider.user_data >> 96) as u8,
                )))
                .or_else(|| {
                    self.parts
                        .slot_target(ColliderId((collider.user_data >> 64) as u32))
                        .map(SlotTarget::Part)
                }),
        })
    }

    pub fn prompt(&self, aim: &ResolvedAim, items: &ItemStore) -> &'static str {
        if self.push.gripping() {
            return "WASD: push and steer · F: release grip";
        }
        if let Some(block) = self.wheel_installation_block(aim) {
            return match block {
                InteractionBlock::Covered => "Remove the cover first.",
                InteractionBlock::Incompatible => "This part does not fit.",
                InteractionBlock::Occupied => "This slot is occupied.",
                InteractionBlock::Unavailable => "Cannot install this part.",
            };
        }
        if self.held().is_some() && aim.installation_target.is_some() {
            return "F: install part";
        }
        if let Some(target) = aim.use_target {
            if self.mechanisms.contains(target.item) {
                return if self.mechanisms.primary_switch(target.item) {
                    "E / LMB: switch"
                } else if self.mechanisms.turn_state(target.item).is_some() {
                    "E: switch"
                } else {
                    "Hold E + mouse: move"
                };
            }
            if self.seats.iter().any(|seat| seat.item == target.item) && self.held().is_none() {
                return "E: enter";
            }
            if let Some(opening) = self
                .openings
                .iter()
                .find(|opening| opening.reference.cap == target.item)
            {
                if opening.reference.can_tune {
                    return if aim.filling_target.is_some() {
                        "Hold E + mouse / Scroll: tune valve · Hold LMB: fill"
                    } else {
                        "Hold E + mouse / Scroll: tune valve"
                    };
                }
                if self.held().is_some() {
                    return if aim.filling_target.is_some() {
                        "E: cap · Hold LMB: fill · F: drop"
                    } else {
                        "E: cap · F: drop"
                    };
                }
                if aim.grab_target.is_some() {
                    return "E: cap · F: carry";
                }
                return "E: cap";
            }
        }
        if self.held().is_some() {
            if aim.filling_target.is_some() {
                return "Hold LMB: fill · F: drop";
            }
            return if self.carry.allows_held_rotation() {
                "F: drop / hold to throw · MMB: rotate · RMB: move · Tab: recenter"
            } else {
                "F: drop / hold to throw · RMB: move · Tab: recenter"
            };
        }
        if let Some(target) = aim.grab_target {
            if items
                .get(target.item)
                .is_some_and(|item| matches!(item.location, ItemLocation::WheelSlot(_)))
            {
                return "F: remove wheel";
            }
            if self.is_wheel(target.item) {
                return "F: carry · X: split wheel";
            }
            return "F: carry";
        }
        if let Some(target) = aim.push_target {
            if self
                .collider_bindings
                .get(&target.collider)
                .is_some_and(|binding| self.push.is_grip(target.item, binding.collider))
            {
                return "F: grip · WASD: push and steer";
            }
            return "Hold F: push";
        }
        if aim.installation_target.is_some() {
            return "Carry a part. Aim here and press F.";
        }
        if aim.grab_block.is_some() {
            return "Cannot carry this item.";
        }
        ""
    }
}
