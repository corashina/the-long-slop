//! Required component Starts run after the synchronous source caller.
use super::spawn_contents::SpawnEvaluation;
use super::spawn_rules::*;
use super::{GeneratedId, Placement};
#[derive(Clone, Copy)]
pub(super) enum ConditionOverride {
    Default,
    Explicit([i32; 2]),
}
use crate::reference::{ReferenceError, SourceId};

impl SpawnEvaluation<'_> {
    pub(super) fn queue_start(
        &mut self,
        placement: Placement,
        condition: Option<ConditionOverride>,
    ) {
        self.starts.push((placement, condition));
    }
    pub fn finish_starts(&mut self) -> Result<(), ReferenceError> {
        let mut cursor = 0;
        while cursor < self.starts.len() {
            let (parent, condition) = self.starts[cursor].clone();
            cursor += 1;
            let actual = if let Some(unparent) =
                self.rules
                    .component(self.reference, &parent.prototype, "childunparent")?
            {
                link(unparent.fields, "g")?
            } else {
                &parent.prototype
            };
            let condition_source = self
                .rules
                .component(self.reference, actual, "tosaveitemscript")?
                .and_then(|r| array(r.fields, "partconditions").ok()?.first())
                .and_then(|v| value_link(v).ok());
            let mut callers = Vec::new();
            self.rules
                .start_components(self.reference, &parent.prototype, &mut callers)?;
            callers.sort_by_key(|source| {
                self.rules
                    .get(self.reference, source)
                    .unwrap()
                    .execution_order
            });
            let mut completed = vec![false; callers.len()];
            loop {
                let mut selected = None;
                for (index, source) in callers.iter().enumerate() {
                    if !completed[index] && self.component_active(&parent, source)? {
                        selected = Some(index);
                        break;
                    }
                }
                let Some(index) = selected else {
                    break;
                };
                completed[index] = true;
                let source = callers[index];
                let record = self.rules.get(self.reference, source)?;
                let f = &record.fields;
                match record.class_name {
                    "tosaveitemscript" if flag(f, "manualStart")? => {
                        self.start_save(&parent, record, 0)?
                    }
                    "partconditionscript" => {
                        let limits = if condition_source == Some(record.source) {
                            match condition {
                                Some(ConditionOverride::Explicit(v)) => Some(v),
                                Some(ConditionOverride::Default) => {
                                    let limits =
                                        [integer(f, "randomMin")?, integer(f, "randomMax")?];
                                    Some(if limits == [0, 0] { [0, 1] } else { limits })
                                }
                                None => None,
                            }
                        } else {
                            None
                        };
                        self.start_condition(&parent, record, limits, 0)?;
                    }
                    "randomTypeSelector" if flag(f, "forceStart")? => {
                        self.start_selector(&parent, record)?
                    }
                    "electronicsscript" => {
                        self.mark_started(&parent.id, source);
                    }
                    "flashlightscript" => {
                        if self.mark_started(&parent.id, source) {
                            let on = if flag(f, "randomStart")? && self.random.range_int(0, 2) == 0
                            {
                                true
                            } else {
                                flag(f, "ON")?
                            };
                            self.initialization
                                .lamps
                                .push((parent.id.clone(), source.clone(), on));
                        }
                    }
                    "partslotscript" => {
                        if self.mark_started(&parent.id, source) {
                            self.deferred.push((parent.clone(), source.clone()));
                        }
                    }
                    "newAiScript" => {
                        if !self.mark_started(&parent.id, source) {
                            continue;
                        }
                        if !flag(f, "sleeping")? {
                            self.random.range(0., 1.);
                        }
                        self.random
                            .range(number(f, "voiceMinPitch")?, number(f, "voiceMaxPitch")?);
                        self.random.range(
                            number(f, "startPatrolTimeMin")?,
                            number(f, "startPatrolTimeMax")?,
                        );
                    }
                    "randomFlyer" => {
                        if !self.mark_started(&parent.id, source) {
                            continue;
                        }
                        self.random
                            .range(number(f, "minTime")?, number(f, "maxTime")?);
                        self.random.on_unit_sphere();
                        self.random
                            .range(number(f, "minRange")?, number(f, "maxRange")?);
                        self.random
                            .range(number(f, "minSpeed")?, number(f, "maxSpeed")?);
                        self.random
                            .range(number(f, "soundMinTime")?, number(f, "soundMaxTime")?);
                    }
                    "nyulscript" => {
                        if !self.mark_started(&parent.id, source) {
                            continue;
                        }
                        self.random.range(0., 1.);
                    }
                    _ => {}
                }
            }
        }
        self.bake_appearance()?;
        self.bake_lamps()?;
        Ok(())
    }
    pub(super) fn mark_started(&mut self, parent: &GeneratedId, source: &SourceId) -> bool {
        if self
            .initialization
            .initialized
            .iter()
            .any(|(p, s)| p == parent && s == source)
        {
            return false;
        }
        self.initialization
            .initialized
            .push((parent.clone(), source.clone()));
        true
    }
    pub(super) fn start_selector(
        &mut self,
        parent: &Placement,
        record: SourceRecord<'_>,
    ) -> Result<(), ReferenceError> {
        let f = &record.fields;
        if flag(f, "started")?
            || !flag(f, "m_Enabled")?
            || !self.mark_started(&parent.id, &record.source)
        {
            return Ok(());
        }
        let rows = array(f, "tipusok")?;
        let total = rows.iter().try_fold(0., |v, r| {
            Ok::<_, ReferenceError>(v + number(fields(r)?, "chance")?)
        })?;
        let selected = self.random.range(0., total);
        let mut cumulative = 0.;
        let mut index = 0;
        for (i, row) in rows.iter().enumerate() {
            cumulative += number(fields(row)?, "chance")?;
            if selected < cumulative {
                index = i;
                break;
            }
        }
        self.refresh_selector(parent, record, index, 0)
    }
    fn refresh_selector(
        &mut self,
        parent: &Placement,
        record: SourceRecord<'_>,
        mut index: usize,
        depth: usize,
    ) -> Result<(), ReferenceError> {
        if depth >= 64 {
            return Err(ReferenceError("Source selector child cycle".into()));
        }
        let f = record.fields;
        let rows = array(f, "tipusok")?;
        if index >= rows.len() {
            index = 0;
        }
        let selected = rows
            .get(index)
            .ok_or_else(|| ReferenceError("Source selector has no variants".into()))?;
        if let Some(state) = self
            .initialization
            .selectors
            .iter_mut()
            .find(|(p, s, _)| *p == parent.id && s == record.source)
        {
            state.2 = index;
        } else {
            self.initialization
                .selectors
                .push((parent.id.clone(), record.source.clone(), index));
        }
        self.mark_started(&parent.id, record.source);
        for row in rows {
            for object in array(fields(row)?, "objects")? {
                self.set_active(parent, value_link(object)?, false);
            }
        }
        let selected = fields(selected)?;
        for object in array(selected, "objects")? {
            self.set_active(parent, value_link(object)?, true);
        }
        let slots = optional_array(selected, "partslots")?;
        let parts = optional_array(selected, "parts")?;
        if !slots.is_empty() && !parts.is_empty() {
            for (index, slot) in slots.iter().enumerate() {
                let slot = value_link(slot)?;
                let part =
                    value_link(parts.get(index).ok_or_else(|| {
                        ReferenceError("Source selector part count differs".into())
                    })?)?;
                if let Some(state) = self
                    .initialization
                    .slot_parts
                    .iter_mut()
                    .find(|(p, s, _)| *p == parent.id && s == slot)
                {
                    state.2 = part.clone();
                } else {
                    self.initialization.slot_parts.push((
                        parent.id.clone(),
                        slot.clone(),
                        part.clone(),
                    ));
                }
                let root = self.rules.root(self.reference, part)?;
                self.set_active(parent, self.rules.node(self.reference, root)?.owner, true);
            }
        }
        for child in array(f, "childs")? {
            self.refresh_selector(
                parent,
                self.rules.get(self.reference, value_link(child)?)?,
                index,
                depth + 1,
            )?;
        }
        Ok(())
    }
    fn set_active(&mut self, parent: &Placement, object: &SourceId, active: bool) {
        if let Some(state) = self
            .initialization
            .activations
            .iter_mut()
            .find(|(p, s, _)| *p == parent.id && s == object)
        {
            state.2 = active;
        } else {
            self.initialization
                .activations
                .push((parent.id.clone(), object.clone(), active));
        }
    }
    fn component_active(
        &self,
        parent: &Placement,
        source: &SourceId,
    ) -> Result<bool, ReferenceError> {
        let mut current = self.rules.root(self.reference, source)?;
        for _ in 0..256 {
            let node = self.rules.node(self.reference, current)?;
            let active = self
                .initialization
                .activations
                .iter()
                .find(|(p, s, _)| *p == parent.id && s == node.owner)
                .map(|s| s.2)
                .unwrap_or(*current == parent.prototype || node.active);
            if !active {
                return Ok(false);
            }
            if *current == parent.prototype {
                return Ok(true);
            }
            let Some(next) = node.parent else {
                return Err(ReferenceError(
                    "Source Start node is outside its instance".into(),
                ));
            };
            current = next;
        }
        Err(ReferenceError("Source Start hierarchy cycle".into()))
    }
}
