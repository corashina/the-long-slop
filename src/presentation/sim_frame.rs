//! The newest simulation frame. Bevy systems read the simulation only through it.
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use simulation::boundary::{Change, Handle, ItemRow, PoseKey, PoseRow, SimFrame, SpawnData};
use simulation::reference::{ItemId, NodeId};
use simulation::starting_area::spawned::SpawnedItem;
use simulation::vehicle::VehiclePose;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub(crate) struct SimFrameResource {
    pub frame: SimFrame,
    poses: HashMap<PoseKey, (Handle, SpawnData)>,
    rig: Vec<NodeId>,
    spawned: Vec<SpawnedItem>,
}

impl SimFrameResource {
    /// Applies the frame changes to the pose index. Calls `retired` for each dropped handle.
    pub fn apply_changes(&mut self, mut retired: impl FnMut(Handle)) {
        for change in &self.frame.changes {
            match change {
                Change::Spawn { handle, key, data } => {
                    self.poses.insert(*key, (*handle, *data));
                    if let SpawnData::Spawned(item) = data {
                        self.spawned.push(*item);
                    }
                }
                Change::Retire(handle) => {
                    let spawned = &mut self.spawned;
                    self.poses.retain(|key, (h, _)| {
                        let keep = h != handle;
                        if !keep && let PoseKey::Item(id) = key {
                            spawned.retain(|item| item.id != *id);
                        }
                        keep
                    });
                    retired(*handle);
                }
                Change::PlayerRig(joints) => self.rig.clone_from(joints),
                Change::Failure(_) => {}
            }
        }
    }
    /// Previous and current rows of `key`.
    pub fn pose(&self, key: PoseKey) -> Option<(PoseRow, PoseRow)> {
        let slot = self.poses.get(&key)?.0.0 as usize;
        Some((self.frame.previous[slot], self.frame.current[slot]))
    }
    /// The pose of `key` between the last two steps.
    pub fn interpolated(&self, key: PoseKey) -> Option<VehiclePose> {
        self.pose(key).map(|(previous, current)| {
            super::interpolate(previous.pose(), current.pose(), self.frame.state.alpha)
        })
    }
    pub fn item(&self, item: ItemId) -> Option<ItemRow> {
        let slot = self.poses.get(&PoseKey::Item(item))?.0.0 as usize;
        Some(self.frame.items[slot])
    }
    /// Items with their spawn data.
    pub fn items(&self) -> impl Iterator<Item = (ItemId, SpawnData)> + '_ {
        self.poses.iter().filter_map(|(key, (_, data))| match key {
            PoseKey::Item(id) => Some((*id, *data)),
            PoseKey::Body(_) => None,
        })
    }
    /// Food prototype index and bite of `item`.
    pub fn food(&self, item: ItemId) -> Option<(usize, u8)> {
        match self.poses.get(&PoseKey::Item(item))?.1 {
            SpawnData::Food { prototype } => {
                Some((prototype as usize, self.item(item)?.bite as u8))
            }
            _ => None,
        }
    }
    /// Spawned item prototype bodies in spawn order.
    pub fn spawned(&self) -> &[SpawnedItem] {
        &self.spawned
    }
    /// Skin joint nodes of the player body, in `SimFrame::joints` order.
    pub fn rig(&self) -> &[NodeId] {
        &self.rig
    }
}
