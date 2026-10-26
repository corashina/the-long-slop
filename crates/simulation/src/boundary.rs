//! The data that crosses between the simulation and the renderer.
//! `SimFrame` goes out once per tick. `SimCommand` comes in once per rendered frame.
//! Poses and state are Pod rows; rare changes are postcard encoded.
use crate::interaction::items::ItemLocation;
use crate::lighting::{
    ControlPoseOutput, CycleOutput, LampMaterialOutput, LightOutput, NodeVisibilityOutput,
};
use crate::reference::{
    BodyId, ConditionId, ConditionStartup, ItemId, NodeId, OpeningId, ReferenceManifest,
};
use crate::simulation::FrameInput;
use crate::starting_area::StartingArea;
use crate::starting_area::spawned::SpawnedItem;
use crate::vehicle::{PhysicsWorld, VehiclePose};
use crate::world::GeneratedId;
use bevy_platform::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use serde::de::DeserializeOwned;
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
    /// The object sits in an inventory slot.
    pub const STORED: u32 = 1 << 1;
    /// The body or item takes part in physics.
    pub const ACTIVE: u32 = 1 << 2;
    pub fn pose(&self) -> VehiclePose {
        VehiclePose {
            position: self.position,
            rotation: self.rotation,
        }
    }
    pub fn new(pose: VehiclePose, flags: u32) -> Self {
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
    /// Starting area origin in world space.
    pub origin: [f64; 3],
    /// Day cycle active seconds.
    pub cycle_elapsed_seconds: f64,
    /// World stream session.
    pub session: u64,
    /// Required content selection revision.
    pub selection_revision: u64,
    /// Interpolation from `SimFrame::previous` to `SimFrame::current`.
    pub alpha: f32,
    /// Sleep clock factor on frame time.
    pub time_scale: f32,
    pub flags: u32,
    pub fov_degrees: f32,
    /// Items stored in the inventory slots, as `ItemId + 1`. 0 is an empty slot.
    pub stored_items: [u32; 4],
    /// Player body at the current pose; the renderer moves it with the camera interpolation.
    pub body_pose: PoseRow,
    /// `CharacterRoot` relative to the Player.
    pub body_root: PoseRow,
    /// Uniform scale of every joint relative to `CharacterRoot`.
    pub joint_scale: f32,
    /// Sky traffic anchor relative to the stream origin.
    pub sky_anchor: [f32; 3],
    /// Cigarette that owns the ember light, as `ItemId + 1`. 0 is none.
    pub ember_light: u32,
    /// Bumps when the audio motion history restarts.
    pub motion_revision: u32,
    /// Source `Cam2FarPlane` of the draw distance.
    pub far_plane: f32,
    /// Keeps the block free of implicit padding.
    pub pad: u32,
}

impl StateBlock {
    pub const STARTED: u32 = 1;
    pub const PAUSED: u32 = 1 << 1;
    pub const SEATED: u32 = 1 << 2;
    pub const SLEEPING: u32 = 1 << 3;
    pub const DEAD: u32 = 1 << 4;
    pub const RECOVERING: u32 = 1 << 5;
    /// The world stream failed.
    pub const WORLD_FAILED: u32 = 1 << 6;
}

/// Valve of one opening: 0 closed, 1 open.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ValveRow {
    pub opening: u32,
    pub valve: f32,
}

/// State of one condition owner. Frames carry changed owners; the first frame carries all.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConditionRow {
    pub id: u32,
    pub state: u32,
    pub state2: u32,
    pub state3: u32,
    pub repair_progress: f32,
    pub color: [f32; 4],
}

impl ConditionRow {
    pub fn new(id: ConditionId, s: &ConditionStartup) -> Self {
        Self {
            id: id.0,
            state: u32::from(s.state),
            state2: s.state2,
            state3: s.state3,
            repair_progress: s.repair_progress,
            color: s.color,
        }
    }
    pub fn startup(&self) -> ConditionStartup {
        ConditionStartup {
            state: self.state as u8,
            state2: self.state2,
            state3: self.state3,
            repair_progress: self.repair_progress,
            color: self.color,
        }
    }
}

/// Per-handle item state. Body rows stay zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ItemRow {
    /// Gib shrink scale; 1 for other items.
    pub scale: f32,
    /// Food bite index.
    pub bite: u32,
}

/// One cigarette: part blend and ember output.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct CigaretteRow {
    pub item: u32,
    pub blend: f32,
    pub color: [f32; 4],
    pub light_intensity: f32,
}

/// One sky traffic plane relative to the sky anchor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PlaneRow {
    pub id: u32,
    pub prefab: u32,
    pub offset: [f32; 3],
    pub heading: [f32; 4],
}

/// One live particle. Positions are relative to the stream origin (contrails: the sky anchor).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ParticleRow {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub rotation: f32,
    pub age: f32,
    pub lifetime: f32,
    pub uv_draw: f32,
}

/// One particle system and its rows in `SimFrame::particles`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ParticleGroupRow {
    /// `ParticleGroupRow::URINE` and the other kinds.
    pub kind: u32,
    /// Kind id: tool, wash, opening, storm, plane, item, effect or car system.
    pub a: u32,
    /// Car prototype.
    pub b: u32,
    /// Car chassis body index.
    pub c: u32,
    /// System index inside the owner.
    pub source: u32,
    pub start: u32,
    pub count: u32,
    /// Rows come from a fluid contact: no age or sheet.
    pub fluid: u32,
}

impl ParticleGroupRow {
    pub const URINE: u32 = 0;
    pub const BRUSH: u32 = 1;
    pub const SPRAY: u32 = 2;
    pub const WASH: u32 = 3;
    pub const OUTLET: u32 = 4;
    pub const SPRAY_BOTTLE: u32 = 5;
    pub const CAR: u32 = 6;
    pub const STORM: u32 = 7;
    pub const MELEE: u32 = 8;
    pub const SKY: u32 = 9;
    pub const EMBER: u32 = 10;
    pub const EXHALE: u32 = 11;
    pub const COBWEB: u32 = 12;
    /// Car systems in `a`.
    pub const CAR_EXHAUST: u32 = 0;
    /// Wheel effects: source is wheel * 8 + system (0 dust, 1.. grass).
    pub const CAR_WHEEL: u32 = 1;
    /// Skid bursts: `tiresmoke` (0) or `tiresand` (1).
    pub const CAR_SKID: u32 = 2;
}

/// The `(car system, source)` pairs of every particle system of a car prototype.
pub fn car_particle_sources(
    reference: &ReferenceManifest,
    prototype: usize,
) -> impl Iterator<Item = (u32, usize)> + '_ {
    let record = crate::starting_area::world_vehicle(reference, prototype);
    let exhaust = (0..record.exhaust.len()).map(|i| (ParticleGroupRow::CAR_EXHAUST, i));
    let wheels = record
        .wheel_effects
        .iter()
        .enumerate()
        .flat_map(|(wheel, w)| {
            let dust = w.dust.iter().map(|_| 0);
            let grass = 1..=w.grass.len();
            dust.chain(grass)
                .map(move |system| (ParticleGroupRow::CAR_WHEEL, wheel * 8 + system))
        });
    let skid = (0..2).map(|i| (ParticleGroupRow::CAR_SKID, i));
    exhaust.chain(wheels).chain(skid)
}

/// Day cycle and lamp outputs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameLighting {
    pub cycle: CycleOutput,
    pub road_lamp: LightOutput,
}

/// What the renderer needs to build a new handle's visual.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpawnData {
    /// The visual already exists in the reference scene.
    Bound,
    /// A spawned item prototype body.
    Spawned(SpawnedItem),
    /// A food item that is not world content.
    Food { prototype: u32 },
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
        data: SpawnData,
    },
    /// Skin joint nodes of the player body, in `SimFrame::joints` order.
    PlayerRig(Vec<NodeId>),
    /// The renderer drops the handle, then releases it.
    Retire(Handle),
    /// Home cable rope aims. Empty: no pole in range, the root stays hidden.
    HomeCable(Vec<RopeAim>),
    /// The world car the player last sat in, else the start car: id and prototype.
    FollowedCar(Option<(GeneratedId, u32)>),
    Failure(String),
}

/// Local rotation and z scale of one home cable rope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RopeAim {
    pub node: NodeId,
    pub rotation: [f32; 4],
    pub scale_z: f32,
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
    pub items: Vec<ItemRow>,
    pub valves: Vec<ValveRow>,
    /// Player body joints relative to `CharacterRoot`.
    pub joints: Vec<PoseRow>,
    pub cigarettes: Vec<CigaretteRow>,
    pub particle_groups: Vec<ParticleGroupRow>,
    pub particles: Vec<ParticleRow>,
    pub planes: Vec<PlaneRow>,
    pub conditions: Vec<ConditionRow>,
    /// 1 when the wash shows, in `ConditionRecord::washes` order.
    pub washes: Vec<u8>,
    /// Brush model poses, in `ConditionRecord::brushes` order.
    pub brush_poses: Vec<PoseRow>,
    /// Spray colors, in `ConditionRecord::sprays` order.
    pub spray_colors: Vec<[f32; 4]>,
    pub lighting: FrameLighting,
    /// Lights in `LightingRecord::lights` order.
    pub lights: Vec<LightOutput>,
    pub lamp_materials: Vec<LampMaterialOutput>,
    pub control_poses: Vec<ControlPoseOutput>,
    pub node_visibility: Vec<NodeVisibilityOutput>,
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
    put_rows(out, &frame.previous);
    put_rows(out, &frame.current);
    put_rows(out, &frame.items);
    put_rows(out, &frame.valves);
    put_rows(out, &frame.joints);
    put_rows(out, &frame.cigarettes);
    put_rows(out, &frame.particle_groups);
    put_rows(out, &frame.particles);
    put_rows(out, &frame.planes);
    put_rows(out, &frame.conditions);
    put_rows(out, &frame.washes);
    put_rows(out, &frame.brush_poses);
    put_rows(out, &frame.spray_colors);
    put(out, &frame.lighting);
    put_list(out, &frame.lights);
    put_list(out, &frame.lamp_materials);
    put_list(out, &frame.control_poses);
    put_list(out, &frame.node_visibility);
    *out = postcard::to_extend(&frame.changes, std::mem::take(out)).expect("frame changes encode");
}

/// Reads `bytes` into `frame`, reusing its arrays.
pub fn decode(bytes: &[u8], frame: &mut SimFrame) -> Result<(), DecodeError> {
    let mut rest = bytes;
    frame.seq_consumed = u64::from_le_bytes(take(&mut rest, 8)?.try_into().expect("8 bytes"));
    frame.state = bytemuck::pod_read_unaligned(take(&mut rest, size_of::<StateBlock>())?);
    take_rows(&mut rest, &mut frame.previous)?;
    take_rows(&mut rest, &mut frame.current)?;
    take_rows(&mut rest, &mut frame.items)?;
    take_rows(&mut rest, &mut frame.valves)?;
    take_rows(&mut rest, &mut frame.joints)?;
    take_rows(&mut rest, &mut frame.cigarettes)?;
    take_rows(&mut rest, &mut frame.particle_groups)?;
    take_rows(&mut rest, &mut frame.particles)?;
    take_rows(&mut rest, &mut frame.planes)?;
    take_rows(&mut rest, &mut frame.conditions)?;
    take_rows(&mut rest, &mut frame.washes)?;
    take_rows(&mut rest, &mut frame.brush_poses)?;
    take_rows(&mut rest, &mut frame.spray_colors)?;
    frame.lighting = take_value(&mut rest)?;
    take_list(&mut rest, &mut frame.lights)?;
    take_list(&mut rest, &mut frame.lamp_materials)?;
    take_list(&mut rest, &mut frame.control_poses)?;
    take_list(&mut rest, &mut frame.node_visibility)?;
    frame.changes = postcard::from_bytes(rest).map_err(|e| DecodeError(e.to_string()))?;
    Ok(())
}

fn put_rows<T: Pod>(out: &mut Vec<u8>, rows: &[T]) {
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(bytemuck::cast_slice(rows));
}

fn put<T: Serialize>(out: &mut Vec<u8>, value: &T) {
    *out = postcard::to_extend(value, std::mem::take(out)).expect("frame value encode");
}

/// Serde rows, one postcard value each. Decode reuses the vector.
fn put_list<T: Serialize>(out: &mut Vec<u8>, rows: &[T]) {
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in rows {
        put(out, row);
    }
}

fn take_value<T: DeserializeOwned>(rest: &mut &[u8]) -> Result<T, DecodeError> {
    let (value, tail) = postcard::take_from_bytes(rest).map_err(|e| DecodeError(e.to_string()))?;
    *rest = tail;
    Ok(value)
}

fn take_list<T: DeserializeOwned>(rest: &mut &[u8], rows: &mut Vec<T>) -> Result<(), DecodeError> {
    let n = u32::from_le_bytes(take(rest, 4)?.try_into().expect("4 bytes")) as usize;
    rows.clear();
    for _ in 0..n {
        rows.push(take_value(rest)?);
    }
    Ok(())
}

fn take<'a>(rest: &mut &'a [u8], n: usize) -> Result<&'a [u8], DecodeError> {
    if rest.len() < n {
        return Err(DecodeError("short frame".into()));
    }
    let (head, tail) = rest.split_at(n);
    *rest = tail;
    Ok(head)
}

fn take_rows<T: Pod>(rest: &mut &[u8], rows: &mut Vec<T>) -> Result<(), DecodeError> {
    let n = u32::from_le_bytes(take(rest, 4)?.try_into().expect("4 bytes")) as usize;
    let bytes = take(rest, n * size_of::<T>())?;
    rows.clear();
    rows.extend(
        bytes
            .chunks_exact(size_of::<T>())
            .map(bytemuck::pod_read_unaligned::<T>),
    );
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
    /// Joint count of the last `PlayerRig` change.
    rig_joints: Option<usize>,
}

impl FramePoses {
    /// Writes the area poses into `frame` and adds `Spawn` and `Retire` changes.
    pub fn fill(
        &mut self,
        reference: &ReferenceManifest,
        area: &StartingArea,
        physics: &PhysicsWorld,
        frame: &mut SimFrame,
    ) {
        self.stamp = self.stamp.wrapping_add(1);
        let rows = self.seen.len();
        frame.previous.resize(rows, PoseRow::default());
        frame.current.resize(rows, PoseRow::default());
        frame.items.resize(rows, ItemRow::default());
        let stored = area.inventory.slots.map(|id| {
            id.and_then(|id| area.items.get(id))
                .filter(|i| matches!(i.location, ItemLocation::Stored(_)))
                .and_then(|i| i.body)
        });
        for (id, previous, current) in &area.render_poses {
            let body = area.bodies[id];
            let stored = stored.contains(&Some(body));
            let active = physics.body_active(body);
            let mut flags = 0;
            if active || stored {
                flags |= PoseRow::VISIBLE;
            }
            if active {
                flags |= PoseRow::ACTIVE;
            }
            if stored {
                flags |= PoseRow::STORED;
            }
            let slot = self.write(PoseKey::Body(*id), || SpawnData::Bound, frame);
            frame.previous[slot] = PoseRow::new(*previous, flags);
            frame.current[slot] = PoseRow::new(*current, flags);
        }
        let starter = reference.survival.starter_food;
        for (id, previous, current) in &area.item_render_poses {
            let item = area.items.get(*id);
            let mut flags = 0;
            if item.is_some_and(|item| item.inactive_pose.is_none()) {
                flags |= PoseRow::VISIBLE | PoseRow::ACTIVE;
            }
            if item.is_some_and(|item| matches!(item.location, ItemLocation::Stored(_))) {
                flags |= PoseRow::STORED;
            }
            let data = || {
                if let Some(spawned) = area.spawned.get(*id) {
                    SpawnData::Spawned(*spawned)
                } else if let Some(food) = item.and_then(|i| i.food)
                    && (*id == starter || !area.items.is_generated(*id))
                {
                    SpawnData::Food {
                        prototype: food.prototype_index as u32,
                    }
                } else {
                    SpawnData::Bound
                }
            };
            let slot = self.write(PoseKey::Item(*id), data, frame);
            frame.previous[slot] = PoseRow::new(*previous, flags);
            frame.current[slot] = PoseRow::new(*current, flags);
            frame.items[slot] = ItemRow {
                scale: area.gibs.scale(*id).unwrap_or(1.),
                bite: item.and_then(|i| i.food).map_or(0, |f| u32::from(f.bite)),
            };
        }
        for (slot, seen) in self.seen.iter().enumerate() {
            let handle = Handle(slot as u32);
            if *seen != self.stamp && self.handles.key(handle).is_some() {
                self.handles.retire(handle);
                frame.previous[slot] = PoseRow::default();
                frame.current[slot] = PoseRow::default();
                frame.items[slot] = ItemRow::default();
                frame.changes.push(Change::Retire(handle));
            }
        }
        let body = &area.body;
        if self.rig_joints != Some(body.joint_count()) {
            self.rig_joints = Some(body.joint_count());
            frame.changes.push(Change::PlayerRig(
                (0..body.joint_count())
                    .map(|i| body.joint_node(i))
                    .collect(),
            ));
        }
        frame.joints.clear();
        frame
            .joints
            .extend((0..body.joint_count()).map(|i| PoseRow::new(body.joint_local(i), 0)));
    }
    /// The renderer dropped `handle`.
    pub fn release(&mut self, handle: Handle) {
        self.handles.release(handle);
    }
    /// The row of `key`, with a `Spawn` change when the key is new.
    fn write(
        &mut self,
        key: PoseKey,
        data: impl FnOnce() -> SpawnData,
        frame: &mut SimFrame,
    ) -> usize {
        let (handle, new) = self.handles.assign(key);
        let slot = handle.0 as usize;
        if slot >= self.seen.len() {
            self.seen.resize(slot + 1, 0);
            frame.previous.resize(slot + 1, PoseRow::default());
            frame.current.resize(slot + 1, PoseRow::default());
            frame.items.resize(slot + 1, ItemRow::default());
        }
        if new {
            frame.changes.push(Change::Spawn {
                handle,
                key,
                data: data(),
            });
        }
        self.seen[slot] = self.stamp;
        slot
    }
}

#[cfg(test)]
mod tests;
