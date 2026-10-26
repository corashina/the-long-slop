//! The newest simulation frame. Bevy systems read the simulation only through it.
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use simulation::boundary::{Change, Handle, PoseKey, PoseRow, SimFrame};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub(crate) struct SimFrameResource {
    pub frame: SimFrame,
    poses: HashMap<PoseKey, Handle>,
}

impl SimFrameResource {
    /// Applies the frame changes to the pose index. Calls `retired` for each dropped handle.
    pub fn apply_changes(&mut self, mut retired: impl FnMut(Handle)) {
        for change in &self.frame.changes {
            match change {
                Change::Spawn { handle, key } => {
                    self.poses.insert(*key, *handle);
                }
                Change::Retire(handle) => {
                    self.poses.retain(|_, h| h != handle);
                    retired(*handle);
                }
                Change::Failure(_) => {}
            }
        }
    }
    /// Previous and current rows of `key`.
    pub fn pose(&self, key: PoseKey) -> Option<(PoseRow, PoseRow)> {
        let slot = self.poses.get(&key)?.0 as usize;
        Some((self.frame.previous[slot], self.frame.current[slot]))
    }
}
