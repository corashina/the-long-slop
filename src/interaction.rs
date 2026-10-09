pub mod items;

use crate::fluids::{spill, transfer};
use crate::player::{ExitReference, PlayerState};
use crate::reference::{
    BodyId, HingeRecord, ItemId, ItemKind, OpeningId, OpeningRecord, PlayerReference,
    ReferenceError, ReferenceManifest, SeatId, TankRole, WheelSlotId,
};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use items::{ItemLocation, ItemStore};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::{
    ColliderHandle, FixedJointBuilder, ImpulseJointHandle, QueryFilter, Ray, RigidBodyBuilder,
    RigidBodyHandle,
};
use std::collections::{HashMap, HashSet};

pub use crate::vehicle::SLOT_SENSOR_FLAG;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimTarget {
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
    reference: OpeningRecord,
    local_pose: Pose,
    tilt_axis: Vector,
    open: bool,
}
struct SeatState {
    id: SeatId,
    item: ItemId,
    pose: VehiclePose,
    exit: ExitReference,
}
struct HeldState {
    item: ItemId,
    body: RigidBodyHandle,
    offset: Vector,
    rotation: Rotation,
    joint: ImpulseJointHandle,
}

pub struct InteractionState {
    kinds: HashMap<ItemId, ItemKind>,
    bodies: HashMap<BodyId, RigidBodyHandle>,
    hinges: Vec<HingeState>,
    openings: Vec<OpeningState>,
    seats: Vec<SeatState>,
    held: Option<HeldState>,
    carry_anchor: Option<RigidBodyHandle>,
    player_body: Option<RigidBodyHandle>,
    reach: f32,
    carry: PlayerReference,
    supplies: HashSet<ItemId>,
    tank_rules: HashMap<ItemId, TankRules>,
    slow_oil_modifier: f32,
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
                reference: opening.clone(),
                local_pose,
                tilt_axis: Vector::from_array(opening.tilt_axis),
                open: opening.open,
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
        Ok(Self {
            kinds: reference
                .items
                .iter()
                .map(|item| (item.id, item.kind.clone()))
                .collect(),
            bodies: HashMap::with_capacity(reference.bodies.len()),
            hinges,
            openings,
            seats,
            held: None,
            carry_anchor: None,
            player_body: None,
            reach: reference.player.aim_reach,
            carry: reference.player.clone(),
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
        })
    }

    pub fn bind_body(&mut self, id: BodyId, handle: RigidBodyHandle) {
        self.bodies.insert(id, handle);
    }
    pub fn bind_player_body(&mut self, handle: RigidBodyHandle) {
        self.player_body = Some(handle);
    }
    pub fn held(&self) -> Option<ItemId> {
        self.held.as_ref().map(|held| held.item)
    }
    pub fn register_supply(&mut self, item: ItemId) {
        self.supplies.insert(item);
    }
    pub fn opening_is_open(&self, id: OpeningId) -> bool {
        self.openings
            .iter()
            .any(|opening| opening.reference.id == id && opening.open)
    }
    pub fn hinge_angle(&self, id: ItemId) -> Option<f32> {
        self.hinges
            .iter()
            .find(|hinge| hinge.reference.item == id)
            .map(|hinge| hinge.angle)
    }
    pub fn drop_held(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<ItemId> {
        let held = self.held.take()?;
        physics.rapier_mut().impulse_joints.remove(held.joint, true);
        if let Some(item) = items.get_mut(held.item) {
            item.location = ItemLocation::World;
        }
        Some(held.item)
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        for hinge in &mut self.hinges {
            if hinge.parent.is_none() {
                hinge.closed_pose.translation -= Vector::from_array(shift);
            }
        }
    }

    pub fn aim(&self, physics: &PhysicsWorld, view: VehiclePose, reach: f32) -> Option<AimTarget> {
        if !reach.is_finite() || reach <= 0.0 {
            return None;
        }
        let view = to_rapier(view);
        let held_body = self.held.as_ref().map(|held| held.body);
        let valid = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            (collider.parent() != held_body || held_body.is_none())
                && (self.player_body.is_none() || collider.parent() != self.player_body)
        };
        let surfaces = |handle: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            valid(handle, collider)
                && (!collider.is_sensor() || (collider.user_data as u32) != 0)
                && !collider
                    .shape()
                    .contains_point(collider.position(), view.translation)
        };
        let ray = Ray::new(view.translation, view.rotation * -Vector::Z);
        let (handle, hit) = physics
            .queries(QueryFilter::default().predicate(&surfaces))
            .cast_ray_and_get_normal(&ray, reach, true)?;
        let encoded = physics.rapier().colliders[handle].user_data as u32;
        let item = ItemId(encoded.checked_sub(1)?);
        self.kinds.get(&item)?;
        Some(AimTarget {
            item,
            distance: hit.time_of_impact,
            point: ray.point_at(hit.time_of_impact).to_array(),
            slot: (physics.rapier().colliders[handle].user_data & SLOT_SENSOR_FLAG != 0).then_some(
                WheelSlotId((physics.rapier().colliders[handle].user_data >> 96) as u8),
            ),
        })
    }

    pub fn step(
        &mut self,
        input: FrameInput,
        target: Option<AimTarget>,
        player: &mut PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> Option<InteractionAction> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        let target = target.filter(|target| {
            target.distance.is_finite() && target.distance >= 0.0 && target.distance <= self.reach
        });
        let mut action = None;
        if input.pressed & keys::USE != 0 && player.seat().is_some() {
            if let Some(seat) = self
                .seats
                .iter()
                .find(|seat| Some(seat.id) == player.seat())
                && player.try_exit(physics, &seat.exit)
            {
                action = Some(InteractionAction::Seat(seat.id));
            }
        } else if let Some(target) = target {
            if input.pressed & keys::USE != 0 {
                for opening in &mut self.openings {
                    if opening.reference.cap == target.item {
                        opening.open = !opening.open;
                        action = Some(InteractionAction::Cap(opening.reference.id));
                        break;
                    }
                }
            }
            if input.released & keys::USE != 0
                && self.held.is_none()
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
        if input.pressed & keys::GRAB != 0 {
            if let Some(held) = self.held.as_ref()
                && let Some(slot) = target.and_then(|target| target.slot)
                && matches!(
                    self.kinds.get(&held.item),
                    Some(ItemKind::Wheel | ItemKind::Rim)
                )
            {
                action = Some(InteractionAction::WheelInstall {
                    item: held.item,
                    target: slot,
                });
            } else if self.held.is_none()
                && let Some(target) = target
                && matches!(
                    self.kinds.get(&target.item),
                    Some(ItemKind::Wheel | ItemKind::Rim)
                )
                && items
                    .get(target.item)
                    .is_some_and(|item| matches!(item.location, ItemLocation::WheelSlot(_)))
            {
                action = Some(InteractionAction::WheelRemove(target.item));
            } else if let Some(item) = self.drop_held(items, physics) {
                action = Some(InteractionAction::Drop(item));
            } else if let Some(target) = target
                && self.pickup(target, player, items, physics)
            {
                action = Some(InteractionAction::Pickup(target.item));
            }
        }
        if input.pressed & keys::ASSEMBLE != 0
            && let Some(item) = self.held().or_else(|| target.map(|target| target.item))
            && matches!(self.kinds.get(&item), Some(ItemKind::Wheel | ItemKind::Rim))
            && items.get(item).is_some_and(|item| {
                matches!(item.location, ItemLocation::Held | ItemLocation::World)
            })
        {
            action = Some(InteractionAction::WheelSplit(item));
        }
        if self.update_carry(input, player, physics, dt)
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
            && let Some(held) = self.held.as_ref()
            && let Some(target) = target
        {
            if let Some(opening) = self
                .openings
                .iter()
                .find(|opening| opening.reference.cap == target.item && opening.open)
            {
                let (source_id, destination, flow) =
                    if self.supplies.contains(&opening.reference.tank) {
                        if !self.openings.iter().any(|destination| {
                            destination.reference.tank == held.item && destination.open
                        }) {
                            (held.item, held.item, 0.0)
                        } else {
                            (
                                opening.reference.tank,
                                held.item,
                                opening.reference.flow_liters_per_second,
                            )
                        }
                    } else {
                        (
                            held.item,
                            opening.reference.tank,
                            opening.reference.flow_liters_per_second,
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
            if !opening.open
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
                    factor * opening.reference.flow_liters_per_second * dt,
                );
            }
        }
        action
    }

    pub fn sync_hinges(&self, physics: &mut PhysicsWorld) {
        self.place_hinges(physics, true);
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
            let rotation = Pose::from_rotation(Rotation::from_axis_angle(
                hinge.axis,
                (hinge.angle - hinge.reference.closed_degrees).to_radians(),
            ));
            let pose = parent * hinge.closed_pose * hinge.anchor * rotation * hinge.anchor_inverse;
            let body = &mut physics.rapier_mut().bodies[handle];
            if commit {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
        }
    }

    fn pickup(
        &mut self,
        target: AimTarget,
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> bool {
        if !matches!(
            self.kinds.get(&target.item),
            Some(ItemKind::Container | ItemKind::Wheel | ItemKind::Rim | ItemKind::Tire)
        ) {
            return false;
        }
        let Some(item) = items.get_mut(target.item) else {
            return false;
        };
        if item.location != ItemLocation::World {
            return false;
        }
        let Some(body) = item.body else {
            return false;
        };
        if body == physics.chassis_handle() || !physics.rapier().bodies[body].is_dynamic() {
            return false;
        }
        let pose = *physics.rapier().bodies[body].position();
        let anchor = *self.carry_anchor.get_or_insert_with(|| {
            physics
                .rapier_mut()
                .insert_body(RigidBodyBuilder::kinematic_position_based().pose(pose))
        });
        physics.rapier_mut().bodies[anchor].set_position(pose, false);
        let joint = physics.rapier_mut().impulse_joints.insert(
            anchor,
            body,
            FixedJointBuilder::new(),
            true,
        );
        let view = to_rapier(player.view_pose(physics));
        physics.rapier_mut().bodies[body].enable_ccd(true);
        self.held = Some(HeldState {
            item: target.item,
            body,
            offset: (view.rotation.inverse() * (pose.translation - view.translation)
                - Vector::from_array(self.carry.carry_anchor_offset))
            .clamp_length_max(self.carry.carry_offset_radius),
            rotation: view.rotation.inverse() * pose.rotation,
            joint,
        });
        item.location = ItemLocation::Held;
        true
    }

    fn update_carry(
        &mut self,
        input: FrameInput,
        player: &PlayerState,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> bool {
        let Some(held) = self.held.as_mut() else {
            return false;
        };
        let Some(anchor) = self.carry_anchor else {
            return false;
        };
        let view = to_rapier(player.view_pose(physics));
        if input.wheel.is_finite() {
            held.offset = (held.offset
                - Vector::Z * input.wheel * dt * self.carry.obj_move_sensitivity)
                .clamp_length_max(self.carry.carry_offset_radius);
        }
        if input.held & keys::ROTATE != 0 && input.mouse.iter().all(|delta| delta.is_finite()) {
            let sensitivity = dt * 10.0 * self.carry.obj_rotation_sensitivity;
            held.rotation = Rotation::from_rotation_y((-input.mouse[0] * sensitivity).to_radians())
                * held.rotation
                * Rotation::from_rotation_x((input.mouse[1] * sensitivity).to_radians());
        }
        let current = *physics.rapier().bodies[held.body].position();
        let mut desired = Pose::from_parts(
            view.translation
                + view.rotation
                    * (Vector::from_array(self.carry.carry_anchor_offset) + held.offset)
                        .clamp_length(self.carry.carry_min_distance, self.carry.carry_max_distance),
            view.rotation * held.rotation,
        );
        if (desired.translation - current.translation).length() >= self.carry.carry_drop_distance {
            return true;
        }
        desired.translation = current.translation
            + ((desired.translation - current.translation) * self.carry.carry_force_multiplier)
                .clamp_length_max(self.carry.carry_max_force)
                * physics.rapier().integration_parameters.dt;
        let outside_player = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.parent() != Some(player.body_handle())
        };
        let queries = physics.queries(
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(held.body)
                .predicate(&outside_player),
        );
        for collider_handle in physics.rapier().bodies[held.body].colliders() {
            let collider = &physics.rapier().colliders[*collider_handle];
            let local = collider
                .position_wrt_parent()
                .copied()
                .unwrap_or(Pose::IDENTITY);
            let velocity = desired.translation - current.translation;
            if let Some((_, hit)) = queries.cast_shape(
                &(current * local),
                velocity,
                collider.shape(),
                ShapeCastOptions {
                    max_time_of_impact: 1.0,
                    ..Default::default()
                },
            ) {
                desired.translation =
                    current.translation + velocity * (hit.time_of_impact - 0.01).max(0.0);
            }
            if queries
                .intersect_shape(desired * local, collider.shape())
                .next()
                .is_some()
            {
                desired.rotation = current.rotation;
            }
        }
        physics.rapier_mut().bodies[anchor].set_next_kinematic_position(desired);
        false
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
