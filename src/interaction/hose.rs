//! Siphon hose (`tubescript`): rules and runtime.

use super::feedback::HosePrompt;
use super::items::{ItemLocation, ItemStore};
use super::level::{inverse_lerp, lerp};
use super::{AimTarget, InteractionState, OpeningState};
use crate::fluids::transfer;
use crate::reference::{HOSE_SEGMENTS, ItemId, ItemPrototype, OpeningId, ReferenceError};
use crate::vehicle::{PhysicsWorld, to_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ImpulseJointHandle, RigidBodyHandle, SphericalJointBuilder};

/// Source `tubescript.Update`: the cap joint cannot break for this time after attach.
const BREAK_DELAY: f32 = 0.5;
/// Source `CoilingUp` lerp rate.
const COIL_RATE: f32 = 15.0;

/// Fluid moved this frame between two attached ends.
#[allow(clippy::too_many_arguments)]
pub(crate) fn siphon_rate(
    height_diff: f32,
    min_h: f32,
    max_h: f32,
    valve1: f32,
    flow1: f32,
    valve2: f32,
    flow2: f32,
    pour_speed: f32,
    dt: f32,
) -> f32 {
    (valve1 * flow1).min(valve2 * flow2)
        * pour_speed
        * dt
        * inverse_lerp(min_h, max_h, height_diff.abs())
}

/// Returns `(source, receiver)` end indices. The higher end is the source.
pub(crate) fn siphon_direction(y1: f32, y2: f32) -> (usize, usize) {
    if y1 > y2 { (0, 1) } else { (1, 0) }
}

/// End index that attaches to the cap, as `TryAttach`.
pub(crate) fn pick_end(
    dist_sq_t1: f32,
    dist_sq_t2: f32,
    t1_attached: bool,
    t2_attached: bool,
    t1_cap_is_target: bool,
    t2_cap_is_target: bool,
) -> Option<usize> {
    if t1_cap_is_target || t2_cap_is_target {
        return None;
    }
    let (near, far, near_free, far_free) = if dist_sq_t1 < dist_sq_t2 {
        (0, 1, !t1_attached, !t2_attached)
    } else {
        (1, 0, !t2_attached, !t1_attached)
    };
    if near_free {
        Some(near)
    } else if far_free {
        Some(far)
    } else {
        None
    }
}

/// A cap accepts a hose end.
pub(crate) fn cap_accepts(valve: f32, cant_tube: bool, tube_flag: bool) -> bool {
    valve > 0.0 && !cant_tube && !tube_flag
}

/// Owncap valve step toward `target`.
pub(crate) fn ease_valve(v: f32, target: f32, dt: f32) -> f32 {
    lerp(v, target, dt * 5.0)
}

#[derive(Clone, Copy)]
enum Link {
    /// Ball joint to the cap body.
    Joint(ImpulseJointHandle, RigidBodyHandle),
    /// The cap has no body. The end snaps to the cap each frame, as the source `ReAttach`.
    Pinned,
}

struct End {
    segment: usize,
    /// The end's own cap (`owncap`).
    own: OpeningId,
    cap: Option<(OpeningId, Link)>,
    attached_at: f32,
}

struct Hose {
    items: [ItemId; HOSE_SEGMENTS],
    bodies: [RigidBodyHandle; HOSE_SEGMENTS],
    connect_anchor: [Vector; HOSE_SEGMENTS],
    break_force: [f32; HOSE_SEGMENTS],
    ends: [End; 2],
    /// Segment in the inventory. The other segments are parked on it.
    stored: Option<usize>,
    coiled: bool,
}

/// Held hose input for one frame.
#[derive(Clone, Copy, Default)]
pub(super) struct HoseInput {
    pub held: Option<ItemId>,
    pub coil: bool,
    pub drain: bool,
}

pub(super) struct HoseState {
    hoses: Vec<Hose>,
    time: f32,
    min_height: f32,
    max_height: f32,
}

fn opening(openings: &[OpeningState], id: OpeningId) -> Option<&OpeningState> {
    openings.iter().find(|o| o.reference.id == id)
}
fn opening_mut(openings: &mut [OpeningState], id: OpeningId) -> Option<&mut OpeningState> {
    openings.iter_mut().find(|o| o.reference.id == id)
}

/// World pose of a cap and the body that carries it. An inactive pose owner has none.
fn cap_frame(
    cap: &OpeningState,
    items: &ItemStore,
    physics: &PhysicsWorld,
) -> Option<(Pose, Option<RigidBodyHandle>)> {
    let owner = items.get(cap.reference.tank)?;
    if owner.inactive_pose.is_some() {
        return None;
    }
    match owner
        .body
        .and_then(|h| physics.rapier().bodies.get(h).map(|b| (h, b)))
    {
        Some((handle, body)) => Some((
            *body.position() * to_rapier(owner.local_pose) * cap.local_pose,
            Some(handle),
        )),
        None => Some((to_rapier(owner.pose) * cap.local_pose, None)),
    }
}

fn snap(body: RigidBodyHandle, pose: Pose, physics: &mut PhysicsWorld) {
    physics.edit_body(body, |b| {
        b.set_position(pose, true);
        b.set_linvel(Vector::ZERO, true);
        b.set_angvel(Vector::ZERO, true);
    });
}

impl Hose {
    fn translation(&self, segment: usize, physics: &PhysicsWorld) -> Option<Vector> {
        physics
            .rapier()
            .bodies
            .get(self.bodies[segment])
            .map(|b| b.translation())
    }

    /// Source `Attach`: snap the end segment to the cap and joint it to the cap body.
    fn attach(
        &mut self,
        end: usize,
        cap: OpeningId,
        time: f32,
        openings: &[OpeningState],
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        let Some((pose, body)) = opening(openings, cap).and_then(|o| cap_frame(o, items, physics))
        else {
            return;
        };
        let segment = self.ends[end].segment;
        snap(self.bodies[segment], pose, physics);
        let link = match body {
            Some(body) => {
                let anchor = physics.rapier().bodies[body].position().inverse() * pose.translation;
                let joint = SphericalJointBuilder::new()
                    .local_anchor1(self.connect_anchor[segment])
                    .local_anchor2(anchor)
                    .contacts_enabled(false);
                Link::Joint(
                    physics.insert_impulse_joint(self.bodies[segment], body, joint, true),
                    body,
                )
            }
            None => Link::Pinned,
        };
        self.ends[end].cap = Some((cap, link));
        self.ends[end].attached_at = time;
    }

    fn detach(&mut self, end: usize, physics: &mut PhysicsWorld) {
        if let Some((_, Link::Joint(joint, _))) = self.ends[end].cap.take() {
            physics.remove_impulse_joint(joint, true);
        }
    }

    /// Source `Update` joint care: break force, lost body, and lost joint.
    fn care(
        &mut self,
        end: usize,
        time: f32,
        openings: &[OpeningState],
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        let Some((cap, link)) = self.ends[end].cap else {
            return;
        };
        let Some(state) = opening(openings, cap) else {
            self.detach(end, physics);
            return;
        };
        let Some((pose, body)) = cap_frame(state, items, physics) else {
            return;
        };
        let segment = self.ends[end].segment;
        match link {
            Link::Joint(joint, joined) => {
                let rapier = physics.rapier();
                // Rapier has no break force. The last step impulse gives the joint force.
                let force = rapier.impulse_joints.get(joint).map(|j| {
                    let [x, y, z, ..] = j.impulses;
                    Vector::new(x, y, z).length() / rapier.integration_parameters.dt
                });
                if body != Some(joined) {
                    self.detach(end, physics);
                    self.attach(end, cap, time, openings, items, physics);
                } else if force.is_none_or(|force| {
                    time - self.ends[end].attached_at >= BREAK_DELAY
                        && force > self.break_force[segment]
                }) {
                    self.detach(end, physics);
                }
            }
            Link::Pinned if body.is_some() => {
                self.detach(end, physics);
                self.attach(end, cap, time, openings, items, physics);
            }
            Link::Pinned => snap(self.bodies[segment], pose, physics),
        }
    }

    /// Source `RefreshOwnCap`: a free end borrows the tank of the attached end's cap.
    fn refresh_own_caps(&self, openings: &mut [OpeningState]) {
        let tank = |end: &End| {
            end.cap
                .and_then(|(cap, _)| opening(openings, cap))
                .map(OpeningState::fluid_owner)
        };
        let tanks = [tank(&self.ends[0]), tank(&self.ends[1])];
        let attached = self.ends.each_ref().map(|e| e.cap.is_some());
        for (index, end) in self.ends.iter().enumerate() {
            if let Some(own) = opening_mut(openings, end.own) {
                own.borrowed_tank = if attached[index] {
                    None
                } else {
                    tanks[1 - index]
                };
                if attached == [false; 2] {
                    own.valve = 0.0;
                }
            }
        }
    }

    /// Source `tubescript.Fill`: the higher end's tank pours into the lower end's tank.
    fn siphon(
        &self,
        dt: f32,
        fluid_scale: f32,
        heights: [f32; 2],
        openings: &mut [OpeningState],
        items: &mut ItemStore,
        physics: &PhysicsWorld,
    ) {
        let (Some((a, _)), Some((b, _))) = (self.ends[0].cap, self.ends[1].cap) else {
            return;
        };
        let (Some(a), Some(b)) = (opening(openings, a), opening(openings, b)) else {
            return;
        };
        let (Some(ya), Some(yb)) = (
            self.translation(self.ends[0].segment, physics),
            self.translation(self.ends[1].segment, physics),
        ) else {
            return;
        };
        // Flow already includes the source pour speed.
        let amount = siphon_rate(
            ya.y - yb.y,
            heights[0],
            heights[1],
            a.valve,
            a.reference.flow_liters_per_second,
            b.valve,
            b.reference.flow_liters_per_second,
            1.0,
            dt,
        ) * fluid_scale;
        let caps = [a, b];
        let (source, receiver) = siphon_direction(ya.y, yb.y);
        let (source, receiver_cap) = (caps[source].fluid_owner(), caps[receiver].reference.id);
        let receiver = caps[receiver].fluid_owner();
        let fluid = |id| items.get(id).and_then(|i| i.fluid.as_ref());
        if amount <= 0.0
            || source == receiver
            || !fluid(source).is_some_and(|f| f.total() > 0.0)
            || !fluid(receiver).is_some_and(|f| f.total() < f.capacity_liters)
        {
            return;
        }
        if let Some((source, receiver)) = items.get_pair_mut(source, receiver)
            && let (Some(source), Some(receiver)) = (source.fluid.as_mut(), receiver.fluid.as_mut())
        {
            transfer(source, receiver, amount);
        }
        if let Some(cap) = opening_mut(openings, receiver_cap) {
            cap.flow = Some((true, 1.));
            cap.fill_called = true;
        }
    }

    /// Source `Store` and `UnStore`: the stored segment carries the others.
    fn store_step(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) {
        let slot = |id| match items.get(id).map(|i| i.location) {
            Some(ItemLocation::Stored(slot)) => Some(slot),
            _ => None,
        };
        let segment = match self.stored {
            Some(segment) => segment,
            None => {
                let Some(segment) = self.items.iter().position(|id| slot(*id).is_some()) else {
                    return;
                };
                self.stored = Some(segment);
                self.detach(0, physics);
                self.detach(1, physics);
                segment
            }
        };
        let slot = slot(self.items[segment]);
        if slot.is_none() {
            self.stored = None;
        }
        let Some(pose) = physics
            .rapier()
            .bodies
            .get(self.bodies[segment])
            .map(|b| *b.position())
        else {
            return;
        };
        let location = slot.map_or(ItemLocation::World, ItemLocation::Stored);
        for index in (0..HOSE_SEGMENTS).filter(|i| *i != segment) {
            if items
                .get(self.items[index])
                .is_some_and(|i| i.location != location)
            {
                items.get_mut(self.items[index]).unwrap().location = location;
            }
            physics.edit_body(self.bodies[index], |b| {
                b.set_position(pose, false);
                b.set_enabled(slot.is_none());
            });
        }
    }

    /// Source `CoilingUp`: the other segments move toward the held one without gravity.
    fn coil(&mut self, held: usize, dt: f32, physics: &mut PhysicsWorld) {
        let Some(target) = self.translation(held, physics) else {
            return;
        };
        let t = (dt * COIL_RATE).min(1.0);
        for index in (0..HOSE_SEGMENTS).filter(|i| *i != held) {
            physics.edit_body(self.bodies[index], |b| {
                b.set_translation(b.translation().lerp(target, t), true);
                b.set_gravity_scale(0.0, true);
            });
        }
        self.coiled = true;
    }

    fn uncoil(&mut self, items: &ItemStore, physics: &mut PhysicsWorld) {
        if !std::mem::take(&mut self.coiled) {
            return;
        }
        for (id, body) in self.items.iter().zip(self.bodies) {
            if items
                .get(*id)
                .is_some_and(|i| i.location != ItemLocation::Held)
            {
                physics.try_edit_body(body, |b| b.set_gravity_scale(1.0, true));
            }
        }
    }

    /// Source `CanDrain`: one end attached and fluid in the tank the free end borrows.
    fn can_drain(&self, openings: &[OpeningState], items: &ItemStore) -> bool {
        let attached = self.ends.each_ref().map(|e| e.cap.is_some());
        if attached[0] == attached[1] {
            return false;
        }
        let free = &self.ends[usize::from(attached[0])];
        opening(openings, free.own)
            .and_then(|o| o.borrowed_tank)
            .and_then(|tank| items.get(tank))
            .and_then(|i| i.fluid.as_ref())
            .is_some_and(|f| f.total() > 0.0)
    }

    /// Source `JustDrain`: both own valves ease toward `target`.
    fn drain(&self, target: f32, dt: f32, openings: &mut [OpeningState]) {
        let attached = self.ends.iter().any(|e| e.cap.is_some());
        for end in &self.ends {
            if let Some(own) = opening_mut(openings, end.own) {
                own.valve = if attached {
                    ease_valve(own.valve, target, dt)
                } else {
                    0.0
                };
            }
        }
    }
}

impl HoseState {
    pub(super) fn new(min_height: f32, max_height: f32) -> Self {
        Self {
            hoses: Vec::new(),
            time: 0.0,
            min_height,
            max_height,
        }
    }

    fn find(&self, item: ItemId) -> Option<usize> {
        self.hoses.iter().position(|h| h.items.contains(&item))
    }

    /// Hose and end that attach to `cap` while `held` is carried, as `TryAttach`.
    fn attach_end(
        &self,
        held: ItemId,
        cap: &OpeningState,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> Option<(usize, usize)> {
        let index = self.find(held)?;
        let r = &cap.reference;
        if !cap_accepts(cap.valve, r.cant_tube, r.tube_connected) {
            return None;
        }
        let hose = &self.hoses[index];
        let (pose, _) = cap_frame(cap, items, physics)?;
        let [a, b] = hose.ends.each_ref().map(|e| {
            hose.translation(e.segment, physics)
                .map_or(f32::INFINITY, |p| p.distance_squared(pose.translation))
        });
        let [t1, t2] = &hose.ends;
        let on = |e: &End| e.cap.is_some_and(|(c, _)| c == r.id);
        pick_end(a, b, t1.cap.is_some(), t2.cap.is_some(), on(t1), on(t2)).map(|end| (index, end))
    }

    /// Attach the nearer free end of the hose of `held` to `cap`.
    fn attach(
        &mut self,
        held: ItemId,
        cap: OpeningId,
        openings: &mut [OpeningState],
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        let Some((index, end)) =
            opening(openings, cap).and_then(|o| self.attach_end(held, o, items, physics))
        else {
            return;
        };
        let hose = &mut self.hoses[index];
        hose.attach(end, cap, self.time, openings, items, physics);
        hose.refresh_own_caps(openings);
    }

    /// Runs before fluid pours: joint care, own cap tanks, siphon, and held input.
    pub(super) fn step(
        &mut self,
        input: HoseInput,
        dt: f32,
        fluid_scale: f32,
        openings: &mut [OpeningState],
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        self.time += dt;
        let heights = [self.min_height, self.max_height];
        for hose in &mut self.hoses {
            hose.store_step(items, physics);
            for end in 0..2 {
                hose.care(end, self.time, openings, items, physics);
            }
            hose.refresh_own_caps(openings);
            hose.siphon(dt, fluid_scale, heights, openings, items, physics);
            let held = input
                .held
                .and_then(|id| hose.items.iter().position(|i| *i == id));
            match held {
                Some(held) if input.coil => hose.coil(held, dt, physics),
                _ => hose.uncoil(items, physics),
            }
            if held.is_some() {
                let open = input.drain && hose.can_drain(openings, items);
                hose.drain(if open { 1.0 } else { 0.0 }, dt, openings);
            }
        }
    }
}

impl InteractionState {
    /// Bind one hose from its 6 segment items in `segments` order.
    pub fn register_hose(
        &mut self,
        prototype: &ItemPrototype,
        segments: [ItemId; HOSE_SEGMENTS],
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let hose = prototype
            .hose
            .as_ref()
            .ok_or_else(|| ReferenceError("Prototype has no hose".into()))?;
        if segments.iter().any(|id| self.hoses.find(*id).is_some()) {
            return Err(ReferenceError("Hose segment is already bound".into()));
        }
        let mut bodies = [RigidBodyHandle::invalid(); HOSE_SEGMENTS];
        for (body, id) in bodies.iter_mut().zip(segments) {
            *body = items
                .get(id)
                .and_then(|i| i.body)
                .ok_or_else(|| ReferenceError("Hose segment has no body".into()))?;
        }
        let segment = |index: usize| {
            hose.segments
                .iter()
                .position(|s| *s == hose.ends[index].body)
                .unwrap()
        };
        let own = [
            self.register_hose_end(segments[segment(0)], &hose.ends[0])?,
            self.register_hose_end(segments[segment(1)], &hose.ends[1])?,
        ];
        // Validation orders joint i from segment i to segment i + 1.
        for (index, joint) in hose.joints.iter().enumerate() {
            let joint = SphericalJointBuilder::new()
                .local_anchor1(Vector::from_array(joint.anchor))
                .local_anchor2(Vector::from_array(joint.connected_anchor))
                .contacts_enabled(false);
            physics.insert_impulse_joint(bodies[index], bodies[index + 1], joint, true);
        }
        physics.ignore_collisions_between(&bodies);
        let ends = [0, 1].map(|index| End {
            segment: segment(index),
            own: own[index],
            cap: None,
            attached_at: 0.0,
        });
        self.hoses.hoses.push(Hose {
            items: segments,
            bodies,
            connect_anchor: hose.connect_anchor.map(Vector::from_array),
            break_force: hose.break_force,
            ends,
            stored: None,
            coiled: false,
        });
        Ok(())
    }

    /// Forget the hose of a removed segment item and its end openings.
    pub(super) fn unregister_hose(&mut self, item: ItemId) {
        if let Some(index) = self.hoses.find(item) {
            let own = self.hoses.hoses.swap_remove(index).ends.map(|e| e.own);
            self.openings.retain(|o| !own.contains(&o.reference.id));
        }
    }

    /// Hose prompt bits and the cap to attach to while a hose segment is carried.
    pub(super) fn hose_actions(
        &self,
        held: ItemId,
        filling_target: Option<AimTarget>,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> (u8, Option<OpeningId>) {
        let Some(index) = self.hoses.find(held) else {
            return (0, None);
        };
        let mut prompts = HosePrompt::COIL;
        let cap = filling_target
            .and_then(|t| self.openings.iter().find(|o| o.reference.cap == t.item))
            .filter(|o| self.hoses.attach_end(held, o, items, physics).is_some())
            .map(|o| o.reference.id);
        if cap.is_some() {
            prompts |= HosePrompt::ATTACH;
        }
        if self.hoses.hoses[index].can_drain(&self.openings, items) {
            prompts |= HosePrompt::DRAIN;
        }
        (prompts, cap)
    }

    /// Drop the held segment and attach its hose to `cap`.
    pub(super) fn attach_held_hose(
        &mut self,
        cap: OpeningId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<ItemId> {
        let held = self.drop_held(items, physics)?;
        self.hoses
            .attach(held, cap, &mut self.openings, items, physics);
        Some(held)
    }

    pub(super) fn step_hoses(
        &mut self,
        input: HoseInput,
        dt: f32,
        fluid_scale: f32,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        self.hoses
            .step(input, dt, fluid_scale, &mut self.openings, items, physics);
    }
}

#[cfg(test)]
mod tests {
    use super::super::OpeningRules;
    use super::*;
    use crate::fluids::{FluidContainer, FluidEntry};
    use crate::reference::FluidPresentation;
    use crate::vehicle::VehiclePose;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    const TANK: ItemId = ItemId(10);

    fn item(
        id: ItemId,
        body: RigidBodyHandle,
        fluid: Option<FluidContainer>,
    ) -> super::super::items::ItemState {
        let pose = VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        };
        super::super::items::ItemState {
            id,
            body: Some(body),
            inactive_pose: None,
            location: ItemLocation::World,
            pose,
            local_pose: pose,
            generated_condition: None,
            temperature: None,
            fluid,
            food: None,
            hit_sound: None,
            removed: false,
            pending_attachment: false,
        }
    }

    fn cap(id: u32, tank: ItemId, valve: f32) -> OpeningState {
        OpeningState {
            flow: None,
            fill_called: false,
            empty_pour: false,
            reference: OpeningRules {
                id: OpeningId(id),
                tank,
                cap: tank,
                flow_liters_per_second: 1.,
                flow_radius: 0.02,
                tilt_degrees: 90.,
                angle_disabled: false,
                spill_disabled: false,
                tube_connected: false,
                cant_tube: false,
                can_toggle: false,
                can_tune: false,
                tune_rate: 0.,
                presentation: FluidPresentation {
                    has_name: false,
                    name: String::new(),
                    no_text: true,
                },
            },
            local_pose: Pose::IDENTITY,
            tilt_axis: -Vector::Z,
            valve,
            particles: None,
            contact: None,
            borrowed_tank: None,
        }
    }

    /// A 6 segment hose along X, a tank body at the T1 end, and its open cap (id 3).
    /// Own caps are 1 and 2.
    fn fixture() -> (HoseState, Vec<OpeningState>, ItemStore, PhysicsWorld) {
        let mut physics = PhysicsWorld::empty(0, 0);
        let mut body = |x: f32| {
            physics
                .insert(
                    RigidBodyBuilder::dynamic().translation(Vector::new(x, 1., 0.)),
                    ColliderBuilder::capsule_x(0.075, 0.02),
                )
                .0
        };
        let bodies: [RigidBodyHandle; HOSE_SEGMENTS] =
            std::array::from_fn(|i| body(0.15 * i as f32));
        let tank = body(-0.3);
        let segments = std::array::from_fn(|i| ItemId(i as u32 + 1));
        let fluid = FluidContainer::new(
            20.,
            &[FluidEntry {
                fluid_type: 0,
                amount: 5.,
            }],
        );
        let mut states: Vec<_> = segments
            .iter()
            .zip(bodies)
            .map(|(id, body)| item(*id, body, None))
            .collect();
        states.push(item(TANK, tank, Some(fluid)));
        let end = |segment, own| End {
            segment,
            own: OpeningId(own),
            cap: None,
            attached_at: 0.,
        };
        let mut hoses = HoseState::new(0.05, 0.15);
        hoses.hoses.push(Hose {
            items: segments,
            bodies,
            connect_anchor: [Vector::ZERO; HOSE_SEGMENTS],
            break_force: [1e9; HOSE_SEGMENTS],
            ends: [end(0, 1), end(5, 2)],
            stored: None,
            coiled: false,
        });
        let openings = vec![
            cap(1, segments[0], 0.),
            cap(2, segments[5], 0.),
            cap(3, TANK, 1.),
        ];
        (hoses, openings, ItemStore::from_states(states), physics)
    }

    fn step(world: &mut (HoseState, Vec<OpeningState>, ItemStore, PhysicsWorld)) {
        let (hoses, openings, items, physics) = world;
        hoses.step(HoseInput::default(), 0.02, 1., openings, items, physics);
    }

    fn attach_t1(world: &mut (HoseState, Vec<OpeningState>, ItemStore, PhysicsWorld)) {
        let (hoses, openings, items, physics) = world;
        hoses.attach(ItemId(3), OpeningId(3), openings, items, physics);
        assert!(hoses.hoses[0].ends[0].cap.is_some());
    }

    #[test]
    fn one_attached_end_lends_its_tank_to_the_free_end() {
        let mut world = fixture();
        attach_t1(&mut world);
        step(&mut world);
        let openings = &world.1;
        assert_eq!(opening(openings, OpeningId(1)).unwrap().borrowed_tank, None);
        assert_eq!(
            opening(openings, OpeningId(2)).unwrap().borrowed_tank,
            Some(TANK)
        );
    }

    #[test]
    fn removed_cap_joint_detaches_the_end() {
        let mut world = fixture();
        attach_t1(&mut world);
        let Some((_, Link::Joint(joint, _))) = world.0.hoses[0].ends[0].cap else {
            panic!("cap joint");
        };
        world.3.remove_impulse_joint(joint, true);
        step(&mut world);
        assert!(world.0.hoses[0].ends[0].cap.is_none());
        assert_eq!(opening(&world.1, OpeningId(2)).unwrap().borrowed_tank, None);
    }

    #[test]
    fn storing_a_segment_detaches_both_ends_and_parks_the_others() {
        let mut world = fixture();
        attach_t1(&mut world);
        world.2.get_mut(ItemId(3)).unwrap().location = ItemLocation::Stored(1);
        step(&mut world);
        let hose = &world.0.hoses[0];
        assert!(hose.ends.iter().all(|e| e.cap.is_none()));
        assert_eq!(world.3.rapier().impulse_joints.len(), 0);
        assert_eq!(opening(&world.1, OpeningId(2)).unwrap().borrowed_tank, None);
        assert_eq!(
            world.2.get(ItemId(1)).unwrap().location,
            ItemLocation::Stored(1)
        );
        assert!(!world.3.rapier().bodies[hose.bodies[0]].is_enabled());
        world.2.get_mut(ItemId(3)).unwrap().location = ItemLocation::Held;
        step(&mut world);
        assert_eq!(
            world.2.get(ItemId(1)).unwrap().location,
            ItemLocation::World
        );
        assert!(world.3.rapier().bodies[world.0.hoses[0].bodies[0]].is_enabled());
    }

    fn rate(h: f32) -> f32 {
        siphon_rate(h, 0.05, 0.15, 1.0, 2.0, 0.5, 3.0, 4.0, 0.1)
    }

    #[test]
    fn siphon_rate_scales_with_height() {
        // min(1*2, 0.5*3) * 4 * 0.1 = 0.6
        assert_eq!(rate(0.04), 0.0);
        assert!((rate(0.10) - 0.3).abs() < 1e-6);
        assert!((rate(0.20) - 0.6).abs() < 1e-6);
        assert!((rate(-0.20) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn siphon_direction_higher_end_is_source() {
        assert_eq!(siphon_direction(1.0, 0.5), (0, 1));
        assert_eq!(siphon_direction(0.5, 1.0), (1, 0));
    }

    #[test]
    fn pick_end_rules() {
        assert_eq!(pick_end(1.0, 2.0, false, false, false, false), Some(0));
        assert_eq!(pick_end(2.0, 1.0, false, false, false, false), Some(1));
        assert_eq!(pick_end(1.0, 2.0, true, false, false, false), Some(1));
        assert_eq!(pick_end(2.0, 1.0, false, true, false, false), Some(0));
        assert_eq!(pick_end(1.0, 2.0, true, true, false, false), None);
        assert_eq!(pick_end(1.0, 2.0, false, true, false, true), None);
        assert_eq!(pick_end(1.0, 2.0, true, false, true, false), None);
    }

    #[test]
    fn cap_accepts_rules() {
        assert!(cap_accepts(1.0, false, false));
        assert!(!cap_accepts(0.0, false, false));
        assert!(!cap_accepts(1.0, true, false));
        assert!(!cap_accepts(1.0, false, true));
    }

    #[test]
    fn ease_valve_steps_and_clamps() {
        assert!((ease_valve(0.0, 1.0, 0.1) - 0.5).abs() < 1e-6);
        assert_eq!(ease_valve(0.0, 1.0, 1.0), 1.0);
    }
}
