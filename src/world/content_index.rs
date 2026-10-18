//! Per-frame lookup from generated identities to live content and lamp buttons.
use super::WorldStream;
use crate::reference::{SourceId, Transform};
use crate::world::{GeneratedCapVisual, GeneratedId, NodeAppearance, contents::ContentInstance};
use std::hash::{Hash, Hasher};

/// Sorted identity hashes. Rebuild it each frame; it keeps its capacity.
#[derive(Default)]
pub struct ContentIndex {
    contents: Vec<(u64, u32)>,
    lamps: Vec<(u64, u32)>,
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

impl WorldStream {
    pub fn index_contents(&self, index: &mut ContentIndex) {
        index.contents.clear();
        index.contents.extend(
            self.contents
                .active
                .iter()
                .chain(&self.static_controls)
                .enumerate()
                .map(|(i, c)| (identity_hash(&c.content.placement.id), i as u32)),
        );
        // Stable sort keeps the first equal identity first, as a linear search does.
        index.contents.sort_by_key(|(hash, _)| *hash);
        index.lamps.clear();
        index.lamps.extend(
            self.lamp_bindings
                .iter()
                .enumerate()
                .map(|(i, b)| (identity_hash(&b.parent), i as u32)),
        );
        index.lamps.sort_by_key(|(hash, _)| *hash);
    }
    /// Requires an index built from the current stream.
    pub fn indexed_content(&self, index: &ContentIndex, id: &GeneratedId) -> IndexedContent<'_> {
        let active = self.contents.active.len();
        let found = matches(&index.contents, identity_hash(id))
            .map(|i| {
                if i < active {
                    (&self.contents.active[i], true)
                } else {
                    (&self.static_controls[i - active], false)
                }
            })
            .find(|(c, _)| &c.content.placement.id == id);
        IndexedContent {
            content: found.map(|(c, _)| c),
            active: found.is_some_and(|(_, a)| a),
        }
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
        matches(&index.lamps, identity_hash(parent))
            .map(|i| &self.lamp_bindings[i])
            .filter(move |b| &b.parent == parent)
            .filter_map(|b| b.button.as_ref())
            .map(|b| (&b.node, b.pose))
    }
}
