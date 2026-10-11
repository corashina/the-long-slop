pub mod carry;
pub mod items;
pub mod labels;
pub mod lighting;
pub mod mechanisms;
pub mod parts;
pub mod push;
pub mod targets;
pub mod wheels;
pub use targets::{InteractionBlock, ResolvedAim};

use crate::fluids::{spill, transfer};
use crate::player::{ExitReference, PlayerState};
use crate::reference::{
    ActionBinding, ActionMasks, BodyId, ColliderId, ItemId, ItemKind, OpeningId, OpeningRecord,
    PartSlotId, ReferenceError, ReferenceManifest, SeatId, TankRole, WheelSlotId,
};
use crate::simulation::{FrameInput, keys};
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
    Part(PartSlotId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractionAction {
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
    WheelRemove(ItemId),
    PartRemove(ItemId),
    PartInstall(ItemId),
    WheelInstall {
        item: ItemId,
        target: WheelSlotId,
    },
    WheelSplit(ItemId),
}

struct OpeningState {
    reference: OpeningRules,
    local_pose: Pose,
    tilt_axis: Vector,
    valve: f32,
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
    kinds: HashMap<ItemId, ItemKind>,
    source_bindings: HashMap<ColliderId, ActionBinding>,
    pickup_layers: HashMap<ItemId, u32>,
    removal_layers: HashMap<ItemId, u32>,
    usable_items: HashSet<ItemId>,
    filling_items: HashSet<ItemId>,
    collider_bindings: HashMap<ColliderHandle, ActionBinding>,
    bodies: HashMap<BodyId, RigidBodyHandle>,
    pub mechanisms: mechanisms::MechanismState,
    pub parts: parts::PartAttachments,
    mechanism_bodies: HashMap<ItemId, BodyId>,
    openings: Vec<OpeningState>,
    seats: Vec<SeatState>,
    carry: carry::CarryState,
    push: push::PushState,
    player_body: Option<RigidBodyHandle>,
    reach: f32,
    action_masks: ActionMasks,
    scroll_sensitivity: f32,
    supplies: HashSet<ItemId>,
    tank_rules: HashMap<ItemId, TankRules>,
    slow_oil_modifier: f32,
    latched_mechanism: Option<(ItemId, BodyId, Vector, ColliderHandle)>,
    use_consumed: bool,
    mechanism_release_distance_squared: f32,
    tuning_opening: Option<OpeningId>,
}

#[derive(Clone, Copy)]
struct TankRules {
    no_fill_this: bool,
    no_fill_from: bool,
    slow_oil: bool,
}

impl InteractionState {
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
        Ok(Self {
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
            mechanism_bodies: reference.items.iter().map(|i| (i.id, i.body)).collect(),
            openings,
            seats,
            carry: carry::CarryState::new(reference)?,
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
                            no_fill_this: tank.no_fill_this,
                            no_fill_from: tank.no_fill_from,
                            slow_oil: tank.slow_oil,
                        },
                    )
                })
                .collect(),
            slow_oil_modifier: reference.fluid_rules.slow_oil_modifier,
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
    pub fn held(&self) -> Option<ItemId> {
        self.carry.held()
    }
    pub fn register_supply(&mut self, item: ItemId) {
        self.supplies.insert(item);
    }
    pub fn register_pump(&mut self, item: ItemId, flow: f32) -> Result<(), ReferenceError> {
        if !flow.is_finite() || flow <= 0.0 {
            return Err(ReferenceError("Invalid pump flow".into()));
        }
        if let Some(opening) = self
            .openings
            .iter_mut()
            .find(|opening| opening.reference.cap == item)
        {
            if !self.supplies.contains(&item) {
                return Err(ReferenceError("Pump ID overlaps another opening".into()));
            }
            opening.reference.flow_liters_per_second = flow;
            return Ok(());
        }
        self.kinds.insert(item, ItemKind::Cap);
        self.filling_items.insert(item);
        self.tank_rules.insert(
            item,
            TankRules {
                no_fill_this: true,
                no_fill_from: false,
                slow_oil: false,
            },
        );
        self.supplies.insert(item);
        self.openings.push(OpeningState {
            reference: OpeningRules {
                id: OpeningId(item.0),
                tank: item,
                cap: item,
                flow_liters_per_second: flow,
                tilt_degrees: 0.0,
                angle_disabled: true,
                spill_disabled: true,
                tube_connected: false,
                can_toggle: false,
                can_tune: false,
                tune_rate: 0.0,
            },
            local_pose: Pose::IDENTITY,
            tilt_axis: Vector::Y,
            valve: 1.0,
        });
        Ok(())
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
    pub fn push_steering(&self) -> Option<f32> {
        self.push.steering()
    }
    pub fn push_gripping(&self) -> bool {
        self.push.gripping()
    }
    pub fn cancel_input(&mut self) {
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
        self.carry.drop(items, physics)
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

    pub fn rebase(&mut self, shift: [f32; 3]) {
        self.mechanisms.rebase(shift);
        self.parts.rebase(shift);
    }

    pub fn step(
        &mut self,
        input: FrameInput,
        aim: ResolvedAim,
        player: &mut PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> Option<InteractionAction> {
        self.finish_cancelled_input(player, physics);
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        let acquisition_release = self.carry.acquisition_held();
        self.carry.release_acquisition(input);
        let reach = self.reach;
        let valid = |target: AimTarget| {
            target.distance.is_finite() && target.distance >= 0.0 && target.distance <= reach
        };
        let target = aim.use_target.filter(|target| valid(*target));
        let grab_target = aim.grab_target.filter(|target| valid(*target));
        let installation_target = aim.installation_target.filter(|target| valid(*target));
        let filling_target = aim.filling_target.filter(|target| valid(*target));
        self.mechanisms.sync(physics, false);
        let released_mechanism = input.released & keys::USE != 0 && self.use_consumed;
        if input.held & keys::USE == 0 || input.released & keys::USE != 0 {
            self.latched_mechanism = None;
            self.use_consumed = false;
        } else if self.latched_mechanism.is_none()
            && let Some(target) = target
            && self.mechanisms.contains(target.item)
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
        let use_target = if let Some((item, body_id, local, collider)) = self.latched_mechanism {
            if self.mechanisms.contains(item)
                && let Some(body) = self
                    .bodies
                    .get(&body_id)
                    .and_then(|handle| physics.rapier().bodies.get(*handle))
            {
                let point = *body.position() * local;
                let distance = (point - Vector::from_array(player.body_pose(physics).position))
                    .length_squared();
                if distance < self.mechanism_release_distance_squared {
                    Some(AimTarget {
                        collider,
                        item,
                        distance: distance.sqrt(),
                        point: point.to_array(),
                        slot: None,
                    })
                } else {
                    self.latched_mechanism = None;
                    None
                }
            } else {
                self.latched_mechanism = None;
                None
            }
        } else {
            target
        };
        let mut action = None;
        let mut carry_input = input;
        self.tuning_opening = None;
        let can_use_target = use_target.is_some_and(|target| {
            self.mechanisms.contains(target.item)
                || self.openings.iter().any(|opening| {
                    opening.reference.cap == target.item
                        && (opening.reference.can_toggle || opening.reference.can_tune)
                })
        });
        if input.pressed & keys::USE != 0 && player.seat().is_some() && !can_use_target {
            if let Some(seat) = self
                .seats
                .iter()
                .find(|seat| Some(seat.id) == player.seat())
                && player.try_exit(physics, &seat.exit)
            {
                action = Some(InteractionAction::Seat(seat.id));
            }
        } else if let Some(target) = use_target {
            if input.pressed & keys::USE != 0 {
                for opening in &mut self.openings {
                    if opening.reference.cap == target.item && opening.reference.can_toggle {
                        opening.valve = if opening.valve > 0.0 { 0.0 } else { 1.0 };
                        action = Some(InteractionAction::Cap(opening.reference.id));
                        break;
                    }
                }
            }
            if let Some(opening) = self
                .openings
                .iter_mut()
                .find(|opening| opening.reference.cap == target.item && opening.reference.can_tune)
            {
                carry_input.wheel = 0.0;
                let delta = if input.wheel.is_finite() && input.wheel != 0.0 {
                    input.wheel * self.scroll_sensitivity
                } else if input.held & keys::USE != 0 {
                    input.mouse[1]
                } else {
                    0.0
                };
                if input.held & keys::USE != 0 || delta != 0.0 {
                    self.tuning_opening = Some(opening.reference.id);
                }
                if delta.is_finite() && delta != 0.0 {
                    opening.valve =
                        (opening.valve + delta * dt * opening.reference.tune_rate).clamp(0.0, 1.0);
                    if opening.valve < 1e-6 {
                        opening.valve = 0.0;
                    }
                    action = Some(InteractionAction::Cap(opening.reference.id));
                }
            }
            if input.released & keys::USE != 0
                && !released_mechanism
                && self.held().is_none()
                && let Some(seat) = self.seats.iter().find(|seat| seat.item == target.item)
                && player.enter(seat.id, seat.pose, physics)
            {
                action = Some(InteractionAction::Seat(seat.id));
            }
            if self.mechanisms.contains(target.item)
                && (input.pressed & keys::USE != 0
                    || (input.pressed & keys::FILL != 0
                        && self.mechanisms.primary_switch(target.item))
                    || input.held & keys::USE != 0
                    || (input.wheel != 0.0 && self.mechanisms.scroll_eligible(target.item)))
            {
                self.mechanisms
                    .step(target.item, input, player.view_pose(physics), dt);
                carry_input.wheel = 0.0;
                action = Some(InteractionAction::Mechanism(target.item));
            }
        }

        let grip_release = self.push.gripping() && input.pressed & keys::GRAB != 0;
        if grip_release {
            self.push.stop(player, physics);
        }
        let blocked_install = self.wheel_installation_block(&aim).is_some()
            || (self.held().is_some() && installation_target.is_some_and(|target| {
                matches!(target.slot, Some(SlotTarget::Part(slot)) if self.parts.can_install(self.held().unwrap(), slot, items).is_err())
            }));
        if input.pressed & keys::GRAB != 0
            && !self.carry.acquisition_held()
            && !blocked_install
            && !grip_release
        {
            if let Some(held) = self.held()
                && let Some(slot) = installation_target.and_then(|target| target.slot)
            {
                match slot {
                    SlotTarget::Wheel(slot) => {
                        self.carry.cancel_input();
                        action = Some(InteractionAction::WheelInstall {
                            item: held,
                            target: slot,
                        })
                    }
                    SlotTarget::Part(slot) => {
                        if self.parts.can_install(held, slot, items).is_ok() {
                            self.drop_held(items, physics);
                            self.parts
                                .install(held, slot, items, physics)
                                .expect("validated part installation");
                            self.parts.mount_mechanism(held, &mut self.mechanisms);
                            self.sync_parts(items, physics).expect("bound parts");
                            action = Some(InteractionAction::PartInstall(held));
                        }
                    }
                }
            } else if self.held().is_none()
                && let Some(target) = grab_target
                && self.parts.contains(target.item)
                && matches!(
                    items.get(target.item).map(|i| i.location),
                    Some(ItemLocation::PartSlot(_))
                )
            {
                if self.parts.can_remove(target.item, player, items).is_ok()
                    && self.parts.loose_body(target.item).is_some_and(|body| {
                        self.carry
                            .can_acquire(target.item, target.point, player, body, physics)
                            .is_ok()
                    })
                {
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
                }
            } else if self.held().is_none()
                && let Some(target) = grab_target
                && matches!(
                    self.kinds.get(&target.item),
                    Some(ItemKind::Wheel | ItemKind::Rim)
                )
                && items
                    .get(target.item)
                    .is_some_and(|item| matches!(item.location, ItemLocation::WheelSlot(_)))
            {
                if self
                    .carry
                    .allows_pickup(target.item, player.seat().is_some(), true)
                    && !self.parts.covered(target.item)
                {
                    action = Some(InteractionAction::WheelRemove(target.item));
                }
            } else if self.held().is_none()
                && let Some(target) = grab_target
                && target.slot.is_none()
                && self
                    .carry
                    .pickup(target.item, target.point, player, items, physics)
                    .is_ok()
            {
                action = Some(InteractionAction::Pickup(target.item));
            }
        }
        if blocked_install
            && (self.carry.throw_active() || (input.held | input.pressed) & keys::GRAB != 0)
        {
            self.carry.cancel_input();
        }
        let push_target = aim.push_target.filter(|target| valid(*target));
        let pushing = if (action.is_none() || self.push.gripping())
            && self.held().is_none()
            && !blocked_install
            && !grip_release
        {
            self.push
                .step(input, push_target, &self.bodies, player, physics, dt)
        } else {
            self.push.stop(player, physics);
            None
        };
        if action.is_none()
            && let Some(item) = pushing
        {
            action = Some(InteractionAction::Push(item));
        }
        if action.is_none() && !blocked_install && !acquisition_release && self.held().is_some() {
            if input.pressed & keys::GRAB != 0 {
                self.carry.begin_throw();
            }
            if input.held & keys::GRAB != 0 {
                self.carry.charge_throw(dt);
            }
            if input.released & keys::GRAB != 0 {
                let throwing = self.carry.throw_charge() > self.carry.reference_throw_minimum();
                if let Some(item) = self.carry.release_throw(player, items, physics) {
                    action = Some(if throwing {
                        InteractionAction::Throw(item)
                    } else {
                        InteractionAction::Drop(item)
                    });
                }
            }
        }
        if input.pressed & keys::ASSEMBLE != 0
            && let Some(item) = self
                .held()
                .or_else(|| grab_target.map(|target| target.item))
            && matches!(self.kinds.get(&item), Some(ItemKind::Wheel | ItemKind::Rim))
            && items.get(item).is_some_and(|item| {
                matches!(item.location, ItemLocation::Held | ItemLocation::World)
            })
        {
            action = Some(InteractionAction::WheelSplit(item));
        }
        if self.carry.step(carry_input, player, physics, dt)
            && let Some(item) = self.drop_held(items, physics)
        {
            action = Some(InteractionAction::Drop(item));
        }
        self.mechanisms.sync(physics, false);
        items.sync_poses(physics);
        let mut filling = None;
        if input.held & keys::FILL != 0
            && let Some(held) = self.held()
            && let Some(target) = filling_target
        {
            if let Some(opening) = self
                .openings
                .iter()
                .find(|opening| opening.reference.cap == target.item && opening.valve > 0.0)
            {
                let (source_id, destination, flow) =
                    if self.supplies.contains(&opening.reference.tank) {
                        if !self.openings.iter().any(|destination| {
                            destination.reference.tank == held && destination.valve > 0.0
                        }) {
                            (held, held, 0.0)
                        } else {
                            (
                                opening.reference.tank,
                                held,
                                opening.reference.flow_liters_per_second * opening.valve,
                            )
                        }
                    } else {
                        (
                            held,
                            opening.reference.tank,
                            opening.reference.flow_liters_per_second * opening.valve,
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
                    let liters = transfer(source, destination_state, amount);
                    if liters > 0.0 {
                        filling = Some(source_id);
                        action = Some(InteractionAction::Fill {
                            source: source_id,
                            destination,
                            liters,
                        });
                    }
                }
            }
        }
        for opening in &self.openings {
            if opening.valve <= 0.0
                || opening.reference.spill_disabled
                || opening.reference.tube_connected
                || filling == Some(opening.reference.tank)
            {
                continue;
            }
            let Some(item) = items.get_mut(opening.reference.tank) else {
                continue;
            };
            let forward = (to_rapier(item.pose) * opening.local_pose).rotation * opening.tilt_axis;
            let angle = forward.dot(Vector::Y).clamp(-1.0, 1.0).acos().to_degrees();
            if let Some(fluid) = item.fluid.as_mut() {
                let fill = (fluid.mixture.total() / fluid.capacity_liters).clamp(0.0, 1.0);
                let threshold = 100.0 + (opening.reference.tilt_degrees - 100.0) * fill;
                let factor = if opening.reference.angle_disabled {
                    1.0
                } else {
                    inverse_lerp(threshold, opening.reference.tilt_degrees + 90.0, angle)
                        .max(inverse_lerp(90.0, 180.0, angle))
                };
                if !opening.reference.angle_disabled && angle <= threshold {
                    continue;
                }
                spill(
                    fluid,
                    factor * opening.valve * opening.reference.flow_liters_per_second * dt,
                );
            }
        }
        action
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
