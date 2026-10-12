//! Source controls use cached transform bindings for physics and rendering.
use crate::reference::*;
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};
use rapier3d::glamx::EulerRot;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;
use std::collections::{HashMap, HashSet};

struct Frame {
    node: NodeId,
    parent: Option<usize>,
    local: Pose,
    scale: Vector,
    world: Pose,
    world_scale: Vector,
    handle: Option<RigidBodyHandle>,
    driven: bool,
    inactive: bool,
    body: Option<BodyId>,
    root_body: Option<BodyId>,
    relative: Pose,
    external: Option<RigidBodyHandle>,
    external_pose: Option<Pose>,
}
struct Control {
    reference: MechanismReference,
    degrees: [f32; 2],
    slide: f32,
    enabled: bool,
    turn_state: u8,
}
pub struct MechanismState {
    controls: Vec<Control>,
    items: HashMap<ItemId, usize>,
    frames: Vec<Frame>,
    nodes: HashMap<NodeId, usize>,
    bodies: HashMap<BodyId, usize>,
    scroll_sensitivity: f32,
    control_action: Option<ControlAction>,
    item_nodes: HashMap<ItemId, NodeId>,
    order: Vec<usize>,
}
impl MechanismState {
    pub(crate) fn restore_save(
        &mut self,
        ids: &crate::save::IdentityMap,
        saved: &[crate::save::MechanismSnapshot],
    ) {
        for record in saved {
            let control = self
                .controls
                .iter_mut()
                .find(|c| c.reference.id == ids.mechanisms[&record.source])
                .unwrap();
            control.degrees = record.degrees;
            control.slide = record.slide;
            control.enabled = record.enabled;
            control.turn_state = record.turn_state;
        }
        for index in 0..self.controls.len() {
            self.apply(index);
        }
        self.control_action = None;
    }
    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
    ) -> Result<Vec<crate::save::MechanismSnapshot>, crate::save::SaveError> {
        let mut records = self
            .controls
            .iter()
            .map(|c| {
                Ok(crate::save::MechanismSnapshot {
                    source: crate::save::identity::source_key(&ids.mechanisms, c.reference.id)?,
                    degrees: c.degrees,
                    slide: c.slide,
                    enabled: c.enabled,
                    turn_state: c.turn_state,
                })
            })
            .collect::<Result<Vec<_>, crate::save::SaveError>>()?;
        records.sort_by(|a, b| a.source.cmp(&b.source));
        Ok(records)
    }
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut needed = HashSet::new();
        needed.extend(reference.items.iter().map(|i| i.node));
        needed.extend(reference.anchors.iter().map(|a| a.node));
        let driven: HashSet<_> = reference
            .mechanisms
            .iter()
            .flat_map(|m| m.driven_nodes.iter().copied())
            .collect();
        for m in &reference.mechanisms {
            needed.extend(m.driven_nodes.iter().copied());
            needed.extend([m.drag.node, m.drag.node2]);
            if let Some(slide) = &m.slide {
                for path in &slide.paths {
                    needed.extend(path.points.iter().map(|p| p.node));
                }
                needed.extend(slide.rotations.iter().map(|r| r.reference));
            }
        }
        // Resolve ancestors once. The frame loop has no hierarchy queries.
        let seeds: Vec<_> = needed.iter().copied().collect();
        for seed in seeds {
            let mut node = Some(seed);
            for _ in 0..=reference.nodes.len() {
                let Some(id) = node else {
                    break;
                };
                needed.insert(id);
                node = reference
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .ok_or_else(|| ReferenceError("Missing mechanism node".into()))?
                    .parent;
            }
            if node.is_some() {
                return Err(ReferenceError("Cyclic mechanism hierarchy".into()));
            }
        }
        let mut frames: Vec<Frame> = Vec::with_capacity(needed.len());
        let mut nodes = HashMap::new();
        let mut bodies = HashMap::new();
        while frames.len() < needed.len() {
            let before = frames.len();
            for node in &reference.nodes {
                if !needed.contains(&node.id)
                    || nodes.contains_key(&node.id)
                    || node.parent.is_some_and(|p| !nodes.contains_key(&p))
                {
                    continue;
                }
                let parent = node.parent.map(|p| nodes[&p]);
                let local = to_rapier(VehiclePose {
                    position: node.transform.translation,
                    rotation: node.transform.rotation,
                });
                let body = reference
                    .bodies
                    .iter()
                    .find(|b| b.node == node.id)
                    .map(|b| b.id);
                if let Some(id) = body {
                    bodies.insert(id, frames.len());
                }
                nodes.insert(node.id, frames.len());
                frames.push(Frame {
                    node: node.id,
                    parent,
                    local,
                    scale: Vector::from_array(node.transform.scale),
                    world: local,
                    world_scale: Vector::ONE,
                    handle: None,
                    driven: driven.contains(&node.id),
                    inactive: false,
                    body,
                    root_body: None,
                    relative: Pose::IDENTITY,
                    external: None,
                    external_pose: None,
                });
            }
            if frames.len() == before {
                return Err(ReferenceError("Unresolved mechanism hierarchy".into()));
            }
        }
        let items = reference
            .mechanisms
            .iter()
            .enumerate()
            .map(|(i, m)| (m.item, i))
            .collect();
        let controls = reference
            .mechanisms
            .iter()
            .map(|m| Control {
                reference: m.clone(),
                degrees: m.rotation.as_ref().map_or([0.0; 2], |r| r.initial_degrees),
                slide: m.slide.as_ref().map_or(0.0, |s| s.initial_value),
                enabled: m.enabled,
                turn_state: m.turn.as_ref().map_or(0, |t| t.initial_state),
            })
            .collect();
        let order = (0..frames.len()).collect();
        let mut state = Self {
            order,
            controls,
            items,
            frames,
            nodes,
            bodies,
            scroll_sensitivity: reference.player.scroll_sensitivity,
            control_action: None,
            item_nodes: reference.items.iter().map(|i| (i.id, i.node)).collect(),
        };
        state.propagate();
        for i in 0..state.controls.len() {
            state.apply(i);
        }
        Ok(state)
    }
    pub fn take_control_action(&mut self) -> Option<ControlAction> {
        self.control_action.take()
    }
    pub fn primary_switch(&self, item: ItemId) -> bool {
        self.items.get(&item).is_some_and(|i| {
            self.controls[*i].enabled
                && self.controls[*i]
                    .reference
                    .turn
                    .as_ref()
                    .is_some_and(|t| matches!(t.action, TurnAction::ToggleHomeLight { .. }))
        })
    }
    pub fn turn_state(&self, item: ItemId) -> Option<u8> {
        let c = &self.controls[*self.items.get(&item)?];
        c.reference.turn.as_ref().map(|_| c.turn_state)
    }
    pub fn contains(&self, item: ItemId) -> bool {
        self.items
            .get(&item)
            .is_some_and(|i| self.controls[*i].enabled)
    }
    pub fn rotation_degrees(&self, item: ItemId) -> Option<[f32; 2]> {
        let c = &self.controls[*self.items.get(&item)?];
        c.reference.rotation.as_ref().map(|_| c.degrees)
    }
    pub fn scroll_eligible(&self, item: ItemId) -> bool {
        self.items.get(&item).is_some_and(|i| {
            self.controls[*i].enabled
                && self.controls[*i]
                    .reference
                    .slide
                    .as_ref()
                    .is_some_and(|s| s.scroll_without_use)
        })
    }
    pub fn value(&self, item: ItemId, include_child: bool) -> Option<f32> {
        let c = &self.controls[*self.items.get(&item)?];
        let value = if c.reference.slide.is_some() {
            c.slide
        } else if let Some(r) = &c.reference.rotation {
            let mut value: f32 = 0.0;
            for i in 0..2 {
                let [a, b] = r.limits_degrees[i];
                value = value.max(if r.inverted[i] {
                    inverse_lerp(b, a, c.degrees[i])
                } else {
                    inverse_lerp(a, b, c.degrees[i])
                });
            }
            if c.reference.invert_value {
                1.0 - value
            } else {
                value
            }
        } else {
            0.0
        };
        let child = if include_child {
            c.reference
                .child
                .and_then(|id| self.value(id, true))
                .unwrap_or(0.0)
        } else {
            0.0
        };
        Some(
            value
                + if c.reference.invert_child {
                    -child
                } else {
                    child
                },
        )
    }
    /// Current world pose, in rebased runtime coordinates.
    pub fn node_pose(&self, node: NodeId) -> Option<VehiclePose> {
        self.nodes
            .get(&node)
            .map(|i| from_rapier(&self.frames[*i].world))
    }
    pub fn body_pose(&self, body: BodyId) -> Option<Pose> {
        self.bodies
            .get(&body)
            .filter(|i| self.frames[**i].driven)
            .map(|i| self.frames[*i].world)
    }
    /// Body-local meters, computed without a world-space round trip.
    pub fn body_local_pose(&self, body: BodyId, root: BodyId) -> Option<Pose> {
        let f = &self.frames[*self.bodies.get(&body)?];
        (f.driven && f.root_body == Some(root)).then_some(f.relative)
    }
    /// Cached node-to-body links for part attachment integration.
    pub fn body_node(&self, body: BodyId) -> Option<NodeId> {
        self.bodies.get(&body).map(|i| self.frames[*i].node)
    }
    pub fn bind_body(&mut self, id: BodyId, handle: RigidBodyHandle) {
        if let Some(i) = self.bodies.get(&id) {
            self.frames[*i].handle = Some(handle);
        }
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        let shift = Vector::from_array(shift);
        for f in &mut self.frames {
            if f.inactive {
                continue;
            }
            if f.parent.is_none() {
                f.local.translation -= shift;
            }
            f.world.translation -= shift;
            if let Some(pose) = &mut f.external_pose {
                pose.translation -= shift;
            }
        }
    }
    pub fn cancel_input(&mut self) {
        self.control_action = None;
    }
    pub fn set_part_root(&mut self, node: NodeId, body: Option<RigidBodyHandle>) {
        let f = &mut self.frames[self.nodes[&node]];
        f.external = body;
        if body.is_none() {
            f.external_pose = None;
        }
    }
    pub fn set_item_pose(&mut self, item: ItemId, pose: VehiclePose) {
        let f = &mut self.frames[self.nodes[&self.item_nodes[&item]]];
        f.external_pose = Some(to_rapier(pose));
    }
    pub fn mount_part(&mut self, node: NodeId, mount: NodeId, rotation: [f32; 4]) {
        let parent = self.nodes[&mount];
        let f = &mut self.frames[self.nodes[&node]];
        f.external = None;
        f.external_pose = None;
        f.parent = Some(parent);
        f.local = Pose::from_rotation(Rotation::from_xyzw(
            rotation[0],
            rotation[1],
            rotation[2],
            rotation[3],
        ));
        let mut order = Vec::with_capacity(self.frames.len());
        while order.len() < self.frames.len() {
            let before = order.len();
            for i in 0..self.frames.len() {
                if !order.contains(&i) && self.frames[i].parent.is_none_or(|p| order.contains(&p)) {
                    order.push(i);
                }
            }
            assert!(order.len() > before, "Cyclic part mount");
        }
        self.order = order;
        self.propagate();
    }
    pub fn node_local_pose(&self, node: NodeId, root: BodyId) -> Option<Pose> {
        let f = &self.frames[*self.nodes.get(&node)?];
        (f.root_body == Some(root)).then_some(f.relative)
    }
    pub fn uncraft(&mut self, usable: ItemId, node: NodeId) -> Option<Rotation> {
        let index = *self.items.get(&usable)?;
        let control_node = self.nodes[&self.item_nodes[&self.controls[index].reference.item]];
        let rotation = self.frames[control_node].world.rotation;
        let root = self.frames[self.nodes[&node]].world.rotation;
        let c = &mut self.controls[index];
        c.degrees = [0.; 2];
        c.slide = 0.;
        self.apply(index);
        Some(rotation * self.frames[control_node].world.rotation.inverse() * root)
    }
    pub fn set_mounted(&mut self, parent: ItemId, mounted: bool) {
        for c in &mut self.controls {
            if c.reference.parent_item != Some(parent) {
                continue;
            }
            if c.reference.disable_on_craft {
                c.enabled = !mounted;
            }
            if c.reference.disable_on_uncraft {
                c.enabled = mounted;
            }
        }
    }
    pub fn step(&mut self, item: ItemId, input: FrameInput, view: VehiclePose, dt: f32) {
        if !dt.is_finite()
            || dt <= 0.0
            || !input.mouse.iter().all(|v| v.is_finite())
            || !input.wheel.is_finite()
        {
            return;
        }
        let Some(&index) = self.items.get(&item) else {
            return;
        };
        if !self.controls[index].enabled {
            return;
        }
        if input.pressed & keys::USE != 0
            || (input.pressed & keys::FILL != 0 && self.primary_switch(item))
        {
            let c = &mut self.controls[index];
            if let Some(turn) = &c.reference.turn {
                c.turn_state = if c.turn_state >= turn.max_state {
                    0
                } else {
                    c.turn_state + 1
                };
                self.control_action = Some(match &turn.action {
                    TurnAction::ToggleHandbrake => ControlAction::ToggleHandbrake,
                    TurnAction::CycleHeadlights => ControlAction::CycleHeadlights,
                    TurnAction::InteriorLight => ControlAction::InteriorLight(c.turn_state),
                    TurnAction::ToggleHomeLight { source } => {
                        ControlAction::ToggleHomeLight(source.clone())
                    }
                });
                return;
            }
        }
        let held = input.held & keys::USE != 0;
        if !held && !(input.wheel != 0.0 && self.scroll_eligible(item)) {
            return;
        }
        let c = &self.controls[index];
        let drag = &c.reference.drag;
        // Browser movementY points down. Source LOOKY points up.
        let [x, y] = if held {
            [input.mouse[0], -input.mouse[1]]
        } else {
            [0.0; 2]
        };
        let view = to_rapier(view).rotation;
        let (x, y) = match drag.mode {
            DragMode::Direct => (x, y),
            DragMode::Relative => {
                let direction = self.frames[self.nodes[&drag.node]].world.rotation
                    * Vector::from_array(drag.axis);
                let value = (view * (Vector::X * x - Vector::Z * y)).dot(direction);
                (value, value)
            }
            DragMode::RelativeTwo => {
                let basis = self.frames[self.nodes[&drag.node2]].world.rotation;
                let dx = basis * Vector::from_array(drag.axes2[0]);
                let dy = basis * Vector::from_array(drag.axes2[1]);
                let px = (view * Vector::X).dot(dx);
                let up = (view * Vector::Y).dot(dy);
                let forward = (view * -Vector::Z).dot(dy);
                (
                    if px.abs() > 0.2 { px * x } else { x },
                    if up.abs() > forward.abs() {
                        up * y
                    } else {
                        forward * y
                    },
                )
            }
        };
        let c = &mut self.controls[index];
        if held && let Some(r) = &c.reference.rotation {
            for (i, delta) in [x, y].into_iter().enumerate() {
                let sign = if r.inverted[i] == (i == 0) { 1.0 } else { -1.0 };
                c.degrees[i] = (c.degrees[i] + delta * r.sensitivity * sign)
                    .clamp(r.limits_degrees[i][0], r.limits_degrees[i][1]);
            }
        }
        if let Some(s) = &c.reference.slide {
            let values = [x, y, input.wheel * self.scroll_sensitivity];
            let mut sum = 0.0;
            for i in 0..6 {
                if s.axes[i] {
                    let axis = i % 3;
                    sum += values[axis]
                        * s.speed
                        * s.sensitivity
                        * s.multiplier[axis]
                        * if i < 3 { 1.0 } else { -1.0 };
                    c.slide += sum;
                }
            }
            c.slide = c.slide.clamp(0.0, 1.0);
        }
        self.apply(index);
    }
    fn propagate(&mut self) {
        for &i in &self.order {
            if self.frames[i].inactive {
                continue;
            }
            let (world, scale) = if let Some(pose) = self.frames[i].external_pose {
                (pose, self.frames[i].world_scale)
            } else if let Some(parent) = self.frames[i].parent {
                let p = &self.frames[parent];
                let f = &self.frames[i];
                (
                    Pose::from_parts(
                        p.world.translation
                            + p.world.rotation * (p.world_scale * f.local.translation),
                        p.world.rotation * f.local.rotation,
                    ),
                    p.world_scale * f.scale,
                )
            } else {
                (self.frames[i].local, self.frames[i].scale)
            };
            let (root_body, relative) = if self.frames[i].body.is_some() && !self.frames[i].driven {
                (self.frames[i].body, Pose::IDENTITY)
            } else if let Some(parent) = self.frames[i].parent {
                let p = &self.frames[parent];
                let f = &self.frames[i];
                (
                    p.root_body,
                    p.relative
                        * Pose::from_parts(p.world_scale * f.local.translation, f.local.rotation),
                )
            } else {
                (None, self.frames[i].local)
            };
            self.frames[i].root_body = root_body;
            self.frames[i].relative = relative;
            self.frames[i].world = world;
            self.frames[i].world_scale = scale;
        }
    }
    fn apply(&mut self, index: usize) {
        let c = &self.controls[index];
        if let Some(r) = &c.reference.rotation {
            for target in &r.targets {
                let f = &mut self.frames[self.nodes[&target.node]];
                // C * q * C converts to Unity; its Euler order is Y-X-Z.
                let q = f.local.rotation;
                let source = Rotation::from_xyzw(-q.x, -q.y, q.z, q.w);
                let (y, x, z) = source.to_euler(EulerRot::YXZ);
                let mut euler = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
                for (axis, value) in euler.iter_mut().enumerate() {
                    let [mx, my] = [target.mapping[0][axis], target.mapping[1][axis]];
                    if mx {
                        *value = c.degrees[0] + if my && r.sum_axes { c.degrees[1] } else { 0.0 };
                    } else if my {
                        *value = c.degrees[1];
                    }
                }
                f.local.rotation = Rotation::from_rotation_y(-euler[1].to_radians())
                    * Rotation::from_rotation_x(-euler[0].to_radians())
                    * Rotation::from_rotation_z(euler[2].to_radians());
            }
        }
        self.propagate();
        if self.controls[index].reference.slide.is_none() {
            return;
        }
        let count = self.controls[index]
            .reference
            .slide
            .as_ref()
            .unwrap()
            .paths
            .len();
        for i in 0..count {
            let c = &self.controls[index];
            let path = &c.reference.slide.as_ref().unwrap().paths[i];
            let end = path
                .points
                .iter()
                .position(|p| p.value >= c.slide)
                .unwrap_or(path.points.len() - 1)
                .max(1);
            let start = end - 1;
            let t = inverse_lerp(path.points[start].value, path.points[end].value, c.slide);
            let a = self.frames[self.nodes[&path.points[start].node]].world;
            let b = self.frames[self.nodes[&path.points[end].node]].world;
            let node = self.nodes[&path.node];
            let rotation = if path.rotate {
                a.rotation.lerp(b.rotation, t).normalize()
            } else {
                self.frames[node].world.rotation
            };
            self.set_world(
                node,
                Pose::from_parts(a.translation.lerp(b.translation, t), rotation),
            );
        }
        let count = self.controls[index]
            .reference
            .slide
            .as_ref()
            .unwrap()
            .rotations
            .len();
        for i in 0..count {
            let c = &self.controls[index];
            let r = &c.reference.slide.as_ref().unwrap().rotations[i];
            let node = self.nodes[&r.node];
            let mut pose = self.frames[node].world;
            pose.rotation = self.frames[self.nodes[&r.reference]].world.rotation
                * Rotation::from_axis_angle(
                    Vector::from_array(r.axis).normalize(),
                    (r.angle_degrees * c.slide).to_radians(),
                );
            self.set_world(node, pose);
        }
        self.propagate();
    }
    fn set_world(&mut self, node: usize, pose: Pose) {
        let local = if let Some(parent) = self.frames[node].parent {
            let p = &self.frames[parent];
            let relative = p.world.inverse() * pose;
            Pose::from_parts(relative.translation / p.world_scale, relative.rotation)
        } else {
            pose
        };
        self.frames[node].local = local;
        self.frames[node].world = pose;
    }
    pub fn sync(&mut self, physics: &mut PhysicsWorld, commit: bool) {
        for &i in &self.order {
            let f = &self.frames[i];
            let inactive = f
                .external
                .or(f.handle)
                .map(|h| !physics.body_active(h))
                .unwrap_or_else(|| f.parent.is_some_and(|p| self.frames[p].inactive));
            self.frames[i].inactive = inactive;
        }
        for f in &mut self.frames {
            if f.inactive {
                continue;
            }
            if let Some(handle) = f.external {
                f.external_pose = Some(*physics.rapier().bodies[handle].position());
            }
        }
        // External body roots supply their current pose. Mechanism bodies use this state.
        for i in 0..self.frames.len() {
            if self.frames[i].inactive || self.frames[i].driven || self.frames[i].external.is_some()
            {
                continue;
            }
            if let Some(handle) = self.frames[i].handle
                && let Some(body) = physics.rapier().bodies.get(handle)
            {
                self.set_world(i, *body.position());
            }
        }
        self.propagate();
        for f in &self.frames {
            if f.inactive || !f.driven {
                continue;
            }
            if let Some(handle) = f.handle
                && let Some(body) = physics.rapier_mut().bodies.get_mut(handle)
            {
                if commit {
                    body.set_position(f.world, false);
                }
                body.set_next_kinematic_position(f.world);
            }
        }
    }
}
fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlAction {
    ToggleHandbrake,
    CycleHeadlights,
    InteriorLight(u8),
    ToggleHomeLight(SourceId),
}
