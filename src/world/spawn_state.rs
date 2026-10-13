//! Source caller boundaries survive unloading; generated content does not.
use super::{GeneratedId, UnityRandom, UnityRandomState};
use crate::reference::{ReferenceError, SourceId};
use serde::{Deserialize, Serialize};

/// Inputs that can change source selection, captured at each first invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnContext {
    pub current_car: i32,
    pub peaceful_enemies: i32,
    pub date: [i32; 3],
    pub chance_bits: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnBoundary {
    pub parent: GeneratedId,
    pub caller: SourceId,
    pub ordinal: u32,
    pub phase: SpawnPhase,
    pub random: UnityRandomState,
    pub context: SpawnContext,
    pub painting_pools: Vec<PaintingPool>,
    pub initialization: SpawnInitialization,
}
/// Mutable component inputs needed by a yielded slot continuation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnInitialization {
    pub paintings: Vec<(GeneratedId, SourceId, i32)>,
    pub scales: Vec<(GeneratedId, SourceId, [f32; 3])>,
    pub lamps: Vec<(GeneratedId, SourceId, bool)>,
    pub(crate) conditions: Vec<super::spawn_conditions::ConditionState>,
    pub initialized: Vec<(GeneratedId, SourceId)>,
    pub selectors: Vec<(GeneratedId, SourceId, usize)>,
    pub activations: Vec<(GeneratedId, SourceId, bool)>,
    pub slot_parts: Vec<(GeneratedId, SourceId, SourceId)>,
}
impl SpawnInitialization {
    pub(crate) fn for_parent(&self, parent: &GeneratedId) -> Self {
        Self {
            scales: self
                .scales
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            lamps: self
                .lamps
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            initialized: self
                .initialized
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            selectors: self
                .selectors
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            activations: self
                .activations
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            slot_parts: self
                .slot_parts
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            paintings: self
                .paintings
                .iter()
                .filter(|v| &v.0 == parent)
                .cloned()
                .collect(),
            conditions: self
                .conditions
                .iter()
                .filter(|v| &v.parent == parent)
                .cloned()
                .collect(),
        }
    }

    pub fn retained_bytes(&self) -> usize {
        self.paintings.capacity() * std::mem::size_of::<(GeneratedId, SourceId, i32)>()
            + self
                .paintings
                .iter()
                .map(|(p, s, _)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self.scales.capacity() * std::mem::size_of::<(GeneratedId, SourceId, [f32; 3])>()
            + self
                .scales
                .iter()
                .map(|(p, s, _)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self.lamps.capacity() * std::mem::size_of::<(GeneratedId, SourceId, bool)>()
            + self
                .lamps
                .iter()
                .map(|(p, s, _)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self.conditions.capacity()
                * std::mem::size_of::<super::spawn_conditions::ConditionState>()
            + self
                .conditions
                .iter()
                .map(|c| c.retained_heap())
                .sum::<usize>()
            + self.initialized.capacity() * std::mem::size_of::<(GeneratedId, SourceId)>()
            + self.selectors.capacity() * std::mem::size_of::<(GeneratedId, SourceId, usize)>()
            + self.activations.capacity() * std::mem::size_of::<(GeneratedId, SourceId, bool)>()
            + self.slot_parts.capacity() * std::mem::size_of::<(GeneratedId, SourceId, SourceId)>()
            + self
                .initialized
                .iter()
                .map(|(p, s)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self
                .selectors
                .iter()
                .map(|(p, s, _)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self
                .activations
                .iter()
                .map(|(p, s, _)| identity_bytes(p) + s.file.capacity())
                .sum::<usize>()
            + self
                .slot_parts
                .iter()
                .map(|(p, s, t)| identity_bytes(p) + s.file.capacity() + t.file.capacity())
                .sum::<usize>()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaintingPool {
    pub source: SourceId,
    pub available: Vec<i32>,
}
fn pools_bytes(pools: &Vec<PaintingPool>) -> usize {
    pools.capacity() * std::mem::size_of::<PaintingPool>()
        + pools
            .iter()
            .map(|p| p.source.file.capacity() + p.available.capacity() * std::mem::size_of::<i32>())
            .sum::<usize>()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnPhase {
    Synchronous,
    ComponentStart,
    SynchronousAndStarts,
}

/// The supplied controlled startup state does not describe untouched game startup.
pub struct SourceSpawnState {
    live: UnityRandom,
    painting_pools: Vec<PaintingPool>,
    parents: usize,
    boundaries: Vec<SpawnBoundary>,
    pub context: SpawnContext,
}
impl SourceSpawnState {
    pub fn supplied_controlled_startup(
        reference: &crate::reference::WorldSpawnContext,
        context: SpawnContext,
    ) -> Self {
        Self {
            live: UnityRandom::restore(UnityRandomState {
                words: reference.startup_state,
            }),
            boundaries: Vec::new(),
            painting_pools: Vec::new(),
            parents: 0,
            context,
        }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + pools_bytes(&self.painting_pools)
            + self.boundaries.capacity() * std::mem::size_of::<SpawnBoundary>()
            + self
                .boundaries
                .iter()
                .map(|b| {
                    identity_bytes(&b.parent)
                        + b.caller.file.capacity()
                        + pools_bytes(&b.painting_pools)
                        + b.initialization.retained_bytes()
                })
                .sum::<usize>()
    }
    pub fn boundaries(&self) -> &[SpawnBoundary] {
        &self.boundaries
    }
    pub fn parent_count(&self) -> usize {
        self.parents
    }
    pub fn boundary_count(&self) -> usize {
        self.boundaries.len()
    }
    pub fn live_state(&self) -> UnityRandomState {
        self.live.snapshot()
    }
    /// Reserve metadata before invoking an atomic source caller. Failed calls cannot consume live draws.
    pub fn invoke<T>(
        &mut self,
        parent: &GeneratedId,
        caller: &SourceId,
        ordinal: u32,
        phase: SpawnPhase,
        available_index_bytes: usize,
        staging: usize,
        initialization: &SpawnInitialization,
        call: impl FnOnce(
            &mut UnityRandom,
            SpawnContext,
            &mut Vec<PaintingPool>,
            &SpawnInitialization,
            usize,
        ) -> Result<T, ReferenceError>,
    ) -> Result<T, ReferenceError> {
        if let Some(boundary) = self.boundaries.iter().find(|b| {
            b.parent == *parent && b.caller == *caller && b.ordinal == ordinal && b.phase == phase
        }) {
            let overlap = pools_bytes(&boundary.painting_pools);
            if overlap > staging {
                return Err(ReferenceError(
                    "Spawn replay pool clone exceeds staging".into(),
                ));
            }
            return call(
                &mut UnityRandom::restore(boundary.random),
                boundary.context,
                &mut boundary.painting_pools.clone(),
                &boundary.initialization,
                staging - overlap,
            );
        }
        let extra = std::mem::size_of::<SpawnBoundary>()
            + identity_bytes(parent)
            + caller.file.len()
            + pools_bytes(&self.painting_pools)
            + initialization.retained_bytes();
        if self.retained_bytes().saturating_add(extra) > available_index_bytes {
            return Err(ReferenceError(
                "Source spawn replay metadata exceeds checkpoint budget".into(),
            ));
        }
        let overlap = extra
            + pools_bytes(&self.painting_pools)
            + if self.boundaries.len() == self.boundaries.capacity() {
                self.boundaries.capacity() * std::mem::size_of::<SpawnBoundary>()
            } else {
                0
            };
        if overlap > staging {
            return Err(ReferenceError(
                "Spawn boundary preparation exceeds staging".into(),
            ));
        }
        self.boundaries
            .try_reserve_exact(1)
            .map_err(|_| ReferenceError("Source spawn replay metadata allocation failed".into()))?;
        let boundary = SpawnBoundary {
            parent: parent.clone(),
            caller: caller.clone(),
            ordinal,
            phase,
            random: self.live.snapshot(),
            context: self.context,
            painting_pools: self.painting_pools.clone(),
            initialization: initialization.clone(),
        };
        let actual = self.retained_bytes()
            + identity_bytes(&boundary.parent)
            + boundary.caller.file.capacity()
            + pools_bytes(&boundary.painting_pools)
            + boundary.initialization.retained_bytes();
        if actual > available_index_bytes {
            self.boundaries.shrink_to_fit();
            return Err(ReferenceError(
                "Source spawn replay metadata capacity exceeds checkpoint budget".into(),
            ));
        }
        let mut random = self.live;
        let mut pools = self.painting_pools.clone();
        let value = call(
            &mut random,
            self.context,
            &mut pools,
            initialization,
            staging - overlap,
        )?;
        let final_bytes =
            actual.saturating_sub(pools_bytes(&self.painting_pools)) + pools_bytes(&pools);
        if final_bytes > available_index_bytes {
            self.boundaries.shrink_to_fit();
            return Err(ReferenceError(
                "Source mutable initialization inputs exceed checkpoint budget".into(),
            ));
        }
        if !self.boundaries.iter().any(|b| b.parent == *parent) {
            self.parents += 1;
        }
        self.boundaries.push(boundary);
        self.live = random;
        self.painting_pools = pools;
        Ok(value)
    }
}
fn identity_bytes(id: &GeneratedId) -> usize {
    id.generator.file.capacity()
        + id.children.capacity() * std::mem::size_of::<(SourceId, u32)>()
        + id.children
            .iter()
            .map(|(s, _)| s.file.capacity())
            .sum::<usize>()
}
