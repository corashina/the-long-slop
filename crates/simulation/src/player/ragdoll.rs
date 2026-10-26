//! Source player ragdoll: `fpscontroller.RagdolReset`, `SpawnRagdol`, `DeathLook`, and `ragdolPosScript`.
//! The ragdoll has no Rapier bodies until `spawn`.
use super::body::ik::Frame;
use crate::reference::{
    NodeId, NodeRecord, RagdollJoint, RagdollReference, ReferenceError, ReferenceManifest,
};
use crate::vehicle::{PhysicsWorld, collider_builder};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{
    Collider, GenericJoint, GenericJointBuilder, Group, InteractionGroups, InteractionTestMode,
    JointAxesMask, JointAxis, RigidBodyBuilder, RigidBodyHandle,
};
use std::collections::HashMap;

/// `RagdolReset` places the prefab 10 m below the player, as a player child.
const RESET_DEPTH: f32 = 10.;

struct BodyTemplate {
    node: usize,
    mass_kg: f32,
    linear_damping: f32,
    angular_damping: f32,
}

pub struct Ragdoll {
    /// Ragdoll asset nodes under the root, parents first. Index 0 is the root.
    nodes: Vec<NodeId>,
    index: HashMap<NodeId, usize>,
    parent: Vec<Option<usize>>,
    /// Prefab parent-local frames.
    rest: Vec<Frame>,
    /// Nearest body at or above each node.
    owner: Vec<Option<usize>>,
    /// `ragdolT` pairs: ragdoll node index, live player node.
    pairs: Vec<(usize, NodeId)>,
    /// `ragdolPosScript.head`.
    head: usize,
    bodies: Vec<BodyTemplate>,
    /// Body index and the built collider.
    colliders: Vec<(usize, Collider)>,
    /// Own body index, connected body index, joint.
    joints: Vec<(usize, usize, GenericJoint)>,
    /// Spawned bodies, in record order. Empty while alive.
    handles: Vec<RigidBodyHandle>,
    /// Spawned node frames: relative to the owner body, or world frames for nodes without one.
    frames: Vec<Frame>,
    local: Vec<Frame>,
    world: Vec<Frame>,
}

fn frame(t: &crate::reference::Transform) -> Frame {
    Frame {
        position: Vector::from_array(t.translation),
        rotation: Rotation::from_array(t.rotation),
        scale: Vector::from_array(t.scale),
    }
}

fn unscaled(f: &Frame) -> Frame {
    Frame {
        scale: Vector::ONE,
        ..*f
    }
}

/// Twist about X, swing1 about Y, swing2 about Z. Body 1 is the connected body, so the
/// measured angle is the own body relative to the connected body, as in Unity.
fn joint(r: &RagdollReference, j: &RagdollJoint) -> GenericJoint {
    let x = Vector::from_array(j.axis);
    let swing = Vector::from_array(j.swing_axis);
    let y = (swing - x * x.dot(swing)).normalize();
    let basis = Rotation::from_mat3(&rapier3d::glamx::Mat3::from_cols(x, y, x.cross(y)));
    let rest = |id: crate::reference::BodyId| {
        Rotation::from_array(r.bodies[id.0 as usize - 1].pose.rotation)
    };
    let connected = rest(j.connected_body).inverse() * rest(j.body) * basis;
    GenericJointBuilder::new(JointAxesMask::LOCKED_SPHERICAL_AXES)
        .local_frame1(Pose::from_parts(
            Vector::from_array(j.connected_anchor),
            connected,
        ))
        .local_frame2(Pose::from_parts(Vector::from_array(j.anchor), basis))
        .contacts_enabled(j.collide_connected)
        .limits(
            JointAxis::AngX,
            [
                j.low_twist.limit.to_radians(),
                j.high_twist.limit.to_radians(),
            ],
        )
        .limits(
            JointAxis::AngY,
            [-j.swing1.limit, j.swing1.limit].map(f32::to_radians),
        )
        .limits(
            JointAxis::AngZ,
            [-j.swing2.limit, j.swing2.limit].map(f32::to_radians),
        )
        .build()
}

impl Ragdoll {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        Self::from_parts(&r.nodes, &r.clothing.ragdoll)
    }
    pub fn from_parts(nodes: &[NodeRecord], r: &RagdollReference) -> Result<Self, ReferenceError> {
        let records: HashMap<_, _> = nodes
            .iter()
            .filter(|n| n.asset == r.asset)
            .map(|n| (n.id, n))
            .collect();
        let depth = |mut id: NodeId| {
            let mut depth = 0;
            while id != r.root {
                id = records.get(&id)?.parent?;
                depth += 1;
            }
            Some(depth)
        };
        let mut ids: Vec<(usize, NodeId)> = records
            .keys()
            .filter_map(|id| Some((depth(*id)?, *id)))
            .collect();
        ids.sort_by_key(|(depth, id)| (*depth, id.0));
        let ids: Vec<NodeId> = ids.into_iter().map(|(_, id)| id).collect();
        if ids.first() != Some(&r.root) {
            return Err(ReferenceError("Missing ragdoll root".into()));
        }
        let index: HashMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let find = |id: NodeId| {
            index
                .get(&id)
                .copied()
                .ok_or_else(|| ReferenceError("Ragdoll node outside the prefab".into()))
        };
        let parent: Vec<Option<usize>> = ids
            .iter()
            .map(|id| records[id].parent.and_then(|p| index.get(&p).copied()))
            .collect();
        let rest: Vec<Frame> = ids.iter().map(|id| frame(&records[id].transform)).collect();
        let bodies = r
            .bodies
            .iter()
            .map(|b| {
                Ok(BodyTemplate {
                    node: find(b.node)?,
                    mass_kg: b.mass_kg,
                    linear_damping: b.linear_damping,
                    angular_damping: b.angular_damping,
                })
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let mut owner: Vec<Option<usize>> = vec![None; ids.len()];
        for i in 0..ids.len() {
            let inherited = parent[i].and_then(|p| owner[p]);
            owner[i] = bodies.iter().position(|b| b.node == i).or(inherited);
        }
        let body = |id: crate::reference::BodyId| {
            (id.0 as usize)
                .checked_sub(1)
                .filter(|i| *i < bodies.len())
                .ok_or_else(|| ReferenceError("Missing ragdoll body".into()))
        };
        let colliders = r
            .colliders
            .iter()
            .map(|c| {
                let collider = collider_builder(&c.geometry)?
                    .friction(c.friction)
                    .restitution(c.restitution)
                    .sensor(c.sensor)
                    .solver_groups(crate::vehicle::solver_groups(true))
                    .density(0.)
                    .collision_groups(InteractionGroups::new(
                        Group::from_bits_truncate(c.membership),
                        Group::from_bits_truncate(c.filter),
                        InteractionTestMode::And,
                    ))
                    .user_data(
                        crate::vehicle::step_surface_bits(c.step_surface)
                            | crate::vehicle::tag_bits(c.tag),
                    )
                    .build();
                Ok((body(c.body)?, collider))
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let joints = r
            .joints
            .iter()
            .map(|j| Ok((body(j.body)?, body(j.connected_body)?, joint(r, j))))
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let pairs = r
            .pose
            .pairs
            .iter()
            .map(|p| Ok((find(p.ragdoll)?, p.live)))
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let head = find(r.pose.head)?;
        if owner[head].is_none() {
            return Err(ReferenceError("Ragdoll head has no body".into()));
        }
        let count = ids.len();
        Ok(Self {
            handles: Vec::with_capacity(bodies.len()),
            frames: rest.clone(),
            local: rest.clone(),
            world: vec![Frame::IDENTITY; count],
            nodes: ids,
            index,
            parent,
            rest,
            owner,
            pairs,
            head,
            bodies,
            colliders,
            joints,
        })
    }

    pub fn active(&self) -> bool {
        !self.handles.is_empty()
    }
    /// Spawned bodies in record order. Empty while alive.
    pub fn bodies(&self) -> &[RigidBodyHandle] {
        &self.handles
    }
    /// Ragdoll asset nodes that `node_pose` resolves, parents first.
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    fn fk(&mut self) {
        for i in 0..self.nodes.len() {
            self.world[i] = match self.parent[i] {
                Some(p) => self.world[p].mul(&self.local[i]),
                None => self.local[i],
            };
        }
    }

    /// `SpawnRagdol`. `player` is the player transform. `live` gives the world pose of a
    /// player node. `velocity` is the player parent body velocity. Skips a missing live pose.
    pub fn spawn(
        &mut self,
        physics: &mut PhysicsWorld,
        player: Pose,
        live: impl Fn(NodeId) -> Option<Pose>,
        velocity: Vector,
    ) {
        if self.active() {
            return;
        }
        let root = player * Pose::from_translation(Vector::NEG_Y * RESET_DEPTH);
        self.local.copy_from_slice(&self.rest);
        self.local[0] = Frame {
            position: root.translation,
            rotation: root.rotation,
            scale: self.rest[0].scale,
        };
        self.fk();
        // `ragdolPosScript.Upd`: each write moves the node and its children.
        for p in 0..self.pairs.len() {
            let (i, node) = self.pairs[p];
            let Some(pose) = live(node) else {
                continue;
            };
            let world = Frame {
                position: pose.translation,
                rotation: pose.rotation,
                scale: self.world[i].scale,
            };
            self.local[i] = match self.parent[i] {
                Some(parent) => self.world[parent].inverse_mul(&world),
                None => world,
            };
            self.fk();
        }
        for b in &self.bodies {
            let w = &self.world[b.node];
            let body = RigidBodyBuilder::dynamic()
                .pose(Pose::from_parts(w.position, w.rotation))
                .additional_mass(b.mass_kg)
                .linear_damping(b.linear_damping)
                .angular_damping(b.angular_damping)
                .linvel(velocity);
            self.handles.push(physics.insert_body(body));
        }
        for (body, collider) in &self.colliders {
            physics.insert_collider_with_parent(collider.clone(), self.handles[*body]);
        }
        for (body, connected, joint) in &self.joints {
            physics.insert_impulse_joint(
                self.handles[*connected],
                self.handles[*body],
                *joint,
                true,
            );
        }
        for i in 0..self.nodes.len() {
            self.frames[i] = match self.owner[i] {
                Some(b) => unscaled(&self.world[self.bodies[b].node]).inverse_mul(&self.world[i]),
                None => self.world[i],
            };
        }
        physics.refresh_queries();
    }

    /// `RagdolReset`: removes the bodies, their colliders, and the joints.
    pub fn remove(&mut self, physics: &mut PhysicsWorld) {
        for handle in self.handles.drain(..) {
            physics.remove_body(handle);
        }
        physics.refresh_queries();
    }

    /// World frame of a ragdoll node, with its lossy scale. None while alive.
    pub fn node_pose(&self, physics: &PhysicsWorld, node: NodeId) -> Option<Frame> {
        let i = *self.index.get(&node)?;
        if !self.active() {
            return None;
        }
        Some(match self.owner[i] {
            Some(b) => {
                let body = physics.rapier().bodies.get(self.handles[b])?.position();
                Frame {
                    position: body.translation,
                    rotation: body.rotation,
                    scale: Vector::ONE,
                }
                .mul(&self.frames[i])
            }
            None => self.frames[i],
        })
    }

    /// `ragdolHeadPos` world pose. None while alive.
    pub fn head_pose(&self, physics: &PhysicsWorld) -> Option<Pose> {
        let f = self.node_pose(physics, self.nodes[self.head])?;
        Some(Pose::from_parts(f.position, f.rotation))
    }

    /// `DeathLook`: `ragdolHeadPos.rotation = CamParent.rotation`. The head body turns about the
    /// head node and keeps its velocities.
    pub fn set_head_rotation(&self, physics: &mut PhysicsWorld, rotation: Rotation) {
        let Some(b) = self.owner[self.head].filter(|_| self.active()) else {
            return;
        };
        let offset = self.frames[self.head];
        physics.edit_body(self.handles[b], |body| {
            let pose = *body.position();
            let head = pose.translation + pose.rotation * offset.position;
            let turned = rotation * offset.rotation.inverse();
            body.set_position(
                Pose::from_parts(head - turned * offset.position, turned),
                true,
            );
        });
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
        if !self.active() {
            return;
        }
        let shift = Vector::from_array(shift);
        for (frame, owner) in self.frames.iter_mut().zip(&self.owner) {
            if owner.is_none() {
                frame.position -= shift;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{
        AssetId, BodyId, BodyRecord, ColliderGeometry, ColliderId, ColliderRecord, JointLimit,
        JointSpring, ManagedEvidence, RagdollJointKind, RagdollPose, RagdollPosePair, SourceId,
        Transform,
    };

    fn source() -> SourceId {
        SourceId {
            file: "fixture".into(),
            path_id: 0,
        }
    }
    fn transform(translation: [f32; 3]) -> Transform {
        Transform {
            translation,
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }
    }
    fn node(id: u32, parent: Option<u32>, translation: [f32; 3]) -> NodeRecord {
        NodeRecord {
            id: NodeId(id),
            source: source(),
            name: String::new(),
            hierarchy_path: String::new(),
            asset: AssetId(7),
            gltf_node: id,
            parent: parent.map(NodeId),
            active: true,
            transform: transform(translation),
        }
    }
    fn limit(limit: f32) -> JointLimit {
        JointLimit {
            limit,
            bounciness: 0.,
            contact_distance: 0.,
        }
    }

    /// Root, hips body, spine body, head body under the spine.
    fn fixture() -> (Vec<NodeRecord>, RagdollReference) {
        let nodes = vec![
            node(1, None, [0.; 3]),
            node(2, Some(1), [0., 1., 0.]),
            node(3, Some(2), [0., 0.3, 0.]),
            node(4, Some(3), [0., 0.4, 0.]),
        ];
        let body = |id: u32, node: u32, parent: Option<u32>, y: f32| BodyRecord {
            id: BodyId(id),
            source: source(),
            node: NodeId(node),
            dynamic: true,
            mass_kg: 5.,
            linear_damping: 0.,
            angular_damping: 0.05,
            pose: transform([0., y, 0.]),
            parent_body: parent.map(BodyId),
        };
        let collider = |id: u32, node: u32| ColliderRecord {
            id: ColliderId(id),
            source: source(),
            node: NodeId(node),
            body: BodyId(id),
            sensor: false,
            enabled: true,
            membership: 1,
            filter: u32::MAX,
            friction: 0.5,
            restitution: 0.,
            geometry: ColliderGeometry::Sphere {
                center: [0.; 3],
                radius: 0.1,
            },
            step_surface: None,
            tag: None,
        };
        let joint = |body: u32, connected: u32, anchor: f32| RagdollJoint {
            source: source(),
            kind: RagdollJointKind::Character,
            body: BodyId(body),
            connected_body: BodyId(connected),
            anchor: [0.; 3],
            connected_anchor: [0., anchor, 0.],
            collide_connected: false,
            axis: [1., 0., 0.],
            swing_axis: [0., 1., 0.],
            low_twist: limit(-20.),
            high_twist: limit(70.),
            swing1: limit(30.),
            swing2: limit(10.),
            twist_spring: JointSpring {
                spring: 0.,
                damper: 0.,
            },
            swing_spring: JointSpring {
                spring: 0.,
                damper: 0.,
            },
            projection: false,
            projection_distance: 0.1,
            projection_angle: 180.,
        };
        let r = RagdollReference {
            source: source(),
            asset: AssetId(7),
            root: NodeId(1),
            outfit_source: source(),
            invisible_material: source(),
            characters: Vec::new(),
            bodies: vec![
                body(1, 2, None, 1.),
                body(2, 3, Some(1), 1.3),
                body(3, 4, Some(2), 1.7),
            ],
            colliders: vec![collider(1, 2), collider(2, 3), collider(3, 4)],
            joints: vec![joint(2, 1, 0.3), joint(3, 2, 0.4)],
            pose: RagdollPose {
                source: source(),
                live_source: source(),
                head: NodeId(4),
                pairs: [(2, 20), (3, 30), (4, 40)]
                    .map(|(ragdoll, live)| RagdollPosePair {
                        ragdoll: NodeId(ragdoll),
                        live: NodeId(live),
                    })
                    .to_vec(),
            },
            evidence: ManagedEvidence {
                assembly_sha256: String::new(),
                methods: Vec::new(),
            },
        };
        (nodes, r)
    }

    #[test]
    fn spawn_copies_live_poses_into_one_body_per_record() {
        let (nodes, r) = fixture();
        let mut ragdoll = Ragdoll::from_parts(&nodes, &r).unwrap();
        let mut physics = PhysicsWorld::empty_for_tests();
        let player = Pose::from_parts(Vector::new(10., 0., -5.), Rotation::from_rotation_y(0.7));
        let live = |node: NodeId| {
            let i = node.0 as f32 / 10.;
            Some(Pose::from_parts(
                Vector::new(i, 2. * i, -i),
                Rotation::from_rotation_x(0.1 * i),
            ))
        };
        let velocity = Vector::new(3., 0., 1.);
        ragdoll.spawn(&mut physics, player, live, velocity);

        let rapier = physics.rapier();
        assert_eq!(rapier.bodies.len(), r.bodies.len());
        assert_eq!(rapier.colliders.len(), r.colliders.len());
        assert_eq!(rapier.impulse_joints.len(), r.joints.len());
        for (b, pair) in ragdoll.bodies().iter().zip(&r.pose.pairs) {
            let body = &rapier.bodies[*b];
            let expected = live(pair.live).unwrap();
            assert!((body.translation() - expected.translation).length() < 1e-5);
            assert!(body.rotation().angle_between(expected.rotation) < 1e-4);
            assert!((body.linvel() - velocity).length() < 1e-6);
        }
        let head = ragdoll.head_pose(&physics).unwrap();
        assert!((head.translation - live(NodeId(40)).unwrap().translation).length() < 1e-5);

        ragdoll.set_head_rotation(&mut physics, Rotation::from_rotation_y(1.));
        let head = ragdoll.head_pose(&physics).unwrap();
        assert!(head.rotation.angle_between(Rotation::from_rotation_y(1.)) < 1e-4);
        assert!((head.translation - live(NodeId(40)).unwrap().translation).length() < 1e-5);

        ragdoll.remove(&mut physics);
        assert!(!ragdoll.active());
        assert_eq!(physics.rapier().bodies.len(), 0);
        assert_eq!(physics.rapier().impulse_joints.len(), 0);
    }
}
