//! Source player body: `fpscontroller.UpdHandPos` targets, then the five `IKscript` chains.
//! All frames are player-local: Player, then `CharacterRoot`, then the bones.
pub mod ik;

use crate::reference::{ClothingReference, NodeId, NodeRecord, ReferenceError, ReferenceManifest};
use crate::vehicle::VehiclePose;
use ik::{Frame, IkChain, MAX_BONES};
use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;
use std::collections::HashMap;

/// Temporary reach API stub. PR 2 replaces it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReachFrame {
    World,
    Body(RigidBodyHandle),
    Node(NodeId),
}
/// Temporary reach API stub. PR 2 replaces it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReachTarget {
    pub frame: ReachFrame,
    pub pose: VehiclePose,
}

/// Left, then right. Poses are player-local in `BodyInputs` and body-local on a bound seat.
#[derive(Clone, Copy, Debug)]
pub struct SeatFrames {
    /// `seatscript.defPose`: the authored sit pose when a seat frame is missing.
    pub def_pose: usize,
    /// `Bdriving` in a car.
    pub driving: bool,
    /// `seatscript.LHpos`, `RHpos`.
    pub hands: [Option<VehiclePose>; 2],
    /// `seatscript.LLpos`, `RLpos`.
    pub legs: [Option<VehiclePose>; 2],
    /// `carscript.LHDrivePos`, `RHDrivePos`.
    pub driving_hands: [Option<VehiclePose>; 2],
    /// `carscript.LLDrivePos`, `RLDrivePos`.
    pub driving_legs: [Option<VehiclePose>; 2],
}

/// A loose item in the hands (`pickedUp`).
#[derive(Clone, Copy, Debug)]
pub struct CarriedItem {
    /// `pickedUpPoint`.
    pub grab: VehiclePose,
    /// `TpickedUpParent`.
    pub anchor: VehiclePose,
    pub mass: f32,
    pub double_hand: bool,
}

/// An equipped item (`inHandP`).
#[derive(Clone, Copy, Debug)]
pub struct InHandItem {
    /// The `RH` frame.
    pub hand: VehiclePose,
    pub double_hand: bool,
}

/// One step of player state. Poses are player-local.
#[derive(Clone, Copy, Debug)]
pub struct BodyInputs {
    pub camera: VehiclePose,
    /// `fpscontroller.Th`.
    pub head: VehiclePose,
    /// `duckValue`: 0 ducked, 1 standing.
    pub duck: f32,
    pub sleeping: bool,
    /// `currentAdd`: the head bob step. 0 when the bob does not advance.
    pub walk_add: f32,
    pub forward: bool,
    pub backward: bool,
    pub carried: Option<CarriedItem>,
    pub in_hand: Option<InHandItem>,
    pub seat: Option<SeatFrames>,
    /// `pointAt` and `pointingValue`.
    pub point: Option<(VehiclePose, f32)>,
}

/// Node frames: player-local world and parent-local. Parents come before children.
struct Rig {
    ids: Vec<NodeId>,
    parent: Vec<Option<usize>>,
    local: Vec<Frame>,
    world: Vec<Frame>,
}
impl Rig {
    fn new(
        nodes: &[NodeRecord],
        root: NodeId,
        needed: impl Iterator<Item = NodeId>,
    ) -> Result<Self, ReferenceError> {
        let asset = nodes
            .iter()
            .find(|n| n.id == root)
            .ok_or_else(|| ReferenceError("Missing player root".into()))?
            .asset;
        let nodes: HashMap<_, _> = nodes
            .iter()
            .filter(|n| n.asset == asset)
            .map(|n| (n.id, n))
            .collect();
        let mut depth = HashMap::new();
        for id in needed {
            let mut chain = Vec::new();
            let mut current = id;
            while current != root {
                let node = nodes
                    .get(&current)
                    .ok_or_else(|| ReferenceError("Body node outside the player".into()))?;
                chain.push(current);
                current = node
                    .parent
                    .ok_or_else(|| ReferenceError("Body node outside the player".into()))?;
            }
            for (i, id) in chain.iter().rev().enumerate() {
                depth.insert(*id, i);
            }
        }
        let mut ids: Vec<NodeId> = depth.keys().copied().collect();
        ids.sort_by_key(|id| (depth[id], id.0));
        let index: HashMap<_, _> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let local: Vec<Frame> = ids
            .iter()
            .map(|id| {
                let t = &nodes[id].transform;
                Frame {
                    position: Vector::from_array(t.translation),
                    rotation: Rotation::from_array(t.rotation),
                    scale: Vector::from_array(t.scale),
                }
            })
            .collect();
        let mut rig = Self {
            parent: ids
                .iter()
                .map(|id| nodes[id].parent.and_then(|p| index.get(&p).copied()))
                .collect(),
            world: local.clone(),
            local,
            ids,
        };
        rig.fk();
        Ok(rig)
    }
    fn index(&self, id: NodeId) -> usize {
        self.ids.iter().position(|n| *n == id).unwrap()
    }
    fn fk(&mut self) {
        for i in 0..self.ids.len() {
            if let Some(p) = self.parent[i] {
                self.world[i] = self.world[p].mul(&self.local[i]);
            }
        }
    }
    /// Unity `position` and `rotation` setters. Children update on the next `fk`.
    fn set(&mut self, i: usize, position: Vector, rotation: Rotation) {
        self.world[i].position = position;
        self.world[i].rotation = rotation;
        self.local[i] = match self.parent[i] {
            Some(p) => self.world[p].inverse_mul(&self.world[i]),
            None => self.world[i],
        };
    }
    /// Unity `parent` setter with world pose kept.
    fn reparent(&mut self, i: usize, parent: Option<usize>) {
        self.parent[i] = parent;
        self.local[i] = match parent {
            Some(p) => self.world[p].inverse_mul(&self.world[i]),
            None => self.world[i],
        };
    }
    fn target(&self, i: usize) -> (Vector, Rotation) {
        (self.world[i].position, self.world[i].rotation)
    }
}

struct Chain {
    ik: IkChain,
    owner: usize,
    target: usize,
    pole: Option<usize>,
    bones: [usize; MAX_BONES],
}

/// `VCurrentLHT` identity. A change unlocks the hand for `VMoveLockTime`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum HandFrame {
    Node(usize),
    Grab,
    Hand,
    Seat,
    Drive,
    Point,
}

/// Source `MoveLegsWalk` waypoint. `state` is `walkAnimationState` in [0, 1].
fn walk_index(state: f32, offset: f32, count: usize) -> usize {
    let mut t = state + offset;
    while t > 1. {
        t -= 1.;
    }
    while t < 0. {
        t += 1.;
    }
    ((t * count as f32) as i32).clamp(0, count as i32 - 1) as usize
}

fn pose(p: &VehiclePose) -> (Vector, Rotation) {
    (
        Vector::from_array(p.position),
        Rotation::from_array(p.rotation),
    )
}

fn vehicle_pose(f: &Frame) -> VehiclePose {
    VehiclePose {
        position: f.position.to_array(),
        rotation: f.rotation.to_array(),
    }
}

/// Unity `Vector3.Lerp` and `Quaternion.Lerp` clamp `t`.
fn lerp(a: (Vector, Rotation), b: (Vector, Rotation), tp: f32, tr: f32) -> (Vector, Rotation) {
    (
        a.0.lerp(b.0, tp.clamp(0., 1.)),
        a.1.lerp(b.1, tr.clamp(0., 1.)),
    )
}

pub struct BodyState {
    rig: Rig,
    chains: Vec<Chain>,
    joints: Vec<usize>,
    joint_scale: f32,
    character_root: usize,
    root_stand: usize,
    root_duck: usize,
    /// Left, right.
    def_hands: [usize; 2],
    def_hands_stand: [usize; 2],
    def_hands_duck: [usize; 2],
    def_legs: [usize; 2],
    def_def_legs: [usize; 2],
    /// `HHTarget`.
    head_target: usize,
    /// `LHTarget`, `RHTarget`, `LLTarget`, `RLTarget`.
    targets: [usize; 4],
    walk_paths: [Vec<usize>; 2],
    /// `LHSitPose`, `RHSitPose`, `LLSitPose`, `RLSitPose`.
    sit_poses: [Vec<usize>; 4],
    walk_offsets: [f32; 2],
    walk_lerp: f32,
    walk_rot_lerp: f32,
    walk_speed: f32,
    one_hand_limit: f32,
    hand_speed: f32,
    leg_speed: f32,
    lock_time: f32,
    /// `RHRotSnapHand`.
    hand_rotation: Rotation,
    /// `walkAnimationState`.
    walk: f32,
    /// `VLHP`/`VLHR`, `VRHP`/`VRHR`, `VLLP`/`VLLR`, `VRLP`/`VRLR`.
    limbs: [(Vector, Rotation); 4],
    /// `VLastLHT`, `VLastRHT`.
    last_hands: [Option<HandFrame>; 2],
    /// `VMoveLHValue`, `VMoveRHValue`.
    hand_timers: [f32; 2],
    carrying: bool,
}

impl BodyState {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        Self::from_parts(&r.nodes, &r.clothing)
    }
    fn from_parts(
        nodes: &[NodeRecord],
        clothing: &ClothingReference,
    ) -> Result<Self, ReferenceError> {
        let p = &clothing.pose;
        let character = clothing
            .characters
            .first()
            .and_then(|c| c.skins.first())
            .ok_or_else(|| ReferenceError("Missing male body skin".into()))?;
        let anchor = |name: &str| {
            p.anchors
                .get(name)
                .map(|a| a.node)
                .ok_or_else(|| ReferenceError(format!("Missing player anchor {name}")))
        };
        let target = |name: &str| {
            p.targets
                .get(name)
                .copied()
                .ok_or_else(|| ReferenceError(format!("Missing player target {name}")))
        };
        let anchors = [
            "CharacterRoot",
            "CharacterStandPos",
            "CharacterDuckPos",
            "defLHPos",
            "defRHPos",
            "defLHPosStand",
            "defRHPosStand",
            "defLHPosDuck",
            "defRHPosDuck",
            "defLLPos",
            "defRLPos",
            "defdefLLPos",
            "defdefRLPos",
        ]
        .map(anchor);
        let anchors: Vec<NodeId> = anchors.into_iter().collect::<Result<_, _>>()?;
        let targets = ["HHTarget", "LHTarget", "RHTarget", "LLTarget", "RLTarget"].map(target);
        let targets: Vec<NodeId> = targets.into_iter().collect::<Result<_, _>>()?;
        let needed = character
            .joints
            .iter()
            .chain(&anchors)
            .chain(&targets)
            .chain(p.walk_paths.iter().flatten())
            .chain(p.sit_poses.iter().flatten())
            .chain(p.ik_chains.iter().flat_map(|c| {
                c.bones
                    .iter()
                    .chain([&c.owner, &c.target])
                    .chain(c.pole.iter())
            }))
            .copied();
        let mut rig = Rig::new(nodes, p.root, needed)?;
        // `IKscript.Start` runs in Unity world space, at the authored player pose.
        let root = nodes
            .iter()
            .find(|n| n.id == p.root)
            .map(|n| Frame {
                position: Vector::from_array(n.transform.translation),
                rotation: Rotation::from_array(n.transform.rotation),
                scale: Vector::from_array(n.transform.scale),
            })
            .unwrap();
        let mut chains = Vec::with_capacity(p.ik_chains.len());
        for c in &p.ik_chains {
            if c.bones.len() > MAX_BONES || c.bones.len() != c.chain_length as usize + 1 {
                return Err(ReferenceError("Player IK chain length".into()));
            }
            let mut bones = [0; MAX_BONES];
            let mut start = [Frame::IDENTITY; MAX_BONES];
            for (i, id) in c.bones.iter().enumerate() {
                bones[i] = rig.index(*id);
                start[i] = root.mul(&rig.world[bones[i]]);
            }
            let target = rig.index(c.target);
            let ik = IkChain::new(
                &start[..c.bones.len()],
                &root.mul(&rig.world[target]),
                Rotation::from_array(c.offsets),
                c.force_end,
                c.stretch,
            );
            let owner = rig.index(c.owner);
            for &bone in &bones[..c.bones.len()] {
                rig.reparent(bone, Some(owner));
            }
            chains.push(Chain {
                ik,
                owner,
                target,
                pole: c.use_pole.then(|| c.pole.map(|n| rig.index(n))).flatten(),
                bones,
            });
        }
        // `HHTarget` takes its pose from the camera or `Th` each frame.
        let head_target = rig.index(targets[0]);
        rig.reparent(head_target, None);
        let i = |id: NodeId| rig.index(id);
        let character_root = i(anchors[0]);
        let joints: Vec<usize> = character.joints.iter().map(|n| i(*n)).collect();
        let joint_scale = rig.world[joints[0]].scale.x / rig.world[character_root].scale.x;
        if joints.iter().any(|j| {
            let s = rig.world[*j].scale / rig.world[character_root].scale;
            (s - Vector::splat(joint_scale)).abs().max_element() > 1e-3 * joint_scale
        }) {
            return Err(ReferenceError("Player joints differ in scale".into()));
        }
        let scalar = |map: &std::collections::BTreeMap<String, f32>, name: &str| {
            map.get(name)
                .copied()
                .ok_or_else(|| ReferenceError(format!("Missing player scalar {name}")))
        };
        let limbs = [1, 2, 3, 4].map(|t| rig.target(i(targets[t])));
        Ok(Self {
            chains,
            joint_scale,
            character_root,
            root_stand: i(anchors[1]),
            root_duck: i(anchors[2]),
            def_hands: [i(anchors[3]), i(anchors[4])],
            def_hands_stand: [i(anchors[5]), i(anchors[6])],
            def_hands_duck: [i(anchors[7]), i(anchors[8])],
            def_legs: [i(anchors[9]), i(anchors[10])],
            def_def_legs: [i(anchors[11]), i(anchors[12])],
            head_target,
            targets: [1, 2, 3, 4].map(|t| i(targets[t])),
            walk_paths: p.walk_paths.clone().map(|w| w.into_iter().map(i).collect()),
            sit_poses: p.sit_poses.clone().map(|w| w.into_iter().map(i).collect()),
            walk_offsets: p.walk_offsets,
            walk_lerp: scalar(&p.scalars, "walkLerp")?,
            walk_rot_lerp: scalar(&p.scalars, "walkRotLerp")?,
            walk_speed: scalar(&p.scalars, "walkSpeedModifier")?,
            one_hand_limit: scalar(&p.scalars, "onehandLimit")?,
            hand_speed: scalar(&p.method_inputs, "VLerpSpeedHand")?,
            leg_speed: scalar(&p.method_inputs, "VLerpSpeedLeg")?,
            lock_time: scalar(&p.method_inputs, "VMoveLockTime")?,
            hand_rotation: Rotation::from_array(p.right_hand_rotation),
            walk: 0.,
            limbs,
            last_hands: [None; 2],
            hand_timers: [0.; 2],
            carrying: false,
            joints,
            rig,
        })
    }

    pub fn step(&mut self, input: &BodyInputs, dt: f32) {
        // fpscontroller.Update parents HHTarget to the camera, or to Th during sleep.
        let (position, rotation) = pose(if input.sleeping {
            &input.head
        } else {
            &input.camera
        });
        self.rig.set(self.head_target, position, rotation);
        // fpscontroller.Move advances the walk cycle with the head bob.
        if input.forward {
            self.walk += input.walk_add * self.walk_speed;
        } else if input.backward {
            self.walk -= input.walk_add * self.walk_speed;
        }
        // Pickup and Drop unlock both hands.
        if input.carried.is_some() != self.carrying {
            self.carrying = input.carried.is_some();
            self.hand_timers = [self.lock_time; 2];
        }
        self.upd_hand_pos(input, dt);
        self.upd_v_hand_leg_targets(dt);
        // IKscript.LateUpdate, in component order.
        self.rig.fk();
        for c in &self.chains {
            let count = c.ik.bone_count();
            let mut bones = [Frame::IDENTITY; MAX_BONES];
            for (b, &i) in bones.iter_mut().zip(&c.bones[..count]) {
                *b = self.rig.local[i];
            }
            let owner = self.rig.world[c.owner];
            let target = self.rig.world[c.target];
            let pole = c.pole.map(|p| self.rig.world[p]);
            c.ik.solve(&owner, &target, pole.as_ref(), &mut bones[..count]);
            for (b, &i) in bones.iter().zip(&c.bones[..count]) {
                self.rig.local[i] = *b;
            }
            self.rig.fk();
        }
    }

    /// The frame of a seated limb: drive frame, seat frame, then authored sit pose.
    fn seat_frame(&self, seat: &SeatFrames, limb: usize) -> ((Vector, Rotation), HandFrame) {
        let side = limb % 2;
        let (drive, own) = if limb < 2 {
            (seat.driving_hands[side], seat.hands[side])
        } else {
            (seat.driving_legs[side], seat.legs[side])
        };
        if let Some(p) = drive.filter(|_| seat.driving) {
            return (pose(&p), HandFrame::Drive);
        }
        if let Some(p) = own {
            return (pose(&p), HandFrame::Seat);
        }
        let node = self.sit_poses[limb][seat.def_pose];
        (self.rig.target(node), HandFrame::Node(node))
    }

    /// fpscontroller.UpdHandPos without usables or per-item hand frames.
    fn upd_hand_pos(&mut self, input: &BodyInputs, dt: f32) {
        let def = |i: usize| {
            (
                self.rig.target(self.def_hands[i]),
                HandFrame::Node(self.def_hands[i]),
            )
        };
        let mut left = None;
        let mut right = if let Some(h) = input.in_hand {
            let (p, r) = pose(&h.hand);
            let frame = ((p, r * self.hand_rotation), HandFrame::Hand);
            if h.double_hand {
                left = Some(frame);
            }
            frame
        } else if let Some(seat) = &input.seat {
            self.seat_frame(seat, 1)
        } else {
            def(1)
        };
        let seated_left = |s: &Self| match &input.seat {
            Some(seat) => s.seat_frame(seat, 0),
            None => def(0),
        };
        if let Some(c) = &input.carried {
            let grab = (pose(&c.grab), HandFrame::Grab);
            if input.in_hand.is_none() {
                right = grab;
                left = Some(if c.double_hand || c.mass > self.one_hand_limit {
                    ((pose(&c.anchor).0, grab.0.1), HandFrame::Grab)
                } else {
                    seated_left(self)
                });
            } else {
                left = Some(grab);
            }
        } else if input.seat.is_some() || input.in_hand.is_none_or(|h| !h.double_hand) {
            left = Some(seated_left(self));
        }
        let left = left.unwrap();
        self.limbs[0] = left.0;
        self.limbs[1] = right.0;
        if let Some(seat) = &input.seat {
            if seat.driving {
                // Without a drive frame the leg keeps its last target.
                for (side, p) in seat.driving_legs.iter().enumerate() {
                    if let Some(p) = p {
                        self.limbs[2 + side] = pose(p);
                    }
                }
            } else {
                self.limbs[2] = self.seat_frame(seat, 2).0;
                self.limbs[3] = self.seat_frame(seat, 3).0;
            }
        } else {
            self.limbs[2] = self.rig.target(self.def_legs[0]);
            self.limbs[3] = self.rig.target(self.def_legs[1]);
        }
        // Source quirk: with no usable, VCurrentRHT is always defRHPos.
        right.1 = HandFrame::Node(self.def_hands[1]);
        // The body follows the duck value after the targets are read.
        let duck = input.duck.clamp(0., 1.);
        let root = self.rig.world[self.character_root].rotation;
        let position = self.rig.world[self.root_duck]
            .position
            .lerp(self.rig.world[self.root_stand].position, duck);
        self.rig.set(self.character_root, position, root);
        for side in 0..2 {
            let (dp, dr) = self.rig.target(self.def_hands_duck[side]);
            let (sp, sr) = self.rig.target(self.def_hands_stand[side]);
            self.rig
                .set(self.def_hands[side], dp.lerp(sp, duck), dr.lerp(sr, duck));
        }
        self.move_legs_walk(input, dt);
        let mut frames = [left.1, right.1];
        // Mutat: the left hand with an item in the hands, else the right hand.
        if let Some((at, value)) = input.point.filter(|p| p.1 > 0.) {
            let side = if input.carried.is_some() || input.in_hand.is_some() {
                0
            } else {
                1
            };
            let (p, r) = pose(&at);
            self.limbs[side] = (self.limbs[side].0.lerp(p, value), r);
            frames[side] = HandFrame::Point;
        }
        for (side, frame) in frames.into_iter().enumerate() {
            if self.last_hands[side] != Some(frame) {
                self.hand_timers[side] = self.lock_time;
            }
            self.last_hands[side] = Some(frame);
        }
    }

    fn move_legs_walk(&mut self, input: &BodyInputs, dt: f32) {
        if !input.forward && !input.backward {
            for side in 0..2 {
                let leg = self.def_legs[side];
                let rest = self.rig.world[self.def_def_legs[side]].position;
                let rotation = self.rig.world[leg].rotation;
                let position = self.rig.world[leg]
                    .position
                    .lerp(rest, (dt * self.walk_lerp).clamp(0., 1.));
                self.rig.set(leg, position, rotation);
            }
            return;
        }
        while self.walk > 1. {
            self.walk -= 1.;
        }
        while self.walk < 0. {
            self.walk += 1.;
        }
        // Source order: right leg, then left leg.
        for side in [1, 0] {
            let path = &self.walk_paths[side];
            let point = path[walk_index(self.walk, self.walk_offsets[side], path.len())];
            let leg = self.def_legs[side];
            let (p, r) = lerp(
                self.rig.target(leg),
                self.rig.target(point),
                dt * self.walk_lerp,
                dt * self.walk_rot_lerp,
            );
            self.rig.set(leg, p, r);
        }
    }

    /// fpscontroller.UpdVHandLegTargets. Only hands lock.
    fn upd_v_hand_leg_targets(&mut self, dt: f32) {
        for limb in 0..4 {
            let locked = limb < 2 && {
                let timer = &mut self.hand_timers[limb];
                if *timer > 0. {
                    *timer -= dt;
                    false
                } else {
                    true
                }
            };
            let target = self.targets[limb];
            let (p, r) = if locked {
                self.limbs[limb]
            } else {
                let speed = if limb < 2 {
                    self.hand_speed
                } else {
                    self.leg_speed
                };
                lerp(
                    self.rig.target(target),
                    self.limbs[limb],
                    dt * speed,
                    dt * speed,
                )
            };
            self.rig.set(target, p, r);
        }
    }

    /// Temporary reach API stub. PR 2 replaces it.
    pub fn move_nearest_hand(&mut self, _target: ReachTarget, _delay: Option<f32>) {}
    /// Temporary reach API stub. PR 2 replaces it.
    pub fn move_nearest_leg(&mut self, _target: ReachTarget, _delay: Option<f32>) {}

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
    /// Skin joint `i` of the male body, in glTF skin order.
    pub fn joint_node(&self, i: usize) -> NodeId {
        self.rig.ids[self.joints[i]]
    }
    /// Joint `i` relative to `CharacterRoot`. Its scale is `joint_scale`.
    pub fn joint_local(&self, i: usize) -> VehiclePose {
        vehicle_pose(
            &self.rig.world[self.character_root].inverse_mul(&self.rig.world[self.joints[i]]),
        )
    }
    /// Uniform scale of every joint relative to `CharacterRoot`.
    pub fn joint_scale(&self) -> f32 {
        self.joint_scale
    }
    /// `CharacterRoot` relative to the Player.
    pub fn root_local(&self) -> VehiclePose {
        vehicle_pose(&self.rig.world[self.character_root])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_waypoint_index_and_offset() {
        // Source walk_offsets: left 0.5, right 0. Six waypoints per leg.
        assert_eq!(walk_index(0., 0., 6), 0);
        assert_eq!(walk_index(0., 0.5, 6), 3);
        assert_eq!(walk_index(0.2, 0., 6), 1);
        assert_eq!(walk_index(0.2, 0.5, 6), 4);
        assert_eq!(walk_index(0.6, 0.5, 6), 0);
        assert_eq!(walk_index(0.9, 0.5, 6), 2);
        // 1 stays 1, so it clamps to the last waypoint.
        assert_eq!(walk_index(1., 0., 6), 5);
        assert_eq!(walk_index(0.5, 0.5, 6), 5);
    }

    #[test]
    #[ignore = "Requires exported Steam assets"]
    fn body_standing_pose_matches_source() {
        use rapier3d::parry::shape::Shape;
        #[derive(serde::Deserialize)]
        struct Manifest {
            nodes: Vec<NodeRecord>,
            clothing: ClothingReference,
            player: crate::reference::PlayerReference,
        }
        let r: Manifest =
            serde_json::from_slice(&std::fs::read("web/assets/reference/manifest.json").unwrap())
                .unwrap();
        let mut body = BodyState::from_parts(&r.nodes, &r.clothing).unwrap();
        let head = VehiclePose {
            position: r.player.posture.standing_head.translation,
            rotation: [0., 0., 0., 1.],
        };
        let input = BodyInputs {
            camera: head,
            head,
            duck: 1.,
            sleeping: false,
            walk_add: 0.,
            forward: false,
            backward: false,
            carried: None,
            in_hand: None,
            seat: None,
            point: None,
        };
        for _ in 0..120 {
            body.step(&input, 1. / 60.);
        }
        let node = |pid: i64| {
            let id = r
                .nodes
                .iter()
                .find(|n| n.source.file == "level1" && n.source.path_id == pid)
                .unwrap()
                .id;
            body.rig.world[body.rig.index(id)]
        };
        let bottom = crate::player::collision::PlayerSweepShape::new(&r.player)
            .compute_local_aabb()
            .mins
            .y;
        // LeftLegMoveObj, RightLegMoveObj.
        for pid in [30531, 34137] {
            let foot = node(pid).position;
            assert!(
                (foot.y - bottom).abs() < 0.05,
                "foot {foot} bottom {bottom}"
            );
        }
        // HeadMoveObj.
        let head_bone = node(29226).position;
        assert!((head_bone - Vector::from_array(head.position)).length() < 1e-3);
        // The thigh keeps its length. Source stretch and end passes can change the others.
        for c in &r.clothing.pose.ik_chains[..2] {
            let [hip, knee] = [c.bones[0], c.bones[1]].map(|n| body.rig.world[body.rig.index(n)]);
            let rest = r.nodes.iter().find(|n| n.id == c.bones[1]).unwrap();
            let length = (hip.scale * Vector::from_array(rest.transform.translation)).length();
            assert!(((knee.position - hip.position).length() - length).abs() < 1e-4);
        }
    }
}
