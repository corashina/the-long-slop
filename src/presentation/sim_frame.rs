//! The newest simulation frame. Bevy systems read the simulation only through it.
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use simulation::boundary::{
    Change, Feedback, Handle, ItemRow, ParticleGroupRow, ParticleRow, PoseKey, PoseRow, RopeAim,
    SimFrame, SpawnData,
};
use simulation::conditions::ConditionState;
use simulation::reference::{ConditionId, ItemId, NodeId};
use simulation::starting_area::spawned::SpawnedItem;
use simulation::vehicle::VehiclePose;
use simulation::world::GeneratedId;
use std::collections::HashMap;

/// One particle group of the frame: kind, kind ids and source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ParticleGroupKey {
    pub kind: u32,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub source: u32,
}

#[derive(Resource, Default)]
pub(crate) struct SimFrameResource {
    pub frame: SimFrame,
    poses: HashMap<PoseKey, (Handle, SpawnData)>,
    rig: Vec<NodeId>,
    spawned: Vec<SpawnedItem>,
    particle_groups: HashMap<ParticleGroupKey, usize>,
    conditions: HashMap<ConditionId, ConditionState>,
    /// Owners whose condition changed since the visuals last read them.
    pub changed_conditions: Vec<ConditionId>,
    /// Home cable aims waiting for the reference visuals.
    pub home_cable: Option<Vec<RopeAim>>,
    /// The world car the player last sat in, else the start car: id and prototype.
    pub followed_car: Option<(GeneratedId, u32)>,
    /// Feedback waiting for the HUD.
    pub feedback: Option<Box<Feedback>>,
}

impl SimFrameResource {
    /// Applies the frame changes to the pose index. Calls `retired` for each dropped handle.
    pub fn apply_changes(&mut self, mut retired: impl FnMut(Handle)) {
        for row in &self.frame.conditions {
            let id = ConditionId(row.id);
            self.conditions.insert(id, row.startup());
            self.changed_conditions.push(id);
        }
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
                Change::HomeCable(aims) => self.home_cable = Some(aims.clone()),
                Change::FollowedCar(car) => self.followed_car.clone_from(car),
                Change::Feedback(feedback) => self.feedback = Some(feedback.clone()),
                Change::Failure(_) => {}
            }
        }
    }
    pub fn condition(&self, id: ConditionId) -> &ConditionState {
        &self.conditions[&id]
    }
    /// Indexes the particle groups of a new frame.
    pub fn index_particles(&mut self) {
        self.particle_groups.clear();
        for (index, g) in self.frame.particle_groups.iter().enumerate() {
            let key = ParticleGroupKey {
                kind: g.kind,
                a: g.a,
                b: g.b,
                c: g.c,
                source: g.source,
            };
            self.particle_groups.insert(key, index);
        }
    }
    /// The group row and its live particles.
    pub fn particle_group(
        &self,
        key: ParticleGroupKey,
    ) -> Option<(ParticleGroupRow, &[ParticleRow])> {
        let row = self.frame.particle_groups[*self.particle_groups.get(&key)?];
        let start = row.start as usize;
        Some((
            row,
            &self.frame.particles[start..start + row.count as usize],
        ))
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
