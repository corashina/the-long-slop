//! Temporary stub of the PR 2 ragdoll API. PR 2 replaces this file.
use super::body::ik::Frame;
use crate::reference::NodeId;
use crate::vehicle::PhysicsWorld;

#[derive(Debug, Default)]
pub struct Ragdoll;

impl Ragdoll {
    /// The ragdoll has bodies. It is active only while the player is dead.
    pub fn active(&self) -> bool {
        false
    }
    /// Ragdoll asset nodes that `node_pose` resolves, parents first.
    pub fn nodes(&self) -> &[NodeId] {
        &[]
    }
    /// World position, rotation and lossy scale of a ragdoll asset node. None while alive.
    pub fn node_pose(&self, _physics: &PhysicsWorld, _node: NodeId) -> Option<Frame> {
        None
    }
}
