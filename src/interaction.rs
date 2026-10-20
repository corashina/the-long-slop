pub mod attachments;
pub mod carry;
pub mod cleaning;
pub mod consumption;
pub mod equipment;
pub mod feedback;
pub mod fluid_contact;
mod input;
pub use input::InteractionOutcome;
pub mod items;
pub mod labels;
pub mod mechanisms;
pub(crate) mod memory;
pub mod parts;
pub mod push;
pub mod storage;
pub mod targets;
mod wheel_actions;
pub mod wheels;
use feedback::ActionMask;
pub use targets::{InteractionBlock, LightAim, ResolvedAim};
pub use wheel_actions::WheelAction;

use crate::fluids::{spill, transfer};
use crate::player::{ExitReference, PlayerState};
use crate::reference::{
    ActionBinding, ActionMasks, AttachmentTargetId, BodyId, ColliderGeometry, ColliderId,
    FluidParticleReference, ItemId, ItemKind, LightControlId, LightControllerKind, NodeId,
    OpeningId, OpeningRecord, PartSlotId, ReferenceError, ReferenceManifest, SeatId, TankRole,
    WheelSlotId,
};
use crate::simulation::{FrameInput, keys};
use crate::survival::NeedsOutput;
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use items::{ItemLocation, ItemStore};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::{HashMap, HashSet};

pub use crate::vehicle::SLOT_SENSOR_FLAG;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimTarget {
    pub collider: ColliderHandle,
    pub item: ItemId,
    pub distance: f32,
    pub point: [f32; 3],
    pub slot: Option<SlotTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotTarget {
    Wheel(WheelSlotId),
    Tire(ItemId),
    Part(PartSlotId),
    Attachment(AttachmentTargetId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractionAction {
    LightControl(LightControlId),
    Pickup(ItemId),
    Drop(ItemId),
    Throw(ItemId),
    Push(ItemId),
    Cap(OpeningId),
    Mechanism(ItemId),
    Seat(SeatId),
    Fill {
        source: ItemId,
        destination: ItemId,
        liters: f32,
    },
    Consume(ItemId),
    Drink {
        source: ItemId,
        requested_liters: f32,
    },
    DrinkContact {
        source: ItemId,
        poured_liters: f32,
        fractions: [f32; 6],
    },
    Wheel(WheelAction),
    PartRemove(ItemId),
    PlateRemove(ItemId),
    PlateMount(ItemId),
    PartInstall(ItemId),
}

#[derive(Clone, Copy, Debug)]
pub enum InteractionSound {
    Action(InteractionAction),
    Cap {
        id: OpeningId,
        previous: f32,
        current: f32,
        tune: Option<f32>,
    },
}

struct OpeningState {
    flow: Option<(bool, f32)>,
    fill_called: bool,
    empty_pour: bool,
    reference: OpeningRules,
    local_pose: Pose,
    tilt_axis: Vector,
    valve: f32,
    particles: Option<FluidParticleReference>,
    contact: Option<fluid_contact::FluidContact>,
}
struct OpeningRules {
    id: OpeningId,
    tank: ItemId,
    cap: ItemId,
    flow_liters_per_second: f32,
    tilt_degrees: f32,
    angle_disabled: bool,
    spill_disabled: bool,
    tube_connected: bool,
    can_toggle: bool,
    can_tune: bool,
    tune_rate: f32,
}
impl From<&OpeningRecord> for OpeningRules {
    fn from(opening: &OpeningRecord) -> Self {
        Self {
            id: opening.id,
            tank: opening.tank,
            cap: opening.cap,
            flow_liters_per_second: opening.flow_liters_per_second,
            tilt_degrees: opening.tilt_degrees,
            angle_disabled: opening.angle_disabled,
            spill_disabled: opening.spill_disabled,
            tube_connected: opening.tube_connected,
            can_toggle: opening.can_toggle,
            can_tune: opening.can_tune,
            tune_rate: opening.tune_rate,
        }
    }
}
struct SeatState {
    id: SeatId,
    item: ItemId,
    pose: VehiclePose,
    exit: ExitReference,
}

pub struct InteractionState {
    pub cleaning: cleaning::CleaningTools,
    sounds: [Option<InteractionSound>; 64],
    sound_count: usize,
    next_generated_collider: u32,
    next_generated_light: u32,
    light_controls: HashMap<ColliderId, LightControlId>,
    held_controls: HashMap<ItemId, LightControlId>,
    pub storage: storage::StorageLocks,
    light_nodes: HashMap<ColliderId, NodeId>,
    /// Door usable and owner per lighting controller. Only cabin controllers have links.
    cabin_doors: Vec<[Option<(ItemId, ItemId)>; 4]>,
    seat_action_latched: bool,
    cancelled_input: u32,
    usable_light_controls: HashSet<LightControlId>,
    fill_opening: Option<OpeningId>,
    kinds: HashMap<ItemId, ItemKind>,
    slot_fits: HashMap<WheelSlotId, HashSet<ItemId>>,
    tire_fits: HashSet<(ItemId, ItemId)>,
    f_gesture_consumed: bool,
    source_bindings: HashMap<ColliderId, ActionBinding>,
    pickup_layers: HashMap<ItemId, u32>,
    removal_layers: HashMap<ItemId, u32>,
    usable_items: HashSet<ItemId>,
    filling_items: HashSet<ItemId>,
    collider_bindings: HashMap<ColliderHandle, ActionBinding>,
    bodies: HashMap<BodyId, RigidBodyHandle>,
    pub mechanisms: mechanisms::MechanismState,
    pub parts: parts::PartAttachments,
    pub attachments: attachments::PlateAttachments,
    mechanism_bodies: HashMap<ItemId, BodyId>,
    openings: Vec<OpeningState>,
    seats: Vec<SeatState>,
    pub(crate) carry: carry::CarryState,
    pub equipment: equipment::EquipmentState,
    push: push::PushState,
    player_body: Option<RigidBodyHandle>,
    reach: f32,
    action_masks: ActionMasks,
    scroll_sensitivity: f32,
    supplies: HashSet<ItemId>,
    source_supply_emission: HashMap<ItemId, [f32; 2]>,
    tank_rules: HashMap<ItemId, TankRules>,
    slow_oil_modifier: f32,
    drink_flow_liters_per_second: f32,
    drink_contact_radius: f32,
    drink_collider_radius: f32,
    drink_collider_center: Vector,
    pending_drink_contacts: Vec<(ItemId, f32, [f32; 6])>,
    latched_mechanism: Option<(ItemId, BodyId, Vector, ColliderHandle)>,
    use_consumed: bool,
    mechanism_release_distance_squared: f32,
    tuning_opening: Option<OpeningId>,
}

#[derive(Clone, Copy)]
struct TankRules {
    no_drink: bool,
    no_fill_this: bool,
    no_fill_from: bool,
    slow_oil: bool,
}

impl InteractionState {
    pub(crate) fn restore_save(
        &mut self,
        saved: &crate::save::MechanismsSnapshot,
        ids: &crate::save::IdentityMap,
    ) {
        self.mechanisms.restore_save(ids, &saved.controls);
        for record in &saved.openings {
            self.openings
                .iter_mut()
                .find(|o| o.reference.id == ids.openings[&record.source])
                .unwrap()
                .valve = record.valve;
        }
        self.cancel_input();
    }
    pub(crate) fn restore_carry_save(
        &mut self,
        saved: &crate::save::CarrySnapshot,
        ids: &crate::save::IdentityMap,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), crate::save::SaveError> {
        self.carry.restore_save(saved, ids, items, physics)
    }
    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
    ) -> Result<crate::save::MechanismsSnapshot, crate::save::SaveError> {
        let mut openings = ids
            .openings
            .iter()
            .map(|(source, id)| {
                let opening = self
                    .openings
                    .iter()
                    .find(|o| o.reference.id == *id)
                    .ok_or_else(|| {
                        crate::save::SaveError::InvalidLinks("Missing source opening".into())
                    })?;
                Ok(crate::save::OpeningSnapshot {
                    source: source.clone(),
                    valve: opening.valve,
                })
            })
            .collect::<Result<Vec<_>, crate::save::SaveError>>()?;
        openings.sort_by(|a, b| a.source.cmp(&b.source));
        Ok(crate::save::MechanismsSnapshot {
            controls: self.mechanisms.capture_save(ids)?,
            openings,
        })
    }
    pub fn capture_carry_save(
        &self,
        ids: &crate::save::IdentityMap,
    ) -> Result<Option<crate::save::CarrySnapshot>, crate::save::SaveError> {
        self.carry.capture_save(ids)
    }
    pub(crate) fn retained_binding_bytes(&self) -> usize {
        use memory::{map, set, vector};
        map(&self.light_controls)
            + map(&self.light_nodes)
            + set(&self.usable_light_controls)
            + map(&self.kinds)
            + map(&self.slot_fits)
            + self.slot_fits.values().map(set).sum::<usize>()
            + set(&self.tire_fits)
            + map(&self.pickup_layers)
            + map(&self.removal_layers)
            + set(&self.usable_items)
            + set(&self.filling_items)
            + map(&self.collider_bindings)
            + map(&self.mechanism_bodies)
            + vector(&self.openings)
            + set(&self.supplies)
            + map(&self.source_supply_emission)
            + map(&self.tank_rules)
            + self.carry.retained_binding_bytes()
            + self.parts.retained_binding_bytes()
            + self.mechanisms.retained_binding_bytes()
    }

    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut openings = Vec::with_capacity(reference.openings.len());
        for opening in &reference.openings {
            let anchor = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == opening.anchor)
                .ok_or_else(|| ReferenceError("Missing opening anchor".into()))?;
            let item = reference
                .items
                .iter()
                .find(|item| item.id == opening.tank)
                .ok_or_else(|| ReferenceError("Missing opening tank".into()))?;
            let body = reference
                .bodies
                .iter()
                .find(|body| body.id == item.body)
                .ok_or_else(|| ReferenceError("Missing tank body".into()))?;
            let item_pose = to_rapier(items::node_pose(reference, item.node)?);
            let local_pose = item_pose.inverse()
                * transform_pose(&body.pose)
                * transform_pose(&anchor.transform);
            openings.push(OpeningState {
                reference: OpeningRules::from(opening),
                local_pose,
                tilt_axis: Vector::from_array(opening.tilt_axis),
                valve: opening.valve,
                flow: None,
                fill_called: false,
                empty_pour: false,
                particles: Some(opening.particles.clone()),
                contact: Some(fluid_contact::FluidContact::new(
                    &opening.particles,
                    reference
                        .particles
                        .iter()
                        .find(|p| p.role == format!("outlet_{}", opening.id.0))
                        .unwrap(),
                    reference
                        .particles
                        .iter()
                        .find(|p| p.role == format!("splash_{}", opening.id.0))
                        .unwrap(),
                    opening.id.0,
                )),
            });
        }
        let mut seats = Vec::with_capacity(reference.seats.len());
        for seat in &reference.seats {
            let anchor = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == seat.anchor)
                .ok_or_else(|| ReferenceError("Missing seat anchor".into()))?;
            seats.push(SeatState {
                id: seat.id,
                item: seat.item,
                pose: VehiclePose {
                    position: anchor.transform.translation,
                    rotation: anchor.transform.rotation,
                },
                exit: ExitReference::from_reference(&reference.player, seat),
            });
        }
        let mut pickup_layers = HashMap::new();
        for binding in &reference.action_bindings {
            if let Some(owner) = binding.pickup_owner {
                let collider = reference
                    .colliders
                    .iter()
                    .find(|collider| collider.id == binding.collider)
                    .ok_or_else(|| ReferenceError("Missing action collider".into()))?;
                *pickup_layers.entry(owner).or_insert(0) |= collider.membership;
            }
        }
        let usable_items = reference
            .seats
            .iter()
            .map(|seat| seat.item)
            .chain(
                reference
                    .openings
                    .iter()
                    .filter(|opening| opening.can_toggle || opening.can_tune)
                    .map(|opening| opening.cap),
            )
            .collect();
        let filling_items = reference
            .openings
            .iter()
            .map(|opening| opening.cap)
            .collect();
        let mut removal_layers = HashMap::new();
        for binding in &reference.action_bindings {
            if let Some(owner) = binding.removal_owner {
                let collider = reference
                    .colliders
                    .iter()
                    .find(|collider| collider.id == binding.collider)
                    .ok_or_else(|| ReferenceError("Missing action collider".into()))?;
                *removal_layers.entry(owner).or_insert(0) |= collider.membership;
            }
        }
        let is_rim = |id: ItemId| {
            reference
                .items
                .iter()
                .any(|item| item.id == id && matches!(item.kind, ItemKind::Rim | ItemKind::Wheel))
        };
        let slot_fits = reference
            .wheel_slots
            .iter()
            .map(|slot| {
                (
                    slot.id,
                    reference
                        .wheel_parts
                        .iter()
                        .filter(|part| {
                            is_rim(part.item)
                                && wheels::fits(
                                    &slot.rim_main_types,
                                    &slot.rim_types,
                                    &part.main_types,
                                    &part.part_types,
                                )
                        })
                        .map(|part| part.item)
                        .collect(),
                )
            })
            .collect();
        let mut tire_fits = HashSet::new();
        for rim in &reference.wheel_parts {
            if is_rim(rim.item) {
                for tire in &reference.wheel_parts {
                    if !is_rim(tire.item)
                        && wheels::fits(
                            &rim.tire_main_types,
                            &rim.tire_types,
                            &tire.main_types,
                            &tire.part_types,
                        )
                    {
                        tire_fits.insert((rim.item, tire.item));
                    }
                }
            }
        }
        let mut light_controls = HashMap::new();
        for control in &reference.lighting.controllers {
            for &collider in &control.aim_colliders {
                if let Some(previous) = light_controls.get(&collider) {
                    let previous = reference
                        .lighting
                        .controllers
                        .iter()
                        .find(|c| c.id == *previous)
                        .unwrap();
                    if control.kind != LightControllerKind::Headlight
                        || previous.kind != control.kind
                        || previous.usable_source != control.usable_source
                        || previous.usable_event != control.usable_event
                    {
                        return Err(ReferenceError("Conflicting light control collider".into()));
                    }
                    if previous.id.0 < control.id.0 {
                        continue;
                    }
                }
                light_controls.insert(collider, control.id);
            }
        }
        let cabin_doors = reference
            .lighting
            .controllers
            .iter()
            .map(|c| {
                c.cabin_input.as_ref().map_or([None; 4], |input| {
                    std::array::from_fn(|i| {
                        reference
                            .mechanisms
                            .iter()
                            .find(|m| m.source == input.door_usable_sources[i])
                            .map(|m| (m.item, m.parent_item.unwrap_or(m.item)))
                    })
                })
            })
            .collect();
        Ok(Self {
            cleaning: cleaning::CleaningTools::new(reference)?,
            source_supply_emission: HashMap::new(),
            light_nodes: reference.colliders.iter().map(|c| (c.id, c.node)).collect(),
            light_controls,
            held_controls: reference
                .flashlights
                .instances
                .iter()
                .filter(|f| f.startup_active)
                .map(|f| (f.item, f.controller))
                .collect(),
            storage: storage::StorageLocks::new(reference)?,
            cabin_doors,
            seat_action_latched: false,
            cancelled_input: 0,
            next_generated_light: reference
                .lighting
                .controllers
                .iter()
                .map(|c| c.id.0)
                .max()
                .unwrap_or(0)
                + 1,
            next_generated_collider: reference
                .colliders
                .iter()
                .map(|c| c.id.0)
                .max()
                .unwrap_or(0)
                + 1,
            fill_opening: None,
            usable_light_controls: reference
                .lighting
                .controllers
                .iter()
                .filter(|c| {
                    matches!(
                        c.kind,
                        LightControllerKind::Cabin | LightControllerKind::Headlight
                    )
                })
                .map(|c| c.id)
                .collect(),
            slot_fits,
            tire_fits,
            f_gesture_consumed: false,
            pickup_layers,
            removal_layers,
            usable_items,
            filling_items,
            source_bindings: reference
                .action_bindings
                .iter()
                .map(|binding| (binding.collider, *binding))
                .collect(),
            collider_bindings: HashMap::with_capacity(reference.colliders.len()),
            kinds: reference
                .items
                .iter()
                .map(|item| (item.id, item.kind.clone()))
                .collect(),
            bodies: HashMap::with_capacity(reference.bodies.len()),
            mechanisms: mechanisms::MechanismState::new(reference)?,
            parts: parts::PartAttachments::new(reference)?,
            attachments: attachments::PlateAttachments::new(reference)?,
            mechanism_bodies: reference.items.iter().map(|i| (i.id, i.body)).collect(),
            openings,
            sounds: [None; 64],
            sound_count: 0,
            seats,
            carry: carry::CarryState::new(reference)?,
            equipment: equipment::EquipmentState::new(reference),
            push: push::PushState::new(reference),
            player_body: None,
            reach: reference.player.aim_reach,
            action_masks: reference.player.action_masks,
            scroll_sensitivity: reference.player.scroll_sensitivity,
            supplies: reference
                .tanks
                .iter()
                .filter(|tank| tank.role == TankRole::Supply)
                .map(|tank| tank.item)
                .collect(),
            tank_rules: reference
                .tanks
                .iter()
                .map(|tank| {
                    (
                        tank.item,
                        TankRules {
                            no_drink: tank.no_drink,
                            no_fill_this: tank.no_fill_this,
                            no_fill_from: tank.no_fill_from,
                            slow_oil: tank.slow_oil,
                        },
                    )
                })
                .collect(),
            slow_oil_modifier: reference.fluid_rules.slow_oil_modifier,
            drink_flow_liters_per_second: reference.fluid_rules.pour_speed
                * reference.survival.drinking.valve
                * reference.survival.drinking.radius.powi(2)
                * std::f32::consts::PI
                * 0.025
                * reference.survival.drink_modifier,
            drink_contact_radius: reference.survival.drinking.radius
                * reference
                    .survival
                    .drinking
                    .particle_contact_radius_multiplier,
            drink_collider_radius: match &reference.survival.drinking.geometry {
                ColliderGeometry::Sphere { radius, .. } => *radius,
                _ => return Err(ReferenceError("Drink collider is not a sphere".into())),
            },
            drink_collider_center: match &reference.survival.drinking.geometry {
                ColliderGeometry::Sphere { center, .. } => {
                    Vector::from_array(reference.survival.drinking.local_position)
                        + Vector::from_array(*center)
                }
                _ => unreachable!(),
            },
            pending_drink_contacts: Vec::with_capacity(reference.openings.len()),
            latched_mechanism: None,
            use_consumed: false,
            mechanism_release_distance_squared: (reference.player.aim_reach * 1.5).powi(2),
            tuning_opening: None,
        })
    }

    pub fn bind_body(&mut self, id: BodyId, handle: RigidBodyHandle) {
        self.bodies.insert(id, handle);
        self.mechanisms.bind_body(id, handle);
    }
    pub fn bind_player_body(&mut self, handle: RigidBodyHandle) {
        self.player_body = Some(handle);
    }
    pub fn pickup_rule(&self, id: ItemId) -> Option<&crate::reference::PickupReference> {
        self.carry.rule(id)
    }
    pub fn held(&self) -> Option<ItemId> {
        self.carry.held()
    }
    pub fn held_light_control(&self) -> Option<LightControlId> {
        self.equipment
            .item()
            .or(self.held())
            .and_then(|item| self.held_controls.get(&item).copied())
    }
    pub fn pickup_stored_item(
        &mut self,
        item: ItemId,
        point: [f32; 3],
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<ItemId, ReferenceError> {
        let body = self
            .storage
            .loose_body(item)
            .ok_or_else(|| ReferenceError("Missing flashlight body".into()))?;
        if !self.storage.stored(item)
            || !self
                .carry
                .allows_pickup(item, player.seat().is_some(), items.mounted(item))
        {
            return Err(ReferenceError("Cannot carry this item".into()));
        }
        self.carry.can_acquire(item, point, player, body, physics)?;
        self.storage.unlock(item, items, physics)?;
        self.carry.pickup(item, point, player, items, physics)?;
        Ok(item)
    }
    pub fn allocate_generated_light(&mut self) -> Result<LightControlId, ReferenceError> {
        let id = self.next_generated_light;
        self.next_generated_light = id
            .checked_add(1)
            .ok_or_else(|| ReferenceError("Generated light identity exhausted".into()))?;
        Ok(LightControlId(id))
    }
    pub fn bind_generated_light(&mut self, control: LightControlId, collider: ColliderId) {
        self.light_controls.insert(collider, control);
        self.light_nodes.insert(collider, NodeId(collider.0));
        self.usable_light_controls.insert(control);
    }
    pub fn remove_generated_light(&mut self, control: LightControlId) {
        self.light_nodes
            .retain(|id, _| self.light_controls.get(id) != Some(&control));
        self.light_controls.retain(|_, id| *id != control);
        self.usable_light_controls.remove(&control);
    }
    pub fn allocate_generated_collider(&mut self) -> Result<ColliderId, ReferenceError> {
        let id = self.next_generated_collider;
        self.next_generated_collider = id
            .checked_add(1)
            .ok_or_else(|| ReferenceError("Generated collider identity exhausted".into()))?;
        Ok(ColliderId(id))
    }
    pub fn register_generated_part_kind(&mut self, item: ItemId, layers: u32) {
        self.removal_layers.insert(item, layers);
        for binding in self.collider_bindings.values_mut() {
            if binding.pickup_owner == Some(item) {
                binding.removal_owner = Some(item);
            }
        }
    }
    pub fn register_generated_wheel(
        &mut self,
        rule: &crate::reference::WheelPartReference,
        rim: bool,
        rules: &[crate::reference::WheelPartReference],
        reference: &ReferenceManifest,
    ) {
        self.kinds
            .insert(rule.item, if rim { ItemKind::Rim } else { ItemKind::Tire });
        for slot in &reference.wheel_slots {
            if rim
                && wheels::fits(
                    &slot.rim_main_types,
                    &slot.rim_types,
                    &rule.main_types,
                    &rule.part_types,
                )
            {
                self.slot_fits.entry(slot.id).or_default().insert(rule.item);
            }
        }
        for old in rules {
            if old.item == rule.item {
                continue;
            }
            let old_rim = self
                .kinds
                .get(&old.item)
                .is_some_and(|kind| matches!(kind, ItemKind::Rim | ItemKind::Wheel));
            if rim
                && !old_rim
                && wheels::fits(
                    &rule.tire_main_types,
                    &rule.tire_types,
                    &old.main_types,
                    &old.part_types,
                )
            {
                self.tire_fits.insert((rule.item, old.item));
            }
            if !rim
                && old_rim
                && wheels::fits(
                    &old.tire_main_types,
                    &old.tire_types,
                    &rule.main_types,
                    &rule.part_types,
                )
            {
                self.tire_fits.insert((old.item, rule.item));
            }
        }
    }
    /// Remove only bindings owned by this generated instance.
    pub fn unbind_generated(&mut self, id: ItemId, colliders: &[ColliderHandle]) {
        for collider in colliders {
            if self.collider_bindings.get(collider).is_some_and(|binding| {
                [
                    binding.use_owner,
                    binding.pickup_owner,
                    binding.removal_owner,
                    binding.push_owner,
                ]
                .contains(&Some(id))
            }) {
                self.collider_bindings.remove(collider);
            }
        }
        if let Some(body) = self.mechanism_bodies.remove(&id) {
            self.bodies.remove(&body);
        }
        if self
            .latched_mechanism
            .is_some_and(|(item, _, _, _)| item == id)
        {
            self.latched_mechanism = None;
        }
        self.carry.unregister_generated(id);
        for fits in self.slot_fits.values_mut() {
            fits.remove(&id);
        }
        self.tire_fits
            .retain(|(rim, tire)| *rim != id && *tire != id);
        self.kinds.remove(&id);
        self.pickup_layers.remove(&id);
        self.removal_layers.remove(&id);
        self.usable_items.remove(&id);
        self.filling_items.remove(&id);
        self.supplies.remove(&id);
        self.source_supply_emission.remove(&id);
        self.tank_rules.remove(&id);
        self.openings
            .retain(|opening| opening.reference.tank != id && opening.reference.cap != id);
        if self.fill_opening.is_some_and(|opening| {
            !self
                .openings
                .iter()
                .any(|state| state.reference.id == opening)
        }) {
            self.fill_opening = None;
        }
        if self.tuning_opening.is_some_and(|opening| {
            !self
                .openings
                .iter()
                .any(|state| state.reference.id == opening)
        }) {
            self.tuning_opening = None;
        }
    }

    pub fn register_generated_mechanism(
        &mut self,
        item: ItemId,
        body: RigidBodyHandle,
        colliders: &[(crate::reference::SourceId, ColliderHandle)],
    ) -> Result<(), ReferenceError> {
        let id = BodyId(
            self.bodies
                .keys()
                .map(|b| b.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| ReferenceError("Generated body identity exhausted".into()))?,
        );
        self.bodies.insert(id, body);
        self.mechanism_bodies.insert(item, id);
        self.usable_items.insert(item);
        for (_, handle) in colliders {
            if let Some(binding) = self.collider_bindings.get_mut(handle) {
                binding.use_owner = Some(item);
            }
        }
        Ok(())
    }
    pub fn bind_generated_mechanism_surface(
        &mut self,
        item: ItemId,
        handle: ColliderHandle,
        collider: ColliderId,
    ) {
        let binding = self
            .collider_bindings
            .entry(handle)
            .or_insert(ActionBinding {
                collider,
                use_owner: None,
                pickup_owner: None,
                removal_owner: None,
                push_owner: None,
            });
        binding.use_owner = Some(item);
    }
    pub fn register_generated_pickup(
        &mut self,
        id: ItemId,
        rule: crate::reference::PickupReference,
        layers: u32,
        container: bool,
    ) {
        self.carry.register_generated(id, rule);
        self.pickup_layers.insert(id, layers);
        self.kinds.insert(
            id,
            if container {
                ItemKind::Container
            } else {
                ItemKind::Decoration
            },
        );
    }
    pub fn generated_opening(&self, cap: ItemId) -> Option<OpeningId> {
        self.openings
            .iter()
            .find(|o| o.reference.cap == cap)
            .map(|o| o.reference.id)
    }
    pub fn generated_valve(&self, cap: ItemId) -> Option<f32> {
        self.openings
            .iter()
            .find(|opening| opening.reference.cap == cap)
            .map(|opening| opening.valve)
    }
    pub fn restore_generated_valve(
        &mut self,
        cap: ItemId,
        valve: f32,
    ) -> Result<(), ReferenceError> {
        if !valve.is_finite() || !(0.0..=1.0).contains(&valve) {
            return Err(ReferenceError("Invalid generated valve snapshot".into()));
        }
        let opening = self
            .openings
            .iter_mut()
            .find(|opening| opening.reference.cap == cap)
            .ok_or_else(|| ReferenceError("Missing generated valve".into()))?;
        opening.valve = valve;
        Ok(())
    }

    pub fn register_supply(&mut self, item: ItemId) {
        self.supplies.insert(item);
    }
    /// Bind checked source contents and opening rules once per generated instance.
    pub fn register_generated_supply(
        &mut self,
        tank: ItemId,
        cap: ItemId,
        definition: &crate::world::SupplyDefinition,
        point: &crate::world::SupplyPoint,
        reference: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        if self
            .openings
            .iter()
            .any(|opening| opening.reference.cap == cap)
        {
            return if self.source_supply_emission.contains_key(&cap) {
                Ok(())
            } else {
                Err(ReferenceError(
                    "Generated cap overlaps an imported opening".into(),
                ))
            };
        }
        let get = |source: &crate::reference::SourceId| {
            reference
                .world
                .records
                .iter()
                .find(|record| &record.source == source)
                .ok_or_else(|| {
                    ReferenceError(format!(
                        "Missing source supply {}:{}",
                        source.file, source.path_id
                    ))
                })
        };
        let cap_record = get(&definition.cap)?;
        let tank_record = get(&definition.tank)?;
        let number = |record: &crate::reference::WorldSourceRecord,
                      name: &str|
         -> Result<f32, ReferenceError> {
            use crate::reference::WorldValue;
            let value = match record.fields.get(name) {
                Some(WorldValue::Number(value)) => *value as f32,
                Some(WorldValue::Integer(value)) => *value as f32,
                Some(WorldValue::Bool(value)) => u8::from(*value) as f32,
                _ => {
                    return Err(ReferenceError(format!(
                        "Missing source supply field {name}"
                    )));
                }
            };
            if !value.is_finite() {
                return Err(ReferenceError(format!(
                    "Nonfinite source supply field {name}"
                )));
            }
            Ok(value)
        };
        let radius = number(cap_record, "radius")?;
        let flow =
            radius * radius * std::f32::consts::PI * 0.025 * reference.fluid_rules.pour_speed;
        let valve = number(cap_record, "valve")?;
        let can_toggle = number(cap_record, "canToggle")? != 0.0;
        let can_tune = number(cap_record, "canTune")? != 0.0;
        let rules = OpeningRules {
            id: OpeningId(
                self.openings
                    .iter()
                    .map(|opening| opening.reference.id.0)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| ReferenceError("Opening identity space exhausted".into()))?,
            ),
            tank,
            cap,
            flow_liters_per_second: flow,
            tilt_degrees: number(cap_record, "defPourAngle")?,
            angle_disabled: number(cap_record, "noAngle")? != 0.0,
            spill_disabled: definition.no_pour,
            tube_connected: number(cap_record, "tube")? != 0.0,
            can_toggle,
            can_tune,
            tune_rate: number(cap_record, "tuneSpeed")? * reference.fluid_rules.fluid_tune_speed,
        };
        let tank_rules = TankRules {
            no_drink: number(tank_record, "noDrink")? != 0.0,
            no_fill_this: definition.no_fill_this,
            no_fill_from: definition.no_fill_from,
            slow_oil: number(tank_record, "slowOil")? != 0.0,
        };
        // Subtract in f64 before conversion, so distant bindings retain local precision.
        let relative = VehiclePose {
            position: std::array::from_fn(|a| {
                (point.cap_pose.position[a] - point.tank_pose.position[a]) as f32
            }),
            rotation: point.cap_pose.rotation,
        };
        let tank_rotation = to_rapier(VehiclePose {
            position: [0.0; 3],
            rotation: point.tank_pose.rotation,
        });
        self.openings.push(OpeningState {
            reference: rules,
            local_pose: tank_rotation.inverse() * to_rapier(relative),
            tilt_axis: -Vector::Z,
            valve,
            flow: None,
            fill_called: false,
            empty_pour: false,
            particles: None,
            contact: None,
        });
        self.source_supply_emission.insert(
            cap,
            [definition.emission_min_rate, definition.emission_max_rate],
        );
        self.tank_rules.insert(tank, tank_rules);
        self.kinds.insert(cap, ItemKind::Cap);
        if can_toggle || can_tune {
            self.usable_items.insert(cap);
        }
        self.filling_items.insert(cap);
        self.supplies.insert(tank);
        Ok(())
    }

    /// A movable container shares source opening rules but is not a fixed supply nozzle.
    pub fn register_generated_container(
        &mut self,
        tank: ItemId,
        cap: ItemId,
        definition: &crate::world::SupplyDefinition,
        point: &crate::world::SupplyPoint,
        reference: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        self.register_generated_supply(tank, cap, definition, point, reference)?;
        self.supplies.remove(&tank);
        self.kinds.insert(tank, ItemKind::Container);
        Ok(())
    }
    pub fn bind_generated_cap(&mut self, handle: ColliderHandle, cap: ItemId) {
        let binding = self
            .collider_bindings
            .entry(handle)
            .or_insert(ActionBinding {
                collider: ColliderId(0),
                use_owner: None,
                pickup_owner: None,
                removal_owner: None,
                push_owner: None,
            });
        binding.use_owner = Some(cap);
    }
    fn sound(&mut self, sound: InteractionSound) {
        if self.sound_count < self.sounds.len() {
            self.sounds[self.sound_count] = Some(sound);
            self.sound_count += 1;
        }
    }

    fn complete_carry_release(
        &mut self,
        action: &mut Option<InteractionAction>,
        release: InteractionAction,
    ) {
        if matches!(*action, Some(InteractionAction::LightControl(_))) {
            // The runtime must still execute this external owner.
            self.completed_action(release);
        } else {
            *action = Some(release);
        }
    }
    pub fn completed_action(&mut self, action: InteractionAction) {
        self.sound(InteractionSound::Action(action));
    }

    pub fn pop_sound(&mut self) -> Option<InteractionSound> {
        if self.sound_count == 0 {
            return None;
        }
        let sound = self.sounds[0];
        self.sound_count -= 1;
        self.sounds.copy_within(1..=self.sound_count, 0);
        self.sounds[self.sound_count] = None;
        sound
    }

    pub fn opening_flow(&self, id: OpeningId) -> Option<(bool, f32)> {
        self.openings
            .iter()
            .find(|o| o.reference.id == id)
            .and_then(|o| o.flow)
    }

    pub fn opening_audio_flags(&self, id: OpeningId) -> (bool, bool) {
        self.openings
            .iter()
            .find(|o| o.reference.id == id)
            .map_or((false, false), |o| (o.fill_called, o.empty_pour))
    }
    pub fn opening_is_open(&self, id: OpeningId) -> bool {
        self.opening_valve(id) > 0.0
    }
    pub fn opening_valve(&self, id: OpeningId) -> f32 {
        self.openings
            .iter()
            .find(|opening| opening.reference.id == id)
            .map_or(0.0, |opening| opening.valve)
    }
    pub fn tuning_opening(&self) -> Option<OpeningId> {
        self.tuning_opening
    }
    pub fn dragging_mechanism(&self) -> Option<ItemId> {
        self.latched_mechanism.map(|(item, _, _, _)| item)
    }
    pub fn throw_charge(&self) -> f32 {
        self.carry.throw_charge()
    }
    pub fn throw_active(&self) -> bool {
        self.carry.throw_active()
    }
    /// Returns the pushed body and its steering input.
    pub fn push_steering(&self) -> Option<(RigidBodyHandle, f32)> {
        self.push.steering()
    }
    pub fn push_gripping(&self) -> bool {
        self.push.gripping()
    }
    pub fn cabin_door_values(&self, control: usize, items: &ItemStore) -> [f32; 4] {
        self.cabin_doors[control].map(|link| {
            link.filter(|(_, owner)| items.mounted(*owner))
                .and_then(|(usable, _)| self.mechanisms.value(usable, false))
                .unwrap_or(0.)
        })
    }
    pub fn cancel_input(&mut self) {
        self.cancelled_input = keys::GRAB
            | keys::RECENTER
            | keys::TRANSLATE
            | keys::ROTATE
            | keys::USE
            | keys::PRIMARY;
        self.cleaning.reset_contact();
        self.seat_action_latched = false;
        self.fill_opening = None;
        self.f_gesture_consumed = false;
        self.carry.cancel_input();
        self.push.cancel_input();
        self.latched_mechanism = None;
        self.use_consumed = false;
        self.tuning_opening = None;
        self.mechanisms.cancel_input();
    }
    pub fn finish_cancelled_input(&mut self, player: &mut PlayerState, physics: &mut PhysicsWorld) {
        self.push.finish_cancelled_input(player, physics);
    }
    pub fn drop_held(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<ItemId> {
        let item = self.carry.drop(items, physics)?;
        self.cleaning.reset_contact();
        self.storage.mark_dropped(item);
        Some(item)
    }
    pub fn pickup_item(
        &mut self,
        item: ItemId,
        point: [f32; 3],
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.carry.pickup(item, point, player, items, physics)
    }

    pub fn fluid_particles(
        &self,
    ) -> impl Iterator<Item = (OpeningId, &fluid_contact::FluidContact)> {
        self.openings
            .iter()
            .filter_map(|o| o.contact.as_ref().map(|c| (o.reference.id, c)))
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        self.cleaning.reset_contact();
        self.mechanisms.rebase(shift);
        self.parts.rebase(shift);
        self.attachments.rebase(shift);
        self.storage.rebase(shift);
        for opening in &mut self.openings {
            if let Some(contact) = opening.contact.as_mut() {
                contact.rebase(Vector::from_array(shift));
            }
        }
    }

    pub fn take_drink_contact(&mut self) -> Option<(ItemId, f32, [f32; 6])> {
        self.pending_drink_contacts.pop()
    }

    pub fn step(
        &mut self,
        mut input: FrameInput,
        aim: ResolvedAim,
        player: &mut PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        dt: f32,
        needs: NeedsOutput,
        fluid_scale: f32,
    ) -> InteractionOutcome {
        self.finish_cancelled_input(player, physics);
        input = self.filter_cancelled_input(input);
        if self.cleaning.reserves_lmb(self.held()) {
            input.held &= !keys::PRIMARY;
            input.pressed &= !keys::PRIMARY;
            input.released &= !keys::PRIMARY;
        }
        if !dt.is_finite() || dt <= 0.0 {
            return InteractionOutcome {
                action: None,
                look_mouse: input.mouse,
            };
        }
        for opening in &mut self.openings {
            opening.flow = None;
            opening.fill_called = false;
            opening.empty_pour = false;
        }
        self.mechanisms.begin_audio_step();
        let mut cap_sounds = [None; 2];
        self.pending_drink_contacts.clear();
        let acquisition_release = self.carry.acquisition_held();
        let reach = self.reach;
        let valid = |target: AimTarget| {
            target.distance.is_finite() && target.distance >= 0.0 && target.distance <= reach
        };
        if input.held & keys::USE == 0 {
            self.seat_action_latched = false;
        }
        if self.active_mechanism_target(player, physics).is_none() {
            self.latched_mechanism = None;
        }
        let mut availability = self.available_actions(&aim, player, items, physics, needs);
        if availability.active_drag && !self.drag_input(input, &availability) {
            self.latched_mechanism = None;
            availability = self.available_actions(&aim, player, items, physics, needs);
        }
        let resolved = self.resolve_input(input, &availability);
        self.carry.release_acquisition(input);
        let grab_input = resolved.grab_input;
        let use_input = resolved.use_input;
        let held_switch_pressed =
            use_input.pressed & keys::PRIMARY != 0 && self.held_light_control().is_some();
        let light_action = if held_switch_pressed {
            self.held_light_control()
        } else {
            availability.light_control.filter(|_| {
                use_input.pressed & keys::USE != 0
                    || (use_input.pressed & keys::PRIMARY != 0 && availability.lmb_use)
            })
        };
        let target = aim
            .use_target
            .filter(|target| valid(*target) && availability.light_control.is_none());
        let grab_target = aim.grab_target.filter(|target| valid(*target));
        let filling_target = aim.filling_target.filter(|target| valid(*target));
        self.mechanisms.sync(physics, false);
        let released_mechanism = use_input.released & keys::USE != 0 && self.use_consumed;
        let primary_use = availability.lmb_use && self.held_light_control().is_none();
        let mechanism_held = use_input.held & keys::USE != 0
            || (use_input.held & keys::PRIMARY != 0 && primary_use)
            || (use_input.wheel != 0.
                && availability.use_item.is_some_and(|item| {
                    self.mechanisms.drag_eligible(item) && self.mechanisms.scroll_eligible(item)
                }));
        if !mechanism_held {
            self.latched_mechanism = None;
            self.use_consumed = false;
        } else if self.latched_mechanism.is_none()
            && let Some(target) = target
            && self.mechanisms.contains(target.item)
            && (use_input.held & keys::USE != 0 || self.mechanisms.drag_eligible(target.item))
            && let Some(&body_id) = self.mechanism_bodies.get(&target.item)
            && let Some(body) = self
                .bodies
                .get(&body_id)
                .and_then(|handle| physics.rapier().bodies.get(*handle))
        {
            self.latched_mechanism = Some((
                target.item,
                body_id,
                body.position().inverse() * Vector::from_array(target.point),
                target.collider,
            ));
            self.use_consumed = true;
        }
        let use_target = self.active_mechanism_target(player, physics).or(target);
        let mut action = None;
        let available = availability;
        if grab_input.pressed & keys::GRAB != 0 {
            self.f_gesture_consumed = false;
        }
        let grip_release = self.push.gripping() && grab_input.pressed & keys::GRAB != 0;
        if grip_release {
            self.push.stop(player, physics);
            self.f_gesture_consumed = true;
        }
        if grab_input.released & keys::GRAB != 0
            && !self.f_gesture_consumed
            && !acquisition_release
            && available.actions.0 & ActionMask::MOUNT != 0
            && self.carry.throw_charge() < self.carry.reference_throw_minimum()
        {
            if let Some(slot) = available.slot {
                let held = self.held().expect("mount held item");
                match slot {
                    SlotTarget::Wheel(slot) => {
                        self.carry.cancel_input();
                        action = Some(InteractionAction::Wheel(WheelAction::Install {
                            item: held,
                            target: slot,
                        }));
                    }
                    SlotTarget::Tire(rim) => {
                        self.carry.cancel_input();
                        action = Some(InteractionAction::Wheel(WheelAction::Join {
                            rim,
                            tire: held,
                        }));
                    }
                    SlotTarget::Attachment(target) => {
                        self.mount_plate(held, target, items, physics)
                            .expect("validated plate mount");
                        action = Some(InteractionAction::PlateMount(held));
                    }
                    SlotTarget::Part(slot) => {
                        self.drop_held(items, physics);
                        self.parts
                            .install(held, slot, items, physics)
                            .expect("validated part installation");
                        self.parts.mount_mechanism(held, &mut self.mechanisms);
                        self.sync_parts(items, physics).expect("bound parts");
                        action = Some(InteractionAction::PartInstall(held));
                        self.sound(InteractionSound::Action(InteractionAction::PartInstall(
                            held,
                        )));
                    }
                }
            }
        }
        if grab_input.pressed & keys::GRAB != 0
            && !self.carry.acquisition_held()
            && !grip_release
            && self.held().is_none()
        {
            if available.actions.0 & ActionMask::DISMOUNT != 0 {
                if let Some(rim) = available.rim {
                    action = Some(InteractionAction::Wheel(WheelAction::Split(rim)));
                } else if let Some(target) = grab_target {
                    if self.attachments.contains(target.item) {
                        self.remove_plate(target.item, target.point, player, items, physics)
                            .expect("validated plate removal");
                        action = Some(InteractionAction::PlateRemove(target.item));
                    } else if self.parts.contains(target.item) {
                        self.parts
                            .remove(target.item, items, physics)
                            .expect("validated removal");
                        self.parts
                            .detach_mechanism(target.item, &mut self.mechanisms, physics);
                        self.sync_parts(items, physics).expect("bound parts");
                        self.carry
                            .pickup(target.item, target.point, player, items, physics)
                            .expect("validated carry acquisition");
                        action = Some(InteractionAction::PartRemove(target.item));
                        self.completed_action(InteractionAction::PartRemove(target.item));
                    } else {
                        action = Some(InteractionAction::Wheel(WheelAction::Remove(target.item)));
                    }
                }
            }
            if action.is_some() {
                self.f_gesture_consumed = true;
            }
        }

        let carry_input = resolved.carry_input;
        self.tuning_opening = None;
        let can_use_target = availability.light_control.is_some()
            || use_target.is_some_and(|target| {
                self.mechanisms.contains(target.item)
                    || self.openings.iter().any(|opening| {
                        opening.reference.cap == target.item
                            && (opening.reference.can_toggle || opening.reference.can_tune)
                    })
            });
        if action.is_none() {
            action = light_action.map(InteractionAction::LightControl);
            if use_input.pressed & keys::USE != 0 && player.seat().is_some() && !can_use_target {
                if let Some(seat) = self
                    .seats
                    .iter()
                    .find(|seat| Some(seat.id) == player.seat())
                    && player.try_exit(physics, &seat.exit)
                {
                    self.seat_action_latched = true;
                    action = Some(InteractionAction::Seat(seat.id));
                }
            } else if let Some(target) = use_target {
                if use_input.held & keys::USE != 0
                    || use_input.pressed & (keys::USE | keys::PRIMARY) != 0
                    || use_input.wheel != 0.
                {
                    items.mark_dirty(target.item);
                }
                if use_input.pressed & keys::USE != 0
                    || (!held_switch_pressed
                        && use_input.pressed & keys::PRIMARY != 0
                        && availability.lmb_use)
                {
                    for opening in &mut self.openings {
                        if opening.reference.cap == target.item && opening.reference.can_toggle {
                            let previous = opening.valve;
                            items.mark_dirty(target.item);
                            opening.valve = if opening.valve > 0.0 { 0.0 } else { 1.0 };
                            cap_sounds[0] = Some(InteractionSound::Cap {
                                id: opening.reference.id,
                                previous,
                                current: opening.valve,
                                tune: None,
                            });
                            action = Some(InteractionAction::Cap(opening.reference.id));
                            break;
                        }
                    }
                }
                if let Some(opening) = self.openings.iter_mut().find(|opening| {
                    opening.reference.cap == target.item && opening.reference.can_tune
                }) {
                    let delta = if use_input.wheel.is_finite() && use_input.wheel != 0.0 {
                        use_input.wheel * self.scroll_sensitivity
                    } else if use_input.held & keys::USE != 0
                        || (primary_use && use_input.held & keys::PRIMARY != 0)
                    {
                        use_input.mouse[1]
                    } else {
                        0.0
                    };
                    if use_input.held & keys::USE != 0
                        || (primary_use && use_input.held & keys::PRIMARY != 0)
                        || delta != 0.0
                    {
                        self.tuning_opening = Some(opening.reference.id);
                    }
                    if delta.is_finite() && delta != 0.0 {
                        items.mark_dirty(target.item);
                        let previous = opening.valve;
                        let requested = delta * dt * opening.reference.tune_rate;
                        opening.valve = (opening.valve + requested).clamp(0.0, 1.0);
                        cap_sounds[1] = Some(InteractionSound::Cap {
                            id: opening.reference.id,
                            previous,
                            current: opening.valve,
                            tune: Some(requested),
                        });
                        if opening.valve < 1e-6 {
                            opening.valve = 0.0;
                        }
                        action = Some(InteractionAction::Cap(opening.reference.id));
                    }
                }
                if use_input.released & keys::USE != 0
                    && !released_mechanism
                    && self.held().is_none()
                    && let Some(seat) = self.seats.iter().find(|seat| seat.item == target.item)
                    && player.enter(seat.id, seat.pose, physics)
                {
                    self.seat_action_latched = true;
                    action = Some(InteractionAction::Seat(seat.id));
                }
                if self.mechanisms.contains(target.item)
                    && (use_input.pressed & keys::USE != 0
                        || (!held_switch_pressed
                            && use_input.pressed & keys::PRIMARY != 0
                            && self.mechanisms.primary_switch(target.item)
                            && availability.lmb_use)
                        || use_input.held & keys::USE != 0
                        || (use_input.held & keys::PRIMARY != 0
                            && (self.mechanisms.radio_tunable(target.item)
                                || self.mechanisms.drag_eligible(target.item))
                            && primary_use)
                        || (use_input.wheel != 0.0 && self.mechanisms.scroll_eligible(target.item)))
                {
                    let mut mechanism_input = use_input;
                    if !availability.lmb_use {
                        mechanism_input.held &= !keys::PRIMARY;
                        mechanism_input.pressed &= !keys::PRIMARY;
                    }
                    self.mechanisms.step(
                        target.item,
                        mechanism_input,
                        player.view_pose(physics),
                        dt,
                    );
                    action = Some(InteractionAction::Mechanism(target.item));
                }
            }
        }
        for sound in cap_sounds.into_iter().flatten() {
            self.sound(sound);
        }
        if action.is_none()
            && !grip_release
            && !self.carry.acquisition_held()
            && self.held().is_none()
            && (grab_input.held | grab_input.pressed) & keys::GRAB != 0
            && available.actions.0 & ActionMask::PICKUP != 0
            && let Some(target) = grab_target
            && (if self.storage.stored(target.item) {
                self.pickup_stored_item(target.item, target.point, player, items, physics)
                    .is_ok()
            } else {
                self.carry
                    .pickup(target.item, target.point, player, items, physics)
                    .is_ok()
            })
        {
            action = Some(InteractionAction::Pickup(target.item));
        }
        if matches!(action, Some(InteractionAction::Pickup(_)))
            && (grab_input.held | grab_input.pressed) & keys::GRAB != 0
        {
            self.f_gesture_consumed = true;
        }
        let push_target = aim.push_target.filter(|target| valid(*target));
        let pushing =
            if (action.is_none() || self.push.gripping()) && self.held().is_none() && !grip_release
            {
                self.push.step(
                    resolved.push_input,
                    push_target,
                    &self.bodies,
                    player,
                    physics,
                    dt,
                )
            } else {
                self.push.stop(player, physics);
                None
            };
        if action.is_none()
            && let Some(item) = pushing
        {
            action = Some(InteractionAction::Push(item));
        }
        if !acquisition_release && !self.f_gesture_consumed && self.held().is_some() {
            if grab_input.pressed & keys::GRAB != 0 {
                self.carry.begin_throw();
            }
            if grab_input.held & keys::GRAB != 0 {
                self.carry.charge_throw(dt);
            }
            if grab_input.released & keys::GRAB != 0 {
                let throwing = self.carry.throw_charge() > self.carry.reference_throw_minimum();
                if let Some(item) = self.carry.release_throw(player, items, physics) {
                    self.storage.mark_dropped(item);
                    let release = if throwing {
                        InteractionAction::Throw(item)
                    } else {
                        InteractionAction::Drop(item)
                    };
                    self.complete_carry_release(&mut action, release);
                }
            }
        }
        if self.carry.step(carry_input, player, physics, dt)
            && let Some(item) = self.drop_held(items, physics)
        {
            self.complete_carry_release(&mut action, InteractionAction::Drop(item));
        }
        self.mechanisms.sync(physics, false);
        items.sync_poses(physics);
        let mut filling = None;
        let mut filled_destination = None;
        if !held_switch_pressed
            && input.held & keys::PRIMARY != 0
            && available.actions.0 & ActionMask::FILL != 0
            && let Some(held) = self.held()
            && let Some(target) = filling_target
        {
            if let Some(opening) = self
                .openings
                .iter()
                .find(|opening| opening.reference.cap == target.item && opening.valve > 0.0)
            {
                let (source_id, destination, flow, destination_opening) =
                    if self.supplies.contains(&opening.reference.tank) {
                        if let Some(destination) = self.openings.iter().find(|destination| {
                            destination.reference.tank == held && destination.valve > 0.0
                        }) {
                            (
                                opening.reference.tank,
                                held,
                                opening.reference.flow_liters_per_second * opening.valve,
                                destination.reference.id,
                            )
                        } else {
                            (held, held, 0.0, opening.reference.id)
                        }
                    } else {
                        (
                            held,
                            opening.reference.tank,
                            opening.reference.flow_liters_per_second * opening.valve,
                            opening.reference.id,
                        )
                    };
                let source_allowed = self
                    .tank_rules
                    .get(&source_id)
                    .is_some_and(|rules| !rules.no_fill_from);
                let destination_rules = self.tank_rules.get(&destination);
                if source_id != destination
                    && source_allowed
                    && destination_rules.is_some_and(|rules| !rules.no_fill_this)
                    && let Some((source, destination_state)) =
                        items.get_pair_mut(source_id, destination)
                    && let (Some(source), Some(destination_state)) =
                        (source.fluid.as_mut(), destination_state.fluid.as_mut())
                {
                    let mut amount = flow * dt;
                    let source_total = source.mixture.total();
                    let destination_total = destination_state.mixture.total();
                    if destination_rules.is_some_and(|rules| rules.slow_oil)
                        && source_total > 0.0
                        && source.mixture.amount(1) / source_total > 0.25
                        && destination_total > 0.05
                        && destination_state.mixture.amount(1) / destination_total < 1.0
                    {
                        amount = (amount * destination_total * self.slow_oil_modifier)
                            .max(0.0001)
                            .min(amount);
                    }
                    let liters = transfer(source, destination_state, amount * fluid_scale);
                    if liters > 0.0 {
                        filling = Some(source_id);
                        filled_destination = Some(destination_opening);
                        action = Some(InteractionAction::Fill {
                            source: source_id,
                            destination,
                            liters,
                        });
                    }
                }
            }
        }
        for opening in &mut self.openings {
            if filled_destination == Some(opening.reference.id) {
                opening.flow = Some((true, 1.));
                opening.fill_called = true;
            }
            let Some(particles) = opening.particles.as_ref() else {
                continue;
            };
            let contact = opening.contact.as_mut().unwrap();
            let Some(item) = items.get_mut(opening.reference.tank) else {
                continue;
            };
            if item.inactive_pose.is_some() {
                continue;
            }
            let cap_pose = to_rapier(item.pose) * opening.local_pose;
            let forward = cap_pose.rotation * opening.tilt_axis;
            let angle = forward.dot(Vector::Y).clamp(-1.0, 1.0).acos().to_degrees();
            let mut emission_rate = 0.;
            let mut emission_speed = 0.;
            let mut poured = None;
            let mut fractions = [0.; 6];
            if let Some(fluid) = item.fluid.as_mut() {
                let fill = (fluid.mixture.total() / fluid.capacity_liters).clamp(0.0, 1.0);
                let threshold = 100.0 + (opening.reference.tilt_degrees - 100.0) * fill;
                let factor = if opening.reference.angle_disabled {
                    1.0
                } else {
                    inverse_lerp(threshold, opening.reference.tilt_degrees + 90.0, angle)
                        .max(inverse_lerp(90.0, 180.0, angle))
                };
                opening.empty_pour = fluid.mixture.total() <= 0.;
                let emitting = opening.valve > 0.
                    && !opening.reference.spill_disabled
                    && !opening.reference.tube_connected
                    && filling != Some(opening.reference.tank)
                    && (opening.reference.angle_disabled || angle > threshold)
                    && fluid.mixture.total() > 0.;
                if emitting {
                    let speed = factor * opening.valve * opening.reference.flow_liters_per_second;
                    let requested_liters = speed * dt * fluid_scale;
                    let total = fluid.mixture.total();
                    fractions = fluid.mixture.quantities_liters.map(|amount| amount / total);
                    let debited = spill(fluid, requested_liters);
                    if debited > 0. {
                        opening.flow = Some((false, speed));
                    }
                    poured = Some(requested_liters);
                    emission_rate = particles.min_rate
                        + (particles.max_rate - particles.min_rate) * speed.clamp(0., 1.);
                    emission_speed = particles.max_speed * fill * factor;
                }
            }
            let view = to_rapier(player.view_pose(physics));
            let cap_center = view.translation + view.rotation * self.drink_collider_center;
            if poured.is_some() {
                contact.set_color(fractions);
            }
            let hit = contact.advance(
                particles,
                cap_pose,
                opening.tilt_axis,
                item.body,
                self.player_body,
                cap_center,
                self.drink_collider_radius,
                self.drink_contact_radius,
                physics,
                dt,
                emission_rate,
                emission_speed,
                poured,
            );
            if hit && contact.last_poured > 0. {
                if poured.is_none() {
                    let Some(fluid) = item.fluid.as_ref() else {
                        continue;
                    };
                    let total = fluid.mixture.total();
                    if total <= 0. {
                        continue;
                    }
                    fractions = fluid.mixture.quantities_liters.map(|amount| amount / total);
                }
                if action.is_none() {
                    action = Some(InteractionAction::DrinkContact {
                        source: opening.reference.tank,
                        poured_liters: contact.last_poured,
                        fractions,
                    });
                } else {
                    self.pending_drink_contacts.push((
                        opening.reference.tank,
                        contact.last_poured,
                        fractions,
                    ));
                }
            }
        }
        if input.released & keys::GRAB != 0 {
            self.f_gesture_consumed = false;
        }
        if action.is_none()
            && input.pressed & keys::PRIMARY != 0
            && available.actions.0 & ActionMask::CONSUME != 0
            && let Some(item) = self.held()
        {
            action = Some(InteractionAction::Consume(item));
        }
        if action.is_none()
            && input.held & keys::PRIMARY != 0
            && available.actions.0 & ActionMask::DRINK != 0
            && let Some(source) = self.held()
        {
            action = Some(InteractionAction::Drink {
                source,
                requested_liters: self.drink_flow_liters_per_second * dt * fluid_scale,
            });
        }
        self.fill_opening = availability.fill_reservation;
        InteractionOutcome {
            action,
            look_mouse: resolved.look_mouse,
        }
    }

    pub fn remove_plate(
        &mut self,
        item: ItemId,
        point: [f32; 3],
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if !self
            .carry
            .allows_pickup(item, player.seat().is_some(), true)
        {
            return Err(ReferenceError("Cannot remove plate".into()));
        }
        let body = self
            .attachments
            .loose_body(item)
            .ok_or_else(|| ReferenceError("Missing loose plate body".into()))?;
        self.carry.can_acquire(item, point, player, body, physics)?;
        self.attachments.remove(item, items, physics)?;
        self.carry.pickup(item, point, player, items, physics)?;
        self.parts.sync(items, physics)?;
        Ok(())
    }
    pub fn mount_plate(
        &mut self,
        item: ItemId,
        target: AttachmentTargetId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.attachments
            .can_mount(item, target, items)
            .map_err(|_| ReferenceError("Cannot mount plate".into()))?;
        self.attachments
            .preflight_mount(item, target, items, physics)?;
        if self.held() != Some(item) {
            return Err(ReferenceError("Plate is not held".into()));
        }
        self.drop_held(items, physics);
        self.attachments.mount(item, target, items, physics)?;
        self.parts.sync(items, physics)
    }
    pub fn sync_parts(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.parts.prepare_mechanisms(items, &mut self.mechanisms);
        self.mechanisms.sync(physics, true);
        self.parts
            .capture_mechanisms(items, &self.mechanisms, physics)?;
        self.attachments.sync(items, physics)?;
        self.parts.sync(items, physics)?;
        Ok(())
    }
    pub fn sync_mechanisms(&mut self, physics: &mut PhysicsWorld) {
        self.mechanisms.sync(physics, true);
    }
}

fn transform_pose(transform: &crate::reference::Transform) -> Pose {
    to_rapier(VehiclePose {
        position: transform.translation,
        rotation: transform.rotation,
    })
}
fn inverse_lerp(start: f32, end: f32, value: f32) -> f32 {
    if (end - start).abs() <= f32::EPSILON {
        0.0
    } else {
        ((value - start) / (end - start)).clamp(0.0, 1.0)
    }
}
