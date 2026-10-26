//! GPU-driven static props. A static renderer is a row in GPU buffers, not an entity.
//! The main world owns the rows. The render world uploads the dirty ranges.
use crate::reference::shadow_candidates::Bounds;
use bevy::asset::{AssetId, UntypedAssetId};
use bevy::ecs as bevy_ecs;
use bevy::math::{Affine3A, DMat4, Mat4};
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::Resource;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

/// Words of one row in the GPU row buffer. Same layout as `PropRow` in source_props.wgsl.
pub const ROW_WORDS: usize = 20;
/// The row slot is free.
pub const NO_BATCH: u32 = u32::MAX;
/// Row flag bits after the 5 layer bits.
pub const ROW_HIDDEN: u32 = 1 << 5;
pub const ROW_SHADOW_ONLY: u32 = 1 << 6;
/// More dirty ranges than this merge into one range.
const DIRTY_RANGES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropRow {
    /// World matrix in the render-origin frame.
    pub world: Mat4,
    /// World AABB in the source frame, as the shadow scene uses it.
    pub bounds: Bounds,
    pub layer: u8,
    /// Source shadow casting mode. 3 is shadows only.
    pub cast_shadows: u8,
    pub hidden: bool,
    /// The material has a shadow caster pass.
    pub caster: bool,
    pub batch: u32,
}
impl PropRow {
    pub fn words(&self) -> [u32; ROW_WORDS] {
        let m = self.world;
        let rows = [m.row(0), m.row(1), m.row(2)];
        let mut flags = u32::from(self.layer);
        if self.hidden {
            flags |= ROW_HIDDEN;
        }
        if self.cast_shadows == 3 {
            flags |= ROW_SHADOW_ONLY;
        }
        let f = f32::to_bits;
        let [c, e] = [self.bounds.center, self.bounds.extents];
        [
            f(rows[0].x),
            f(rows[0].y),
            f(rows[0].z),
            f(rows[0].w),
            f(rows[1].x),
            f(rows[1].y),
            f(rows[1].z),
            f(rows[1].w),
            f(rows[2].x),
            f(rows[2].y),
            f(rows[2].z),
            f(rows[2].w),
            f(c[0]),
            f(c[1]),
            f(c[2]),
            flags,
            f(e[0]),
            f(e[1]),
            f(e[2]),
            self.batch,
        ]
    }
}
/// World matrix of a static root renderer. Entities used the root pose times the baked matrix.
pub fn row_world(root: DMat4, chain: Mat4) -> Mat4 {
    (root * chain.as_dmat4()).as_mat4()
}
/// World AABB in the source frame. Entity and row casters use this one formula.
pub fn world_bounds(world: Affine3A, center: [f32; 3], half_extents: [f32; 3]) -> Bounds {
    let center = world.transform_point3(center.into());
    let e = world.matrix3.x_axis.abs() * half_extents[0]
        + world.matrix3.y_axis.abs() * half_extents[1]
        + world.matrix3.z_axis.abs() * half_extents[2];
    Bounds {
        center: [center.x, center.y, -center.z],
        extents: e.to_array(),
    }
}
/// Draws share mesh and material. Layer and shadow mode are row data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BatchKey {
    pub mesh: AssetId<Mesh>,
    pub material: UntypedAssetId,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropBatch {
    pub key: BatchKey,
    /// Live rows. Zero marks a free batch slot.
    pub rows: u32,
    /// Rows that are not hidden.
    pub shown: u32,
    /// First entry of the batch in each visible list.
    pub first: u32,
    pub capacity: u32,
}
static OWNERS: AtomicU64 = AtomicU64::new(1);

#[derive(Resource)]
pub struct GpuProps {
    /// Unique per owner. A save exchange replaces the owner, so the render world uploads all rows.
    id: u64,
    rows: Vec<PropRow>,
    free: Vec<u32>,
    live: u32,
    dirty: Vec<Range<u32>>,
    batches: Vec<PropBatch>,
    free_batches: Vec<u32>,
    index: HashMap<BatchKey, u32>,
    /// Sum of batch capacities: the length of one visible list.
    visible_len: u32,
    /// Changes when rows are added or removed.
    membership: u64,
    /// Changes on any row change.
    revision: u64,
    /// Changes on any batch table change.
    batch_revision: u64,
}
impl Default for GpuProps {
    fn default() -> Self {
        Self {
            id: OWNERS.fetch_add(1, Ordering::Relaxed),
            rows: Vec::new(),
            free: Vec::new(),
            live: 0,
            dirty: Vec::new(),
            batches: Vec::new(),
            free_batches: Vec::new(),
            index: HashMap::default(),
            visible_len: 0,
            membership: 0,
            revision: 0,
            batch_revision: 0,
        }
    }
}
impl GpuProps {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn rows(&self) -> &[PropRow] {
        &self.rows
    }
    pub fn live(&self) -> u32 {
        self.live
    }
    pub fn batches(&self) -> &[PropBatch] {
        &self.batches
    }
    pub fn visible_len(&self) -> u32 {
        self.visible_len
    }
    pub fn membership(&self) -> u64 {
        self.membership
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn batch_revision(&self) -> u64 {
        self.batch_revision
    }
    pub fn retained_bytes(&self) -> usize {
        self.rows.capacity() * std::mem::size_of::<PropRow>()
            + self.free.capacity() * 4
            + self.batches.capacity() * std::mem::size_of::<PropBatch>()
            + self.free_batches.capacity() * 4
            + self.index.capacity() * (std::mem::size_of::<(BatchKey, u32)>() + 1)
    }
    /// Moves the dirty ranges to `out`. The render world uploads them.
    pub fn take_dirty(&mut self, out: &mut Vec<Range<u32>>) {
        out.append(&mut self.dirty);
    }
    fn mark(&mut self, slot: u32) {
        self.revision += 1;
        if let Some(last) = self.dirty.last_mut() {
            if last.contains(&slot) {
                return;
            }
            if last.end == slot {
                last.end += 1;
                return;
            }
        }
        if self.dirty.len() == DIRTY_RANGES {
            let start = self.dirty.iter().map(|r| r.start).min().unwrap().min(slot);
            let end = self.dirty.iter().map(|r| r.end).max().unwrap().max(slot + 1);
            self.dirty.clear();
            self.dirty.push(start..end);
            return;
        }
        self.dirty.push(slot..slot + 1);
    }
    /// Adds a row. The batch of `row` is replaced.
    pub fn alloc(&mut self, key: BatchKey, mut row: PropRow) -> u32 {
        let batch = match self.index.get(&key) {
            Some(&batch) => batch,
            None => {
                let batch = self.free_batches.pop().unwrap_or_else(|| {
                    self.batches.push(PropBatch {
                        key,
                        rows: 0,
                        shown: 0,
                        first: 0,
                        capacity: 0,
                    });
                    self.batches.len() as u32 - 1
                });
                self.batches[batch as usize].key = key;
                self.index.insert(key, batch);
                batch
            }
        };
        let entry = &mut self.batches[batch as usize];
        entry.rows += 1;
        entry.shown += u32::from(!row.hidden);
        if entry.rows > entry.capacity {
            self.layout();
        }
        self.batch_revision += 1;
        row.batch = batch;
        let slot = self.free.pop().unwrap_or_else(|| {
            self.rows.push(row);
            self.rows.len() as u32 - 1
        });
        self.rows[slot as usize] = row;
        self.live += 1;
        self.membership += 1;
        self.mark(slot);
        slot
    }
    pub fn free(&mut self, slot: u32) {
        let row = &mut self.rows[slot as usize];
        let batch = std::mem::replace(&mut row.batch, NO_BATCH);
        assert_ne!(batch, NO_BATCH, "GPU prop row is already free");
        let entry = &mut self.batches[batch as usize];
        entry.rows -= 1;
        entry.shown -= u32::from(!row.hidden);
        if entry.rows == 0 {
            self.index.remove(&entry.key);
            self.free_batches.push(batch);
        }
        self.batch_revision += 1;
        self.free.push(slot);
        self.live -= 1;
        self.membership += 1;
        self.mark(slot);
        // Free capacity stays until it is a large part of the visible list.
        if self.visible_len > 1024 && self.visible_len > 4 * self.live {
            self.layout();
        }
    }
    pub fn set_hidden(&mut self, slot: u32, hidden: bool) {
        let row = &mut self.rows[slot as usize];
        if row.hidden == hidden {
            return;
        }
        row.hidden = hidden;
        let entry = &mut self.batches[row.batch as usize];
        if hidden {
            entry.shown -= 1;
        } else {
            entry.shown += 1;
        }
        self.batch_revision += 1;
        self.mark(slot);
    }
    pub fn set_world(&mut self, slot: u32, world: Mat4, bounds: Bounds) {
        let row = &mut self.rows[slot as usize];
        if row.world == world && row.bounds == bounds {
            return;
        }
        row.world = world;
        row.bounds = bounds;
        self.mark(slot);
    }
    /// Gives each batch a power-of-two region in the visible lists.
    fn layout(&mut self) {
        let mut first = 0;
        for batch in &mut self.batches {
            batch.capacity = if batch.rows == 0 {
                0
            } else {
                batch.rows.next_power_of_two()
            };
            batch.first = first;
            first += batch.capacity;
        }
        self.visible_len = first;
    }
    /// Removes every row. Retained storage stays.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.free.clear();
        self.batches.clear();
        self.free_batches.clear();
        self.index.clear();
        self.dirty.clear();
        self.live = 0;
        self.visible_len = 0;
        self.membership += 1;
        self.revision += 1;
        self.batch_revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    fn key(mesh: u128) -> BatchKey {
        BatchKey {
            mesh: AssetId::Uuid {
                uuid: bevy::asset::uuid::Uuid::from_u128(mesh),
            },
            material: AssetId::<Mesh>::Uuid {
                uuid: bevy::asset::uuid::Uuid::from_u128(7),
            }
            .untyped(),
        }
    }
    fn row(hidden: bool) -> PropRow {
        PropRow {
            world: Mat4::IDENTITY,
            bounds: Bounds {
                center: [0.; 3],
                extents: [1.; 3],
            },
            layer: 3,
            cast_shadows: 1,
            hidden,
            caster: true,
            batch: 0,
        }
    }
    fn dirty(props: &mut GpuProps) -> Vec<Range<u32>> {
        let mut out = Vec::new();
        props.take_dirty(&mut out);
        out
    }

    #[test]
    fn slots_reuse_the_free_list_and_mark_dirty_ranges() {
        let mut props = GpuProps::default();
        let a = props.alloc(key(1), row(false));
        let b = props.alloc(key(1), row(true));
        let c = props.alloc(key(2), row(false));
        assert_eq!((a, b, c), (0, 1, 2));
        assert_eq!(dirty(&mut props), [0..3]);
        assert_eq!(props.live(), 3);
        assert_eq!(props.batches()[0].rows, 2);
        assert_eq!(props.batches()[0].shown, 1);
        assert_eq!(props.rows()[c as usize].batch, 1);

        props.free(b);
        props.free(a);
        assert_eq!(dirty(&mut props), [1..2, 0..1]);
        assert_eq!(props.rows()[a as usize].batch, NO_BATCH);
        // The emptied batch slot and the last freed row slot are reused first.
        let d = props.alloc(key(3), row(false));
        assert_eq!(d, a);
        assert_eq!(props.rows()[d as usize].batch, 0);
        assert_eq!(props.batches()[0].key, key(3));
        let e = props.alloc(key(1), row(false));
        assert_eq!(e, b);
        assert_eq!(props.rows()[e as usize].batch, 2);
        assert_eq!(dirty(&mut props), [0..2]);

        props.set_hidden(c, true);
        props.set_hidden(c, true);
        props.set_world(e, Mat4::from_translation(Vec3::X), props.rows()[e as usize].bounds);
        assert_eq!(dirty(&mut props), [2..3, 1..2]);
        assert_eq!(props.batches()[1].shown, 0);
        assert_eq!(props.live(), 3);
    }

    #[test]
    fn batch_regions_do_not_overlap() {
        let mut props = GpuProps::default();
        let mut slots = Vec::new();
        for i in 0..40u32 {
            slots.push(props.alloc(key(u128::from(i % 3)), row(false)));
        }
        let check = |props: &GpuProps| {
            let mut end = 0;
            for batch in props.batches() {
                assert!(batch.rows <= batch.capacity);
                assert_eq!(batch.first, end);
                end += batch.capacity;
            }
            assert_eq!(end, props.visible_len());
        };
        check(&props);
        for slot in slots.drain(..30) {
            props.free(slot);
        }
        check(&props);
        assert_eq!(props.live(), 10);
        // Many single ranges merge into one range.
        let mut props = GpuProps::default();
        for i in 0..200 {
            props.alloc(key(1), row(false));
            if i % 2 == 0 {
                props.alloc(key(2), row(false));
            }
        }
        assert_eq!(dirty(&mut props), [0..300]);
        for slot in (0..300).step_by(2) {
            props.set_hidden(slot, true);
        }
        let out = dirty(&mut props);
        assert!(out.len() <= DIRTY_RANGES);
        for slot in (0..300).step_by(2) {
            assert!(out.iter().any(|r| r.contains(&slot)), "slot {slot}");
        }
    }

    #[test]
    fn row_worlds_match_flat_entity_worlds() {
        use crate::node_plan_tests::NodePlan;
        use bevy::math::{Quat, Vec3};
        use bevy::transform::components::{GlobalTransform, Transform};
        // 0 root, 1 body, 2 static leaf under the body, 3 static leaf, 4 static parent, 5 its leaf,
        // 6 non-uniform parent, 7 its leaf.
        let children: [&[u32]; 8] = [&[1, 3, 4, 6], &[2], &[], &[], &[5], &[], &[7], &[]];
        let mut dynamic = vec![false; 8];
        dynamic[1] = true;
        let plan = NodePlan::new(&children, vec![None; 8], dynamic);
        let trs = |s: [f32; 3], r: Quat, t: [f32; 3]| {
            Mat4::from_scale_rotation_translation(Vec3::from_array(s), r, Vec3::from_array(t))
        };
        let locals = [
            trs([2.0; 3], Quat::from_rotation_y(0.3), [1.0, 2.0, 3.0]),
            trs([1.0; 3], Quat::from_rotation_x(0.7), [0.0, 1.0, 0.0]),
            trs([0.5; 3], Quat::from_rotation_z(-0.4), [0.2, 0.0, 0.1]),
            trs([4.0, 1.0, 2.0], Quat::from_rotation_x(-0.9), [0.0, 0.0, 2.0]),
            trs([1.5; 3], Quat::from_rotation_y(0.5), [0.1, 0.0, 0.0]),
            trs([1.0, 2.0, 1.0], Quat::from_rotation_z(0.8), [0.0, 0.2, 0.0]),
            trs([1.0, 2.0, 3.0], Quat::from_rotation_x(0.4), [3.0, 0.0, 0.0]),
            trs([1.0; 3], Quat::from_rotation_y(0.6), [0.0, 1.0, 0.0]),
        ];
        let flat = plan.flatten(&locals);
        let rows = (0..8).filter(|&n| flat.on_root(n)).collect::<Vec<_>>();
        assert_eq!(rows, [0, 3, 4, 5]);
        // A distant origin keeps the root translation in double precision.
        let root = trs([1.0, 1.5, 0.8], Quat::from_rotation_y(-0.2), [0.0; 3]).as_dmat4()
            * DMat4::from_translation(bevy::math::DVec3::new(10.0, 0.0, -5.0));
        for node in rows {
            // The P2 renderer entity: a root child with the baked matrix.
            let entity = GlobalTransform::from(Transform::from_matrix(root.as_mat4()))
                * Transform::from_matrix(flat.rest[node]);
            let row = row_world(root, flat.chain[node]);
            assert!(row.abs_diff_eq(entity.to_matrix(), 1e-4), "node {node}");
        }
    }
}
