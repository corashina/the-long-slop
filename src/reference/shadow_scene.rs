//! Ordered caster data and revision-based extraction, independent of camera visibility.
use super::shadow_frame::{CasterInput, validate_casters};
use std::{collections::HashMap, hash::Hash};

/// A caster is a renderer entity or a GPU prop row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CasterKey {
    Entity(bevy::ecs::entity::Entity),
    Row(u32),
}

pub struct CasterData<K> {
    pub entities: Vec<K>,
    pub casters: Vec<CasterInput>,
    revision: u64,
    changed: bool,
}
impl<K> Default for CasterData<K> {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            casters: Vec::new(),
            revision: 0,
            changed: false,
        }
    }
}
impl<K: Copy> CasterData<K> {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn reserve(&mut self, count: usize) {
        self.entities
            .reserve_exact(count.saturating_sub(self.entities.len()));
        self.casters
            .reserve_exact(count.saturating_sub(self.casters.len()));
    }
    /// Publish only complete, validated collections. Rebuilds always change the revision.
    pub fn publish(&mut self) -> Result<(), &'static str> {
        if self.changed {
            validate_casters(&self.casters)?;
            self.revision = self
                .revision
                .checked_add(1)
                .expect("Source shadow caster revision exhausted");
            self.changed = false;
        }
        Ok(())
    }
    /// The render world owns this copy. Unchanged revisions do not touch its arrays.
    pub fn extract_from(&mut self, input: &Self) -> bool {
        if self.revision == input.revision {
            return false;
        }
        self.entities.clear();
        self.entities.extend_from_slice(&input.entities);
        self.casters.clear();
        self.casters.extend_from_slice(&input.casters);
        self.revision = input.revision;
        self.changed = false;
        true
    }
}

/// Only the main world owns the entity-to-row index.
pub struct CasterIndex<K>(HashMap<K, usize>);
impl<K> Default for CasterIndex<K> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
impl<K: Copy + Eq + Hash> CasterIndex<K> {
    pub fn contains(&self, entity: K) -> bool {
        self.0.contains_key(&entity)
    }
    pub fn reserve(&mut self, count: usize) {
        self.0.reserve(count.saturating_sub(self.0.len()));
    }
    /// Membership changes rebuild in query order. Retained allocations survive scene replacement.
    pub fn clear(&mut self, data: &mut CasterData<K>) {
        self.0.clear();
        data.entities.clear();
        data.casters.clear();
        data.changed = true;
    }
    pub fn update(&mut self, data: &mut CasterData<K>, entity: K, caster: CasterInput) {
        if let Some(&row) = self.0.get(&entity) {
            if !data.casters[row].unchanged(&caster) {
                data.casters[row] = caster;
                data.changed = true;
            }
        } else {
            self.0.insert(entity, data.entities.len());
            data.entities.push(entity);
            data.casters.push(caster);
            data.changed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::shadow_candidates::{Bounds, CandidateState};
    use super::super::shadow_frame::{CasterInput, prepare_frame};
    use super::super::shadow_planes::implicit_cull_planes;
    use super::super::shadows::ShadowFitInput;
    use super::*;

    #[test]
    fn row_casters_give_entity_masks() {
        let projection = [
            1.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            0.,
            0.,
            0.,
            -11. / 9.,
            -1.,
            0.,
            0.,
            -20. / 9.,
            0.,
        ];
        let identity = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        let planes = implicit_cull_planes(projection, identity, 1., 10., 10., 0.);
        let make = |center, disabled| CasterInput {
            bounds: Bounds {
                center,
                extents: [0.5; 3],
            },
            state: CandidateState {
                shadow_casting_mode: 1,
                layer: 0,
                renderer_present: true,
                disabled,
                force_off: false,
                lod_group: 0,
                lod_index_mask: 0,
                active_lod: 0,
            },
            material_has_caster: true,
        };
        let inputs = [
            make([0., 0., -2.], false),
            make([0.5, 0., -6.], false),
            make([20., 0., -5.], false),
            make([0., 0., -3.], true),
        ];
        let mut data = CasterData::<CasterKey>::default();
        let mut index = CasterIndex::default();
        // Each input enters once as an entity and once as a row, interleaved.
        for (i, input) in inputs.iter().enumerate() {
            index.update(
                &mut data,
                CasterKey::Entity(bevy::ecs::entity::Entity::from_raw_u32(i as u32 + 1).unwrap()),
                *input,
            );
            index.update(&mut data, CasterKey::Row(i as u32), *input);
        }
        data.publish().unwrap();
        let mut masks = vec![0; data.casters.len()];
        let mut fit = ShadowFitInput {
            frustum_points: [
                [-1., -1., -1.],
                [1., -1., -1.],
                [1., 1., -1.],
                [-1., 1., -1.],
                [-10., -10., -10.],
                [10., -10., -10.],
                [10., 10., -10.],
                [-10., 10., -10.],
            ],
            camera_basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            camera_position: [0.; 3],
            light_basis: [[0., 0., 1.], [0., 1., 0.], [-1., 0., 0.]],
            caster_min: [0.; 3],
            caster_max: [0.; 3],
            near: 1.,
            far: 10.,
            vertical_fov: 90.,
            shadow_distance: 10.,
            split_fractions: [0.1, 0.3, 0.6],
            tile_size: [64; 2],
            quality_near_offset: 0.,
            shadow_strength: 0.9,
        };
        prepare_frame(&data.casters, &mut masks, &planes, 1, 1, &mut fit)
            .unwrap()
            .unwrap();
        assert!(masks.iter().any(|m| *m != 0));
        for pair in masks.chunks(2) {
            assert_eq!(pair[0], pair[1]);
        }
        assert_eq!(masks[6], 0, "a disabled caster has no split");
    }
}
