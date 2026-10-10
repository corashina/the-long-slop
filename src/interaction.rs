pub mod carry;
pub mod items;
pub mod targets;
pub mod wheels;
pub use targets::{InteractionBlock, ResolvedAim};

use crate::fluids::{spill, transfer};
use crate::player::{ExitReference, PlayerState};
use crate::reference::{
    ActionBinding, ActionMasks, BodyId, ColliderId, HingeRecord, ItemId, ItemKind, OpeningId,
    OpeningRecord, ReferenceError, ReferenceManifest, SeatId, TankRole, WheelSlotId,
};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use items::{ItemLocation, ItemStore};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::{HashMap, HashSet};

pub use crate::vehicle::SLOT_SENSOR_FLAG;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimTarget {
    pub collider: ColliderHandle,
    pub item: ItemId,
    pub distance: f32,
    pub point: [f32; 3],
    pub slot: Option<WheelSlotId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractionAction {
    Pickup(ItemId),
    Drop(ItemId),
    Cap(OpeningId),
    Hinge(ItemId),
    Seat(SeatId),
    Fill {
        source: ItemId,
        destination: ItemId,
        liters: f32,
    },
    WheelRemove(ItemId),
    WheelInstall {
        item: ItemId,
        target: WheelSlotId,
    },
    WheelSplit(ItemId),
}

struct HingeState {
    reference: HingeRecord,
    parent: Option<BodyId>,
    closed_pose: Pose,
    anchor: Pose,
    anchor_inverse: Pose,
    axis: Vector,
    drag_body: BodyId,
    drag_pose: Pose,
    drag_axis: Vector,
    angle: f32,
    target: f32,
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
    hinges: Vec<HingeState>,
    openings: Vec<OpeningState>,
    seats: Vec<SeatState>,
    carry: carry::CarryState,
    player_body: Option<RigidBodyHandle>,
    reach: f32,
    action_masks: ActionMasks,
    scroll_sensitivity: f32,
    supplies: HashSet<ItemId>,
    tank_rules: HashMap<ItemId, TankRules>,
    slow_oil_modifier: f32,
    latched_hinge: Option<(usize, Vector, ColliderHandle)>,
    hinge_release_distance_squared: f32,
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
        let mut hinges = Vec::with_capacity(reference.hinges.len());
        for hinge in &reference.hinges {
            let body = reference
                .bodies
                .iter()
                .find(|body| body.id == hinge.body)
                .ok_or_else(|| ReferenceError("Missing hinge body".into()))?;
            let anchor = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == hinge.anchor)
                .ok_or_else(|| ReferenceError("Missing hinge anchor".into()))?;
            let drag = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == hinge.drag_anchor)
                .ok_or_else(|| ReferenceError("Missing hinge drag anchor".into()))?;
            let mut closed_pose = transform_pose(&body.pose);
            if let Some(parent) = body.parent_body {
                let parent = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == parent)
                    .ok_or_else(|| ReferenceError("Missing hinge parent".into()))?;
                closed_pose = transform_pose(&parent.pose).inverse() * closed_pose;
            }
            hinges.push(HingeState {
                reference: hinge.clone(),
                parent: body.parent_body,
                closed_pose,
                anchor: transform_pose(&anchor.transform),
                anchor_inverse: transform_pose(&anchor.transform).inverse(),
                axis: Vector::from_array(hinge.axis).normalize(),
                drag_body: drag.body,
                drag_pose: transform_pose(&drag.transform),
                drag_axis: Vector::from_array(hinge.drag_axis),
                angle: hinge.closed_degrees,
                target: hinge.closed_degrees,
            });
        }
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
            .hinges
            .iter()
            .map(|hinge| hinge.item)
            .chain(reference.seats.iter().map(|seat| seat.item))
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
            hinges,
            openings,
            seats,
            carry: carry::CarryState::new(reference)?,
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
            latched_hinge: None,
            hinge_release_distance_squared: (reference.player.aim_reach * 1.5).powi(2),
            tuning_opening: None,
        })
    }

    pub fn bind_body(&mut self, id: BodyId, handle: RigidBodyHandle) {
        self.bodies.insert(id, handle);
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
    pub fn hinge_angle(&self, id: ItemId) -> Option<f32> {
        self.hinges
            .iter()
            .find(|hinge| hinge.reference.item == id)
            .map(|hinge| hinge.angle)
    }
    pub fn dragging_hinge(&self) -> Option<ItemId> {
        self.latched_hinge
            .map(|(index, _, _)| self.hinges[index].reference.item)
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
        for hinge in &mut self.hinges {
            if hinge.parent.is_none() {
                hinge.closed_pose.translation -= Vector::from_array(shift);
            }
        }
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
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        self.carry.release_acquisition(input);
        let valid = |target: AimTarget| {
            target.distance.is_finite() && target.distance >= 0.0 && target.distance <= self.reach
        };
        let target = aim.use_target.filter(|target| valid(*target));
        let grab_target = aim.grab_target.filter(|target| valid(*target));
        let installation_target = aim.installation_target.filter(|target| valid(*target));
        let filling_target = aim.filling_target.filter(|target| valid(*target));
        let released_hinge = input.released & keys::USE != 0 && self.latched_hinge.is_some();
        if input.held & keys::USE == 0 || input.released & keys::USE != 0 {
            self.latched_hinge = None;
        } else if self.latched_hinge.is_none()
            && let Some(target) = target
            && let Some(index) = self
                .hinges
                .iter()
                .position(|hinge| hinge.reference.item == target.item)
            && let Some(body) = self
                .bodies
                .get(&self.hinges[index].reference.body)
                .and_then(|handle| physics.rapier().bodies.get(*handle))
        {
            self.latched_hinge = Some((
                index,
                body.position().inverse() * Vector::from_array(target.point),
                target.collider,
            ));
        }
        let use_target = if let Some((index, local, collider)) = self.latched_hinge {
            let hinge = &self.hinges[index];
            if let Some(body) = self
                .bodies
                .get(&hinge.reference.body)
                .and_then(|handle| physics.rapier().bodies.get(*handle))
            {
                let point = *body.position() * local;
                let distance = (point - Vector::from_array(player.body_pose(physics).position))
                    .length_squared();
                if distance < self.hinge_release_distance_squared {
                    Some(AimTarget {
                        collider,
                        item: hinge.reference.item,
                        distance: distance.sqrt(),
                        point: point.to_array(),
                        slot: None,
                    })
                } else {
                    self.latched_hinge = None;
                    None
                }
            } else {
                self.latched_hinge = None;
                None
            }
        } else {
            target
        };
        let mut action = None;
        let mut carry_input = input;
        self.tuning_opening = None;
        let can_use_target = use_target.is_some_and(|target| {
            self.hinges
                .iter()
                .any(|hinge| hinge.reference.item == target.item)
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
                && !released_hinge
                && self.held().is_none()
                && let Some(seat) = self.seats.iter().find(|seat| seat.item == target.item)
                && player.enter(seat.id, seat.pose, physics)
            {
                action = Some(InteractionAction::Seat(seat.id));
            }
            if input.held & keys::USE != 0
                && let Some(hinge) = self
                    .hinges
                    .iter_mut()
                    .find(|hinge| hinge.reference.item == target.item)
            {
                let axis = hinge.reference.drag_input_axis as usize;
                let mut delta = input.mouse[axis];
                if hinge.reference.relative_drag {
                    if let Some(body) = self
                        .bodies
                        .get(&hinge.drag_body)
                        .and_then(|handle| physics.rapier().bodies.get(*handle))
                    {
                        let view = to_rapier(player.view_pose(physics));
                        let direction =
                            (body.position().rotation * hinge.drag_pose.rotation) * hinge.drag_axis;
                        delta = (view.rotation
                            * (-Vector::Z * input.mouse[1] + Vector::X * input.mouse[0]))
                            .dot(direction);
                    } else {
                        delta = 0.0;
                    }
                }
                let sign = if axis == 0 {
                    if hinge.reference.input_inverted[0] {
                        1.0
                    } else {
                        -1.0
                    }
                } else if hinge.reference.input_inverted[1] {
                    -1.0
                } else {
                    1.0
                };
                delta *= hinge.reference.drag_sensitivity * sign;
                if delta.is_finite() {
                    hinge.target = (hinge.target + delta)
                        .clamp(hinge.reference.min_degrees, hinge.reference.max_degrees);
                    action = Some(InteractionAction::Hinge(target.item));
                }
            }
        }
        if input.pressed & keys::GRAB != 0
            && !self.carry.acquisition_held()
            && self.wheel_installation_block(&aim).is_none()
        {
            if let Some(held) = self.held()
                && let Some(slot) = installation_target.and_then(|target| target.slot)
            {
                action = Some(InteractionAction::WheelInstall {
                    item: held,
                    target: slot,
                });
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
                {
                    action = Some(InteractionAction::WheelRemove(target.item));
                }
            } else if let Some(item) = self.drop_held(items, physics) {
                action = Some(InteractionAction::Drop(item));
            } else if let Some(target) = grab_target
                && target.slot.is_none()
                && self
                    .carry
                    .pickup(target.item, target.point, player, items, physics)
                    .is_ok()
            {
                action = Some(InteractionAction::Pickup(target.item));
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
        for hinge in &mut self.hinges {
            if let Some(speed) = hinge.reference.motor_degrees_per_second {
                let max = speed * dt;
                hinge.angle += (hinge.target - hinge.angle).clamp(-max, max);
            } else {
                hinge.angle = hinge.target;
            }
        }
        self.place_hinges(physics, false);
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

    pub fn sync_hinges(&self, physics: &mut PhysicsWorld) {
        self.place_hinges(physics, true);
    }

    pub fn hinge_local_pose(&self, body: BodyId) -> Option<Pose> {
        self.hinges
            .iter()
            .find(|hinge| hinge.reference.body == body)
            .map(local_hinge_pose)
    }

    fn place_hinges(&self, physics: &mut PhysicsWorld, commit: bool) {
        for hinge in &self.hinges {
            let Some(handle) = self.bodies.get(&hinge.reference.body).copied() else {
                continue;
            };
            let parent = if let Some(parent) = hinge.parent {
                let Some(body) = self
                    .bodies
                    .get(&parent)
                    .and_then(|handle| physics.rapier().bodies.get(*handle))
                else {
                    continue;
                };
                *body.position()
            } else {
                Pose::IDENTITY
            };
            let pose = parent * local_hinge_pose(hinge);
            let body = &mut physics.rapier_mut().bodies[handle];
            if commit {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
        }
    }
}

fn local_hinge_pose(hinge: &HingeState) -> Pose {
    let rotation = Pose::from_rotation(Rotation::from_axis_angle(
        hinge.axis,
        (hinge.angle - hinge.reference.closed_degrees).to_radians(),
    ));
    hinge.closed_pose * hinge.anchor * rotation * hinge.anchor_inverse
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
