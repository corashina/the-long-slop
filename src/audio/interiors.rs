//! Live interior list: garage interiors first, then world building interiors.
use crate::reference::{SourceId, WorldInteriorReference, WorldPrototypeReference};
use crate::vehicle::ColliderHandle;
use crate::world::{GeneratedId, StructureInstance};
use std::collections::HashMap;

const CAPACITY: usize = 256;

struct Entry {
    placement: GeneratedId,
    prototype: usize,
    interior: usize,
    /// List index.
    parent: Option<usize>,
}

/// World entries of the live interior list. List indexes start after the garage interiors.
pub struct WorldInteriors {
    offset: usize,
    entries: Vec<Entry>,
    /// Entries before the last rebuild, to remap latched sensor hits.
    previous: Vec<Entry>,
    handles: HashMap<ColliderHandle, usize>,
    revision: Option<u64>,
}

/// `playerscript.SetOutSideAudio` target gain of the outside mixers.
pub fn outside_target(default_gain: f32, use_value: bool, value: f32, doors: f32) -> f32 {
    default_gain + (1. - if use_value { value } else { default_gain }) * doors
}

impl WorldInteriors {
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            entries: Vec::with_capacity(CAPACITY),
            previous: Vec::with_capacity(CAPACITY),
            handles: HashMap::with_capacity(CAPACITY),
            revision: None,
        }
    }
    pub fn reset(&mut self) {
        self.revision = None;
    }
    /// Rebuilds the entries from the published structures when the stream revision changes.
    /// Returns true after a rebuild.
    pub fn sync(
        &mut self,
        revision: u64,
        structures: &[StructureInstance],
        prototypes: &[WorldPrototypeReference],
    ) -> bool {
        if self.revision == Some(revision) {
            return false;
        }
        self.revision = Some(revision);
        std::mem::swap(&mut self.entries, &mut self.previous);
        self.entries.clear();
        self.handles.clear();
        for s in structures {
            let Some(prototype) = prototypes
                .iter()
                .position(|p| !p.interiors.is_empty() && p.source == s.placement.prototype)
            else {
                continue;
            };
            let base = self.offset + self.entries.len();
            for (interior, r) in prototypes[prototype].interiors.iter().enumerate() {
                if let Some((_, handle)) = s.colliders.iter().find(|(c, _)| *c == r.collider) {
                    self.handles.insert(*handle, base + interior);
                }
                self.entries.push(Entry {
                    placement: s.placement.id.clone(),
                    prototype,
                    interior,
                    parent: r.parent.map(|p| base + p as usize),
                });
            }
        }
        true
    }
    /// Maps a list index from before the last rebuild to the current list.
    pub fn remap(&self, index: usize) -> Option<usize> {
        let Some(old) = index.checked_sub(self.offset) else {
            return Some(index);
        };
        let old = self.previous.get(old)?;
        self.entries
            .iter()
            .position(|e| e.interior == old.interior && e.placement == old.placement)
            .map(|i| self.offset + i)
    }
    /// The list index of a world interior collider.
    pub fn entry(&self, handle: ColliderHandle) -> Option<usize> {
        self.handles.get(&handle).copied()
    }
    /// The settings of a world list index.
    pub fn get<'a>(
        &self,
        index: usize,
        prototypes: &'a [WorldPrototypeReference],
    ) -> Option<&'a WorldInteriorReference> {
        let e = self.entries.get(index.checked_sub(self.offset)?)?;
        Some(&prototypes[e.prototype].interiors[e.interior])
    }
    pub fn parent(&self, index: usize) -> Option<usize> {
        self.entries.get(index.checked_sub(self.offset)?)?.parent
    }
    /// `interiorscript.GetValue` for a world list index. `door` returns a bound door value.
    pub fn value(
        &self,
        index: usize,
        prototypes: &[WorldPrototypeReference],
        door: &impl Fn(&GeneratedId, &WorldPrototypeReference, &SourceId) -> Option<f32>,
        caller: Option<usize>,
        depth: usize,
    ) -> f32 {
        let Some(e) = index
            .checked_sub(self.offset)
            .and_then(|i| self.entries.get(i))
        else {
            return 0.;
        };
        if depth > self.entries.len() {
            return 0.;
        }
        let prototype = &prototypes[e.prototype];
        let r = &prototype.interiors[e.interior];
        // A door that is not bound yet counts as open.
        let value = |d| door(&e.placement, prototype, d).unwrap_or(1.);
        let mut total = 0.;
        for (d, &invert) in r.doors.iter().zip(&r.invert_doors) {
            let v = value(d);
            total += (2. * if invert { 1. - v } else { v }).clamp(0., 1.);
        }
        if let Some(parent) = e.parent
            && Some(parent) != caller
        {
            let v = r.parent_door.as_ref().map_or(1., value);
            total += self.value(parent, prototypes, door, Some(index), depth + 1)
                * (2. * v).clamp(0., 1.);
        }
        super::interior_transmission(
            total,
            r.doors.len(),
            r.minimum_doors_to_max,
            r.maximum_transmission,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Placement, WorldBounds, WorldLayer};

    fn source(path_id: i64) -> SourceId {
        SourceId {
            file: "b".into(),
            path_id,
        }
    }
    fn interior(collider: i64, parent: Option<u32>, doors: &[i64]) -> WorldInteriorReference {
        WorldInteriorReference {
            collider: source(collider),
            parent,
            invert: false,
            minimum_doors_to_max: 1,
            maximum_transmission: 0.,
            use_value: false,
            value: 0.,
            doors: doors.iter().map(|&d| source(d)).collect(),
            invert_doors: vec![false; doors.len()],
            parent_door: parent.map(|_| source(50)),
            method: serde_json::from_str(
                r#"{"class":"c","method":"m","file":"f","line_start":1,"line_end":2,"sha256":""}"#,
            )
            .unwrap(),
        }
    }
    /// A hall with door 40 and a room inside it whose parent door is 50.
    fn prototypes() -> Vec<WorldPrototypeReference> {
        let mut p: WorldPrototypeReference = serde_json::from_str(
            r#"{"source":{"file":"b","path_id":9},"role":"structure","asset":1,"nodes":[],"controls":[],"excluded_actions":[],"deferred_materials":[],"skins":[],"bodies":[],"colliders":[],"prefab_pose":{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]}}"#,
        )
        .unwrap();
        p.interiors = vec![interior(1, None, &[40]), interior(2, Some(0), &[])];
        vec![p]
    }
    fn structure(ordinal: u32, first: u32) -> StructureInstance {
        StructureInstance {
            fixed_decoration: false,
            cpu_bytes: 0,
            placement: Placement {
                id: GeneratedId {
                    layer: WorldLayer::Building,
                    generator: source(1),
                    unit: [0, 0],
                    ordinal,
                    children: Vec::new(),
                },
                prototype: source(9),
                pose: crate::world::AbsolutePose {
                    position: [0.; 3],
                    rotation: [0., 0., 0., 1.],
                },
                scale: [1.; 3],
                bounds: WorldBounds {
                    min: [0.; 3],
                    max: [0.; 3],
                },
                object_seed: 0,
            },
            bodies: Vec::new(),
            colliders: vec![
                (source(1), ColliderHandle::from_raw_parts(first, 0)),
                (source(2), ColliderHandle::from_raw_parts(first + 1, 0)),
            ],
        }
    }
    /// Door values by source path id. A missing door is not bound.
    fn doors(
        values: &[(i64, f32)],
    ) -> impl Fn(&GeneratedId, &WorldPrototypeReference, &SourceId) -> Option<f32> {
        move |_, _, d| values.iter().find(|v| v.0 == d.path_id).map(|v| v.1)
    }

    #[test]
    fn closed_door_muffles_in_a_world_room() {
        let prototypes = prototypes();
        let mut list = WorldInteriors::new(3);
        list.sync(1, &[structure(1, 10)], &prototypes);
        let hall = list.entry(ColliderHandle::from_raw_parts(10, 0)).unwrap();
        assert_eq!(hall, 3, "world entries follow the garage entries");
        let closed = list.value(hall, &prototypes, &doors(&[(40, 0.)]), None, 0);
        let open = list.value(hall, &prototypes, &doors(&[(40, 1.)]), None, 0);
        assert_eq!((closed, open), (0., 1.));
        let target = |v| outside_target(0.3, false, 0., v);
        assert!(target(closed) < target(open) && target(open) <= 1.);
        assert_eq!(
            list.value(hall, &prototypes, &doors(&[]), None, 0),
            1.,
            "an unbound door counts as open"
        );
    }

    #[test]
    fn room_follows_the_parent_door_chain() {
        let prototypes = prototypes();
        let mut list = WorldInteriors::new(0);
        list.sync(1, &[structure(1, 10)], &prototypes);
        let room = list.entry(ColliderHandle::from_raw_parts(11, 0)).unwrap();
        assert_eq!(list.parent(room), Some(0));
        for (outer, inner, expected) in [(1., 1., 1.), (0., 1., 0.), (1., 0., 0.)] {
            let value = list.value(
                room,
                &prototypes,
                &doors(&[(40, outer), (50, inner)]),
                None,
                0,
            );
            assert_eq!(value, expected, "hall door {outer}, room door {inner}");
        }
    }

    #[test]
    fn entries_follow_published_structures() {
        let prototypes = prototypes();
        let mut list = WorldInteriors::new(1);
        let both = [structure(1, 10), structure(2, 20)];
        list.sync(1, &both, &prototypes);
        assert_eq!(list.entry(ColliderHandle::from_raw_parts(21, 0)), Some(4));
        assert_eq!(
            list.parent(4),
            Some(3),
            "parents stay inside their structure"
        );
        list.sync(1, &both[1..], &prototypes);
        assert_eq!(list.entries.len(), 4, "same revision keeps the entries");
        list.sync(2, &both[1..], &prototypes);
        assert_eq!(list.entry(ColliderHandle::from_raw_parts(10, 0)), None);
        assert_eq!(list.entry(ColliderHandle::from_raw_parts(21, 0)), Some(2));
        assert_eq!(
            [0, 1, 3, 4].map(|i| list.remap(i)),
            [Some(0), None, Some(1), Some(2)],
            "latched hits follow their room"
        );
        list.sync(3, &[], &prototypes);
        assert!(list.entries.is_empty() && list.handles.is_empty());
    }
}
