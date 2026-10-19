//! Cached lookup from generated identities to live content and lamp buttons.
use super::WorldStream;
use crate::reference::{SourceId, Transform};
use crate::world::{GeneratedCapVisual, GeneratedId, NodeAppearance, contents::ContentInstance};
use std::hash::{Hash, Hasher};

/// Sorted identity hashes. Only membership and order are cached.
#[derive(Default)]
pub struct ContentIndex {
    contents: Vec<(u64, u32)>,
    lamps: Vec<(u64, u32)>,
    content_key: Option<(u64, u64, u64)>,
    lamp_key: Option<(u64, u64)>,
    #[cfg(test)]
    pub(super) rebuilds: [usize; 2],
}

fn identity_hash(id: &GeneratedId) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

fn matches(entries: &[(u64, u32)], hash: u64) -> impl Iterator<Item = usize> + '_ {
    let start = entries.partition_point(|(h, _)| *h < hash);
    entries[start..]
        .iter()
        .take_while(move |(h, _)| *h == hash)
        .map(|(_, i)| *i as usize)
}

/// Live content of one generated identity.
pub struct IndexedContent<'a> {
    content: Option<&'a ContentInstance>,
    active: bool,
}
impl<'a> IndexedContent<'a> {
    /// Same result as [`WorldStream::bindings_ready`].
    pub fn bindings_ready(&self) -> bool {
        self.content.is_none_or(|c| c.item.is_none() || c.restored)
    }
    /// Same result as [`WorldStream::generated_cap_visuals`].
    pub fn caps(&self) -> &'a [GeneratedCapVisual] {
        match self.content {
            Some(c) if self.active => &c.caps,
            _ => &[],
        }
    }
    /// Same result as [`WorldStream::generated_node_pose`].
    pub fn node_pose(&self, node: usize) -> Option<crate::vehicle::VehiclePose> {
        self.content.and_then(|c| c.poses.get(node)).copied()
    }
}

impl ContentIndex {
    fn update_contents(
        &mut self,
        key: (u64, u64, u64),
        active: &[ContentInstance],
        controls: &[ContentInstance],
    ) {
        if self.content_key != Some(key) {
            #[cfg(test)]
            {
                self.rebuilds[0] += 1;
            }
            self.contents.clear();
            self.contents.extend(
                active
                    .iter()
                    .chain(controls)
                    .enumerate()
                    .map(|(i, c)| (identity_hash(&c.content.placement.id), i as u32)),
            );
            // Original indices preserve first-match order, including hash collisions.
            self.contents.sort_unstable();
            self.content_key = Some(key);
        }
    }
    pub(super) fn update_lamps(
        &mut self,
        key: (u64, u64),
        lamps: &[super::content_lamps::LampBinding],
    ) {
        if self.lamp_key != Some(key) {
            #[cfg(test)]
            {
                self.rebuilds[1] += 1;
            }
            self.lamps.clear();
            self.lamps.extend(
                lamps
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (identity_hash(&b.parent), i as u32)),
            );
            self.lamps.sort_unstable();
            self.lamp_key = Some(key);
        }
    }
    fn content<'a>(
        &self,
        active: &'a [ContentInstance],
        controls: &'a [ContentInstance],
        id: &GeneratedId,
    ) -> IndexedContent<'a> {
        let found = matches(&self.contents, identity_hash(id))
            .map(|i| {
                if i < active.len() {
                    (&active[i], true)
                } else {
                    (&controls[i - active.len()], false)
                }
            })
            .find(|(c, _)| &c.content.placement.id == id);
        IndexedContent {
            content: found.map(|(c, _)| c),
            active: found.is_some_and(|(_, a)| a),
        }
    }
    pub(super) fn buttons<'a>(
        &'a self,
        lamps: &'a [super::content_lamps::LampBinding],
        parent: &'a GeneratedId,
    ) -> impl Iterator<Item = (&'a SourceId, Transform)> + 'a {
        matches(&self.lamps, identity_hash(parent))
            .map(|i| &lamps[i])
            .filter(move |b| &b.parent == parent)
            .filter_map(|b| b.button.as_ref())
            .map(|b| (&b.node, b.pose))
    }
}

impl WorldStream {
    pub fn index_contents(&self, index: &mut ContentIndex) {
        index.update_contents(
            (
                self.request.session,
                self.contents.active_revision(),
                self.static_control_revision,
            ),
            &self.contents.active,
            &self.static_controls,
        );
        index.update_lamps(
            (self.request.session, self.lamp_binding_revision),
            &self.lamp_bindings,
        );
    }
    /// Requires an index built from the current stream.
    pub fn indexed_content(&self, index: &ContentIndex, id: &GeneratedId) -> IndexedContent<'_> {
        index.content(&self.contents.active, &self.static_controls, id)
    }
    /// Same result as [`WorldStream::appearance`].
    pub fn indexed_appearance<'a>(
        &'a self,
        content: &IndexedContent<'a>,
        id: &GeneratedId,
    ) -> &'a [NodeAppearance] {
        match content.content {
            Some(c) if content.active => &c.content.appearance,
            _ => self.contents.cursor_appearance(id),
        }
    }
    /// Lamp button poses of one parent, as [`WorldStream::lamp_button_pose`] reads them.
    pub fn indexed_lamp_buttons<'a>(
        &'a self,
        index: &'a ContentIndex,
        parent: &'a GeneratedId,
    ) -> impl Iterator<Item = (&'a SourceId, Transform)> + 'a {
        index.buttons(&self.lamp_bindings, parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{CapVisualReference, ItemId, OpeningId};
    use crate::world::{AbsolutePose, Placement, WorldBounds, WorldLayer};

    #[test]
    fn hash_matches_keep_original_order() {
        let mut entries = [(7, 4), (2, 9), (7, 1), (9, 0), (7, 3)];
        entries.sort_unstable();
        assert_eq!(matches(&entries, 7).collect::<Vec<_>>(), [1, 3, 4]);
        assert_eq!(matches(&entries, 2).collect::<Vec<_>>(), [9]);
        assert_eq!(matches(&entries, 9).collect::<Vec<_>>(), [0]);
        assert_eq!(matches(&entries, 0).next(), None);
        assert_eq!(matches(&entries, 8).next(), None);
        assert_eq!(matches(&entries, 10).next(), None);
    }

    #[test]
    fn cache_tracks_membership_and_reads_live_content() {
        let source = SourceId {
            file: "test.assets".into(),
            path_id: 1,
        };
        let placement = |ordinal| Placement {
            id: GeneratedId {
                layer: WorldLayer::Building,
                generator: source.clone(),
                unit: [0; 2],
                ordinal,
                children: Vec::new(),
            },
            prototype: source.clone(),
            pose: AbsolutePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            },
            scale: [1.; 3],
            bounds: WorldBounds {
                min: [0.; 3],
                max: [0.; 3],
            },
            object_seed: 42,
        };
        let instance = |ordinal| ContentInstance::for_structure(placement(ordinal));
        let cap = || GeneratedCapVisual {
            opening: OpeningId(1),
            visual: CapVisualReference {
                open_nodes: Vec::new(),
                closed_nodes: Vec::new(),
                cap_node: None,
                closed_node: None,
                open_node: None,
                euler_rotation: false,
                closed_euler_degrees: [0.; 3],
                open_euler_degrees: [0.; 3],
            },
        };
        let mut active = vec![instance(0), instance(1), instance(1)];
        let mut controls = vec![instance(1), instance(2)];
        let first = placement(1).id;
        let static_id = placement(2).id;
        let missing = placement(9).id;
        let mut index = ContentIndex::default();
        index.update_contents((1, 0, 0), &active, &controls);
        assert_eq!(index.rebuilds, [1, 0]);
        assert!(std::ptr::eq(
            index.content(&active, &controls, &first).content.unwrap(),
            &active[1]
        ));
        assert!(std::ptr::eq(
            index
                .content(&active, &controls, &static_id)
                .content
                .unwrap(),
            &controls[1]
        ));
        assert!(
            index
                .content(&active, &controls, &missing)
                .content
                .is_none()
        );
        assert!(index.content(&active, &controls, &missing).bindings_ready());
        assert!(
            index
                .content(&active, &controls, &missing)
                .caps()
                .is_empty()
        );
        assert_eq!(
            index.content(&active, &controls, &missing).node_pose(0),
            None
        );

        let pose = crate::vehicle::VehiclePose {
            position: [3., 4., 5.],
            rotation: [0., 0., 0., 1.],
        };
        active[1].poses.push(pose);
        active[1].item = Some(ItemId(1));
        active[1].content.appearance.push(NodeAppearance {
            node: source.clone(),
            primary: None,
            scale: Some([2.; 3]),
            active: Some(false),
            material: None,
            color: None,
            color_material: None,
        });
        active[1].caps.push(cap());
        controls[1].caps.push(cap());
        controls[1].poses.push(pose);
        index.update_contents((1, 0, 0), &active, &controls);
        assert_eq!(index.rebuilds, [1, 0]);
        let found = index.content(&active, &controls, &first);
        assert_eq!(found.node_pose(0), Some(pose));
        assert!(!found.bindings_ready());
        assert_eq!(found.caps().len(), 1);
        assert!(std::ptr::eq(found.caps(), active[1].caps.as_slice()));
        assert_eq!(
            found.content.unwrap().content.appearance[0].scale,
            Some([2.; 3])
        );
        let found = index.content(&active, &controls, &static_id);
        assert_eq!(found.node_pose(0), Some(pose));
        assert!(found.caps().is_empty());
        assert!(!found.active);
        active[1].restored = true;
        active[1].content.appearance[0].scale = Some([3.; 3]);
        active[1].poses[0].position[0] = 7.;
        let found = index.content(&active, &controls, &first);
        assert!(found.bindings_ready());
        assert_eq!(found.node_pose(0).unwrap().position[0], 7.);
        assert_eq!(
            found.content.unwrap().content.appearance[0].scale,
            Some([3.; 3])
        );

        // Hash collisions must still compare each complete identity.
        active[0].content.placement.id = first.clone();
        active[0]
            .content
            .placement
            .id
            .children
            .push((source.clone(), 0));
        index.contents = vec![
            (identity_hash(&first), 0),
            (identity_hash(&first), 1),
            (identity_hash(&first), 2),
            (identity_hash(&first), 3),
        ];
        assert!(std::ptr::eq(
            index.content(&active, &controls, &first).content.unwrap(),
            &active[1]
        ));

        active.swap_remove(0);
        active.push(instance(3));
        index.update_contents((1, 1, 0), &active, &controls);
        assert_eq!(index.rebuilds, [2, 0]);
        assert!(
            index
                .content(&active, &controls, &placement(0).id)
                .content
                .is_none()
        );
        assert!(std::ptr::eq(
            index.content(&active, &controls, &first).content.unwrap(),
            &active[0]
        ));
        assert!(index.content(&active, &controls, &placement(3).id).active);
        active.swap_remove(2);
        index.update_contents((1, 2, 0), &active, &controls);
        assert_eq!(index.rebuilds, [3, 0]);
        assert!(std::ptr::eq(
            index
                .content(&active, &controls, &static_id)
                .content
                .unwrap(),
            &controls[1]
        ));

        controls.swap_remove(0);
        controls.push(instance(4));
        index.update_contents((1, 2, 1), &active, &controls);
        assert_eq!(index.rebuilds, [4, 0]);
        assert!(
            index
                .content(&active, &controls, &placement(4).id)
                .content
                .is_some()
        );
        active = vec![instance(5), instance(6)];
        index.update_contents((2, 2, 1), &active, &controls);
        assert_eq!(index.rebuilds, [5, 0]);
        assert!(index.content(&active, &controls, &first).content.is_none());
        assert!(
            index
                .content(&active, &controls, &placement(5).id)
                .content
                .is_some()
        );
        index.update_contents((2, 2, 1), &active, &controls);
        assert_eq!(index.rebuilds, [5, 0]);
    }
}
