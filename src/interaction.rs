pub mod carry;
pub mod feedback;
pub mod items;
pub mod labels;
pub mod mechanisms;
pub mod parts;
pub mod push;
pub mod targets;
pub mod wheels;
use feedback::ActionMask;
pub use targets::{InteractionBlock, LightAim, ResolvedAim};

use crate::fluids::{spill, transfer};
use crate::player::{ExitReference, PlayerState};
use crate::reference::{
    ActionBinding, ActionMasks, BodyId, ColliderId, ItemId, ItemKind, LightControlId,
    LightControllerKind, NodeId, OpeningId, OpeningRecord, PartSlotId, ReferenceError,
    ReferenceManifest, SeatId, TankRole, WheelSlotId,
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
    WheelRemove(ItemId),
    PartRemove(ItemId),
    PartInstall(ItemId),
    WheelInstall {
        item: ItemId,
        target: WheelSlotId,
    },
    WheelSplit(ItemId),
    WheelJoin {
        rim: ItemId,
        tire: ItemId,
    },
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
    light_controls: HashMap<ColliderId, LightControlId>,
    light_nodes: HashMap<ColliderId, NodeId>,
    /// Door usable and owner per lighting controller. Only cabin controllers have links.
    cabin_doors: Vec<[Option<(ItemId, ItemId)>; 4]>,
    seat_action_latched: bool,
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
            light_nodes: reference.colliders.iter().map(|c| (c.id, c.node)).collect(),
            light_controls,
            cabin_doors,
            seat_action_latched: false,
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
                            no_drink: tank.no_drink,
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
                no_drink: true,
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
    pub(crate) fn restore_pump_valve(&mut self, item: ItemId, valve: f32) {
        if let Some(opening) = self
            .openings
            .iter_mut()
            .find(|opening| opening.reference.cap == item)
        {
            opening.valve = valve;
        }
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
        if input.held & keys::USE == 0 {
            self.seat_action_latched = false;
        }
        let availability = self.available_actions(&aim, player, items, physics);
        let light_action = availability.light_control.filter(|_| {
            input.pressed & keys::USE != 0
                || (input.pressed & keys::FILL != 0 && availability.lmb_use)
        });
        let target = aim
            .use_target
            .filter(|target| valid(*target) && availability.light_control.is_none());
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
        let mut action = light_action.map(InteractionAction::LightControl);
        let mut carry_input = input;
        self.tuning_opening = None;
        let can_use_target = availability.light_control.is_some()
            || use_target.is_some_and(|target| {
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
                self.seat_action_latched = true;
                action = Some(InteractionAction::Seat(seat.id));
            }
        } else if let Some(target) = use_target {
            if input.pressed & keys::USE != 0
                || (input.pressed & keys::FILL != 0 && availability.lmb_use)
            {
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
                self.seat_action_latched = true;
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

        let available = self.available_actions(&aim, player, items, physics);
        if input.pressed & keys::GRAB != 0 {
            self.f_gesture_consumed = false;
        }
        let grip_release = self.push.gripping() && input.pressed & keys::GRAB != 0;
        if grip_release {
            self.push.stop(player, physics);
            self.f_gesture_consumed = true;
        }
        let blocked_install = aim.installation_block.is_some()
            || (self.held().is_some() && installation_target.is_some() && available.slot.is_none());
        if input.released & keys::GRAB != 0
            && !self.f_gesture_consumed
            && !acquisition_release
            && !blocked_install
            && available.actions.0 & ActionMask::MOUNT != 0
            && self.carry.throw_charge() < self.carry.reference_throw_minimum()
        {
            if let Some(slot) = available.slot {
                let held = self.held().expect("mount held item");
                match slot {
                    SlotTarget::Wheel(slot) => {
                        self.carry.cancel_input();
                        action = Some(InteractionAction::WheelInstall {
                            item: held,
                            target: slot,
                        });
                    }
                    SlotTarget::Part(slot) => {
                        self.drop_held(items, physics);
                        self.parts
                            .install(held, slot, items, physics)
                            .expect("validated part installation");
                        self.parts.mount_mechanism(held, &mut self.mechanisms);
                        self.sync_parts(items, physics).expect("bound parts");
                        action = Some(InteractionAction::PartInstall(held));
                    }
                }
            } else if let (Some(rim), Some(tire)) = (available.rim, available.tire) {
                self.carry.cancel_input();
                action = Some(InteractionAction::WheelJoin { rim, tire });
            }
        }
        if input.pressed & keys::GRAB != 0
            && !self.carry.acquisition_held()
            && !blocked_install
            && !grip_release
            && self.held().is_none()
        {
            if available.actions.0 & ActionMask::DISMOUNT != 0 {
                if let Some(rim) = available.rim {
                    action = Some(InteractionAction::WheelSplit(rim));
                } else if let Some(target) = grab_target {
                    if self.parts.contains(target.item) {
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
                    } else {
                        action = Some(InteractionAction::WheelRemove(target.item));
                    }
                }
            } else if available.actions.0 & ActionMask::PICKUP != 0
                && let Some(target) = grab_target
                && self
                    .carry
                    .pickup(target.item, target.point, player, items, physics)
                    .is_ok()
            {
                action = Some(InteractionAction::Pickup(target.item));
            }
            if action.is_some() {
                self.f_gesture_consumed = true;
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
        if action.is_none()
            && !blocked_install
            && !acquisition_release
            && !self.f_gesture_consumed
            && self.held().is_some()
        {
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
            && available.actions.0 & ActionMask::SPLIT != 0
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
            && available.actions.0 & ActionMask::FILL != 0
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
            if item.inactive_pose.is_some() {
                continue;
            }
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
        if input.released & keys::GRAB != 0 {
            self.f_gesture_consumed = false;
        }
        self.fill_opening = availability.fill_reservation;
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
