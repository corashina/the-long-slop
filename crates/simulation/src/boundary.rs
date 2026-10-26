//! The data that crosses between the simulation and the renderer.
//! `SimFrame` goes out once per tick. `SimCommand` comes in once per rendered frame.
//! Poses and state are Pod rows; rare changes are postcard encoded.
use crate::reference::{BodyId, ItemId, OpeningId};
use crate::simulation::FrameInput;
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, VehiclePose};
use bevy_platform::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Index of one row in the frame pose arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Handle(pub u32);

/// Gives dense handles to keys. A retired handle is reused only after `release`.
#[derive(Debug)]
pub struct Handles<K> {
    by_key: HashMap<K, Handle>,
    keys: Vec<Option<K>>,
    free: Vec<Handle>,
}

impl<K: Copy + Eq + Hash> Default for Handles<K> {
    fn default() -> Self {
        Self {
            by_key: HashMap::default(),
            keys: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<K: Copy + Eq + Hash> Handles<K> {
    /// The handle of `key`, and true if the key is new.
    pub fn assign(&mut self, key: K) -> (Handle, bool) {
        if let Some(&handle) = self.by_key.get(&key) {
            return (handle, false);
        }
        let handle = self.free.pop().unwrap_or_else(|| {
            self.keys.push(None);
            Handle(self.keys.len() as u32 - 1)
        });
        self.keys[handle.0 as usize] = Some(key);
        self.by_key.insert(key, handle);
        (handle, true)
    }
    /// Removes the key. Its handle stays taken until `release`.
    pub fn retire(&mut self, handle: Handle) {
        if let Some(key) = self.keys[handle.0 as usize].take() {
            self.by_key.remove(&key);
        }
    }
    /// The renderer dropped the handle. It can go to a new key.
    pub fn release(&mut self, handle: Handle) {
        debug_assert!(
            self.keys[handle.0 as usize].is_none(),
            "release of a live handle"
        );
        self.free.push(handle);
    }
    /// Number of rows the handles index.
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn key(&self, handle: Handle) -> Option<K> {
        self.keys[handle.0 as usize]
    }
}

/// One posed object at one tick.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PoseRow {
    pub position: [f32; 3],
    pub flags: u32,
    pub rotation: [f32; 4],
}

impl PoseRow {
    /// The renderer shows the object.
    pub const VISIBLE: u32 = 1;
    /// The body sits in an inventory slot.
    pub const STORED: u32 = 1 << 1;
    pub fn pose(&self) -> VehiclePose {
        VehiclePose {
            position: self.position,
            rotation: self.rotation,
        }
    }
    fn new(pose: VehiclePose, flags: u32) -> Self {
        Self {
            position: pose.position,
            flags,
            rotation: pose.rotation,
        }
    }
}

/// Simulation values the renderer reads each frame. New fields come with their readers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct StateBlock {
    pub simulation_seconds: f64,
    /// Interpolation from `SimFrame::previous` to `SimFrame::current`.
    pub alpha: f32,
    /// Sleep clock factor on frame time.
    pub time_scale: f32,
    pub flags: u32,
    pub fov_degrees: f32,
    /// Items stored in the inventory slots, as `ItemId + 1`. 0 is an empty slot.
    pub stored_items: [u32; 4],
}

impl StateBlock {
    pub const STARTED: u32 = 1;
    pub const PAUSED: u32 = 1 << 1;
    pub const SEATED: u32 = 1 << 2;
    pub const SLEEPING: u32 = 1 << 3;
    pub const DEAD: u32 = 1 << 4;
    pub const RECOVERING: u32 = 1 << 5;
}

/// Valve of one opening: 0 closed, 1 open.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ValveRow {
    pub opening: u32,
    pub valve: f32,
}

/// What a pose handle stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PoseKey {
    Body(BodyId),
    Item(ItemId),
}

/// A rare frame event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    /// The handle row now poses `key`.
    Spawn {
        handle: Handle,
        key: PoseKey,
    },
    /// The renderer drops the handle, then releases it.
    Retire(Handle),
    Failure(String),
}

/// The simulation output of one tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SimFrame {
    /// The last command sequence the simulation consumed.
    pub seq_consumed: u64,
    pub state: StateBlock,
    /// Poses at the step before `current`, indexed by handle.
    pub previous: Vec<PoseRow>,
    pub current: Vec<PoseRow>,
    pub valves: Vec<ValveRow>,
    pub changes: Vec<Change>,
}

impl SimFrame {
    pub fn valve(&self, opening: OpeningId) -> f32 {
        self.valves
            .iter()
            .find(|v| v.opening == opening.0)
            .map_or(0., |v| v.valve)
    }
    pub fn item_stored(&self, item: ItemId) -> bool {
        self.state.stored_items.contains(&(item.0 + 1))
    }
}

/// The renderer input of one rendered frame.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimCommand {
    pub seq: u64,
    pub delta_seconds: f64,
    pub input: FrameInput,
}

#[derive(Debug, PartialEq)]
pub struct DecodeError(pub String);

/// Writes the frame into `out`. `out` keeps its capacity between frames.
pub fn encode(frame: &SimFrame, out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&frame.seq_consumed.to_le_bytes());
    out.extend_from_slice(bytemuck::bytes_of(&frame.state));
    out.extend_from_slice(&(frame.current.len() as u32).to_le_bytes());
    out.extend_from_slice(bytemuck::cast_slice(&frame.previous));
    out.extend_from_slice(bytemuck::cast_slice(&frame.current));
    out.extend_from_slice(&(frame.valves.len() as u32).to_le_bytes());
    out.extend_from_slice(bytemuck::cast_slice(&frame.valves));
    *out = postcard::to_extend(&frame.changes, std::mem::take(out)).expect("frame changes encode");
}

/// Reads `bytes` into `frame`, reusing its arrays.
pub fn decode(bytes: &[u8], frame: &mut SimFrame) -> Result<(), DecodeError> {
    let mut rest = bytes;
    let mut take = |n: usize| {
        if rest.len() < n {
            return Err(DecodeError("short frame".into()));
        }
        let (head, tail) = rest.split_at(n);
        rest = tail;
        Ok(head)
    };
    frame.seq_consumed = u64::from_le_bytes(take(8)?.try_into().expect("8 bytes"));
    frame.state = bytemuck::pod_read_unaligned(take(size_of::<StateBlock>())?);
    let rows = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
    for poses in [&mut frame.previous, &mut frame.current] {
        poses.clear();
        poses.extend(
            take(rows * size_of::<PoseRow>())?
                .as_chunks::<{ size_of::<PoseRow>() }>()
                .0
                .iter()
                .map(|row| bytemuck::cast::<_, PoseRow>(*row)),
        );
    }
    let valves = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
    frame.valves.clear();
    frame.valves.extend(
        take(valves * size_of::<ValveRow>())?
            .as_chunks::<{ size_of::<ValveRow>() }>()
            .0
            .iter()
            .map(|row| bytemuck::cast::<_, ValveRow>(*row)),
    );
    frame.changes = postcard::from_bytes(rest).map_err(|e| DecodeError(e.to_string()))?;
    Ok(())
}

pub fn encode_command(command: &SimCommand, out: &mut Vec<u8>) {
    out.clear();
    *out = postcard::to_extend(command, std::mem::take(out)).expect("command encode");
}

pub fn decode_command(bytes: &[u8]) -> Result<SimCommand, DecodeError> {
    postcard::from_bytes(bytes).map_err(|e| DecodeError(e.to_string()))
}

/// Pose handles of the starting area bodies and items.
#[derive(Debug, Default)]
pub struct FramePoses {
    handles: Handles<PoseKey>,
    /// Tick stamp per handle. A live handle without the current stamp left the area.
    seen: Vec<u32>,
    stamp: u32,
}

impl FramePoses {
    /// Writes the area poses into `frame` and adds `Spawn` and `Retire` changes.
    pub fn fill(&mut self, area: &StartingArea, physics: &PhysicsWorld, frame: &mut SimFrame) {
        self.stamp = self.stamp.wrapping_add(1);
        frame.previous.resize(self.seen.len(), PoseRow::default());
        frame.current.resize(self.seen.len(), PoseRow::default());
        let stored = area.inventory.slots.map(|id| {
            id.and_then(|id| area.items.get(id))
                .filter(|i| {
                    matches!(
                        i.location,
                        crate::interaction::items::ItemLocation::Stored(_)
                    )
                })
                .and_then(|i| i.body)
        });
        for (id, previous, current) in &area.render_poses {
            let body = area.bodies[id];
            let stored = stored.contains(&Some(body));
            let flags = if physics.body_active(body) || stored {
                PoseRow::VISIBLE
            } else {
                0
            } | if stored { PoseRow::STORED } else { 0 };
            self.write(PoseKey::Body(*id), *previous, *current, flags, frame);
        }
        for (id, previous, current) in &area.item_render_poses {
            let visible = area
                .items
                .get(*id)
                .is_some_and(|item| item.inactive_pose.is_none());
            let flags = if visible { PoseRow::VISIBLE } else { 0 };
            self.write(PoseKey::Item(*id), *previous, *current, flags, frame);
        }
        for (slot, seen) in self.seen.iter().enumerate() {
            let handle = Handle(slot as u32);
            if *seen != self.stamp && self.handles.key(handle).is_some() {
                self.handles.retire(handle);
                frame.previous[slot] = PoseRow::default();
                frame.current[slot] = PoseRow::default();
                frame.changes.push(Change::Retire(handle));
            }
        }
    }
    /// The renderer dropped `handle`.
    pub fn release(&mut self, handle: Handle) {
        self.handles.release(handle);
    }
    fn write(
        &mut self,
        key: PoseKey,
        previous: VehiclePose,
        current: VehiclePose,
        flags: u32,
        frame: &mut SimFrame,
    ) {
        let (handle, new) = self.handles.assign(key);
        let slot = handle.0 as usize;
        if slot >= self.seen.len() {
            self.seen.resize(slot + 1, 0);
        }
        if slot >= frame.current.len() {
            frame.previous.resize(slot + 1, PoseRow::default());
            frame.current.resize(slot + 1, PoseRow::default());
        }
        if new {
            frame.changes.push(Change::Spawn { handle, key });
        }
        self.seen[slot] = self.stamp;
        frame.previous[slot] = PoseRow::new(previous, flags);
        frame.current[slot] = PoseRow::new(current, flags);
    }
}

#[cfg(test)]
mod tests;
