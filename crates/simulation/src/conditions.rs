use crate::reference::{
    ConditionId, ConditionStartup, ConditionSurface, ItemId, ReferenceError, ReferenceManifest,
    WashId,
};
use crate::save::{ConditionSnapshot, SaveError, SourceKey, SurfaceSnapshot, WashSnapshot};
use std::collections::{HashMap, VecDeque};

pub type ConditionState = ConditionStartup;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairResult {
    None,
    Improved,
    Worsened,
}
#[derive(Clone, Copy, Debug)]
pub struct WashState {
    pub active: bool,
    pub on: bool,
    pub destroyed: bool,
    pub repair_progress: f32,
}

impl WashState {
    pub fn visible(&self) -> bool {
        self.active && !self.destroyed
    }
}

pub struct ConditionStore {
    states: Vec<ConditionState>,
    records: Vec<ConditionSurface>,
    indices: HashMap<ConditionId, usize>,
    items: HashMap<ItemId, ConditionId>,
    tables: Vec<[[bool; 7]; 5]>,
    decay_rate: f32,
    pub washes: Vec<WashState>,
    wash_indices: HashMap<WashId, usize>,
    dirty: VecDeque<ConditionId>,
    queued: Vec<bool>,
    traversal: Vec<ConditionId>,
}
impl ConditionStore {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        reference.conditions.validate(reference)?;
        let r = &reference.conditions;
        let n = r.surfaces.len();
        Ok(Self {
            states: r.surfaces.iter().map(|s| s.startup).collect(),
            records: r.surfaces.clone(),
            indices: r
                .surfaces
                .iter()
                .enumerate()
                .map(|(i, s)| (s.id, i))
                .collect(),
            items: r
                .surfaces
                .iter()
                .filter_map(|s| s.item.map(|i| (i, s.id)))
                .collect(),
            tables: r.repair_tables.clone(),
            decay_rate: r.repair_decay_rate,
            washes: r
                .washes
                .iter()
                .map(|w| WashState {
                    active: w.active,
                    on: w.on,
                    destroyed: false,
                    repair_progress: w.repair_progress,
                })
                .collect(),
            wash_indices: r
                .washes
                .iter()
                .enumerate()
                .map(|(i, w)| (w.id, i))
                .collect(),
            dirty: VecDeque::with_capacity(n),
            queued: vec![false; n],
            traversal: Vec::with_capacity(
                1 + r.surfaces.iter().map(|s| s.children.len()).sum::<usize>(),
            ),
        })
    }
    pub fn update(&mut self, delta: f32, active: bool) {
        if !active || !delta.is_finite() || delta <= 0. {
            return;
        }
        let amount = self.decay_rate * delta;
        for state in &mut self.states {
            state.repair_progress =
                state.repair_progress.signum() * (state.repair_progress.abs() - amount).max(0.);
        }
    }
    pub fn get(&self, id: ConditionId) -> &ConditionState {
        &self.states[self.indices[&id]]
    }
    pub fn quality(&self, id: ConditionId) -> f32 {
        1. - f32::from(self.get(id).state.min(4)) / 4.
    }
    pub fn item_quality(&self, item: ItemId) -> Option<f32> {
        self.items.get(&item).map(|id| self.quality(*id))
    }
    pub fn wash(&self, id: WashId) -> &WashState {
        &self.washes[self.wash_indices[&id]]
    }
    pub fn can_repair(&self, id: ConditionId, tool: u8) -> bool {
        let i = self.indices[&id];
        self.tables[self.records[i].permission_index as usize]
            .get(self.states[i].state as usize)
            .and_then(|row| row.get(tool as usize))
            .copied()
            .unwrap_or(false)
    }
    pub fn repair(&mut self, id: ConditionId, tool: u8, gain: f32) -> RepairResult {
        if !gain.is_finite() || gain < 0. {
            return RepairResult::None;
        }
        let i = self.indices[&id];
        let result = if tool == 1 && self.can_repair(id, 3) && self.states[i].state < 4 {
            self.states[i].repair_progress -= gain;
            if self.states[i].repair_progress <= -1. {
                self.states[i].state += 1;
                RepairResult::Worsened
            } else {
                RepairResult::None
            }
        } else if self.states[i].state > 0 && self.can_repair(id, tool) {
            self.states[i].repair_progress += gain;
            if self.states[i].repair_progress >= 1. {
                self.states[i].state -= 1;
                RepairResult::Improved
            } else {
                RepairResult::None
            }
        } else {
            RepairResult::None
        };
        if result != RepairResult::None {
            self.states[i].repair_progress = 0.;
            self.refresh(id);
        }
        result
    }
    pub fn paint(&mut self, id: ConditionId, color: [f32; 4]) -> bool {
        if !color.iter().all(|v| v.is_finite()) || !self.can_repair(id, 0) {
            return false;
        }
        let i = self.indices[&id];
        if self.states[i].color == color {
            return false;
        }
        self.states[i].color = color;
        self.refresh(id);
        true
    }
    pub fn wear(&mut self, id: ConditionId) -> bool {
        let i = self.indices[&id];
        if self.states[i].state >= 5 {
            return true;
        }
        self.states[i].state += 1;
        // Source tool disable does not call Refresh with material index five.
        if self.states[i].state < 5 {
            self.refresh(id);
        } else {
            self.mark(id);
        }
        self.states[i].state == 5
    }
    fn mark(&mut self, id: ConditionId) {
        let i = self.indices[&id];
        if !self.queued[i] {
            self.queued[i] = true;
            self.dirty.push_back(id);
        }
    }
    fn refresh(&mut self, id: ConditionId) {
        self.traversal.clear();
        self.traversal.push(id);
        while let Some(id) = self.traversal.pop() {
            let i = self.indices[&id];
            self.mark(id);
            let state = self.states[i];
            for child_index in 0..self.records[i].children.len() {
                let child = self.records[i].children[child_index];
                let ci = self.indices[&child];
                let c = &self.records[ci];
                if !c.is_child && !c.same_state_as_parent {
                    continue;
                }
                if !c.is_child || !self.records[i].no_inherit_state {
                    self.states[ci].state = state.state;
                    self.states[ci].state2 = state.state2;
                    if !c.is_child || !c.random_start_state3 {
                        self.states[ci].state3 = state.state3;
                    }
                }
                // Startup already performed the source's one-time child color draw.
                self.traversal.push(child);
            }
        }
    }
    pub fn take_changed(&mut self) -> Option<ConditionId> {
        let id = self.dirty.pop_front()?;
        self.queued[self.indices[&id]] = false;
        Some(id)
    }
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
    ) -> Result<(Vec<SurfaceSnapshot>, Vec<WashSnapshot>), SaveError> {
        Ok((
            reference
                .conditions
                .surfaces
                .iter()
                .map(|s| {
                    let v = self.get(s.id);
                    SurfaceSnapshot {
                        source: SourceKey::from(&s.source),
                        state: v.state,
                        state2: v.state2,
                        state3: v.state3,
                        color: v.color,
                        repair_progress: v.repair_progress,
                    }
                })
                .collect(),
            reference
                .conditions
                .washes
                .iter()
                .map(|w| {
                    let v = self.wash(w.id);
                    WashSnapshot {
                        source: SourceKey::from(&w.source),
                        active: v.active,
                        on: v.on,
                        destroyed: v.destroyed,
                        repair_progress: v.repair_progress,
                    }
                })
                .collect(),
        ))
    }
    pub fn restore_save(
        &mut self,
        reference: &ReferenceManifest,
        saved: &ConditionSnapshot,
    ) -> Result<(), SaveError> {
        validate_surfaces(reference, saved)?;
        for s in &reference.conditions.surfaces {
            let v = saved
                .surfaces
                .iter()
                .find(|v| v.source == SourceKey::from(&s.source))
                .unwrap();
            self.states[self.indices[&s.id]] = ConditionState {
                state: v.state,
                state2: v.state2,
                state3: v.state3,
                color: v.color,
                repair_progress: v.repair_progress,
            };
            self.mark(s.id);
        }
        for w in &reference.conditions.washes {
            let v = saved
                .washes
                .iter()
                .find(|v| v.source == SourceKey::from(&w.source))
                .unwrap();
            self.washes[self.wash_indices[&w.id]] = WashState {
                active: v.active,
                on: v.on,
                destroyed: v.destroyed,
                repair_progress: v.repair_progress,
            };
        }
        Ok(())
    }
}
pub fn validate_surfaces(
    reference: &ReferenceManifest,
    saved: &ConditionSnapshot,
) -> Result<(), SaveError> {
    let invalid = |s: &str| SaveError::CorruptData(s.into());
    if saved.surfaces.len() != reference.conditions.surfaces.len()
        || saved.washes.len() != reference.conditions.washes.len()
    {
        return Err(invalid("Missing condition owner"));
    }
    for r in &reference.conditions.surfaces {
        let mut entries = saved
            .surfaces
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.source));
        let v = entries
            .next()
            .ok_or_else(|| invalid("Missing surface owner"))?;
        let disabled = reference
            .conditions
            .brushes
            .iter()
            .any(|b| b.own_condition == r.id);
        if entries.next().is_some()
            || v.state > if disabled { 5 } else { 4 }
            || !v.color.iter().all(|c| c.is_finite())
            || !v.repair_progress.is_finite()
            || v.repair_progress.abs() >= 1.
            || (r.use_mesh_list
                && r.renderers
                    .iter()
                    .any(|renderer| v.state2 as usize >= renderer.meshes.len()))
            || (r.use_object_list && !r.objects.is_empty() && v.state3 as usize >= r.objects.len())
            || (!r.use_mesh_list && v.state2 != r.startup.state2)
            || (!r.use_object_list && v.state3 != r.startup.state3)
        {
            return Err(invalid("Invalid surface state or duplicate owner"));
        }
    }
    for r in &reference.conditions.washes {
        let mut entries = saved
            .washes
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.source));
        let v = entries
            .next()
            .ok_or_else(|| invalid("Missing wash owner"))?;
        if entries.next().is_some()
            || !v.repair_progress.is_finite()
            || (v.repair_progress < 0. || (!r.never_repair && v.repair_progress > 1.))
            || (v.destroyed && (!r.destroy_on_repair || v.active || v.on))
        {
            return Err(invalid("Invalid wash state or duplicate owner"));
        }
    }
    Ok(())
}
