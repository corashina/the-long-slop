//! Active nested cursors and compact first-call replay boundaries.
use super::spawn_contents::{SpawnEvaluation, SpawnedContent};
use super::spawn_rules::*;
use super::{GeneratedId, Placement, SourceSpawnState, SpawnContext, SpawnPhase};
use crate::reference::{ItemId, ReferenceError, SourceId, WorldReference};
struct DeferredBatch {
    owner: GeneratedId,
    tasks: Vec<(Placement, SourceId)>,
    inputs: super::spawn_state::SpawnInitialization,
    next: usize,
    eligible: u64,
}
struct Cursor {
    parent: Placement,
    building: SourceId,
    callers: Vec<SourceId>,
    next: usize,
    chance: Option<f32>,
    root: Option<SpawnedContent>,
    inputs: super::spawn_state::SpawnInitialization,
}
pub(crate) struct ContentBinding {
    pub render_body: Option<(
        rapier3d::prelude::RigidBodyHandle,
        crate::vehicle::VehiclePose,
    )>,
    pub source: SourceId,
    pub item: ItemId,
    pub baseline: super::ObjectSnapshot,
}
pub(crate) struct ContentSlot {
    pub source: SourceId,
    pub slot: crate::reference::PartSlotId,
    pub initialized: bool,
}
pub(crate) struct ContentCollider {
    pub handle: crate::vehicle::ColliderHandle,
    pub node: usize,
    pub local: rapier3d::math::Pose,
}
pub struct GeneratedCapVisual {
    pub opening: crate::reference::OpeningId,
    pub visual: crate::reference::CapVisualReference,
}
pub(crate) struct ContentInstance {
    pub caps: Vec<GeneratedCapVisual>,
    pub poses: Vec<crate::vehicle::VehiclePose>,
    pub mechanism_colliders: Vec<ContentCollider>,
    pub slots: Vec<ContentSlot>,
    pub bindings: Vec<ContentBinding>,
    pub nodes: Vec<crate::reference::NodeId>,
    pub render_body: Option<(
        rapier3d::prelude::RigidBodyHandle,
        crate::vehicle::VehiclePose,
    )>,
    pub content: SpawnedContent,
    pub item: Option<ItemId>,
    pub baseline: Option<super::ObjectSnapshot>,
}
impl ContentInstance {
    pub fn for_structure(placement: Placement) -> Self {
        Self {
            caps: Vec::new(),
            slots: Vec::new(),
            bindings: Vec::new(),
            nodes: Vec::new(),
            poses: Vec::new(),
            mechanism_colliders: Vec::new(),
            render_body: None,
            item: None,
            baseline: None,
            content: SpawnedContent {
                placement,
                inputs: super::spawn_state::SpawnInitialization::default(),
                appearance: Vec::new(),
                lights: Vec::new(),
                fluid: None,
                tanks: Vec::new(),
                condition: 1.,
                buried: false,
                distance_offset: [0.; 3],
            },
        }
    }
    pub fn has_external_dependents(
        &self,
        items: &crate::interaction::items::ItemStore,
        parts: &crate::interaction::parts::PartAttachments,
        wheels: &crate::interaction::wheels::WheelAssemblies,
    ) -> bool {
        let owns = |item| self.item == Some(item) || self.bindings.iter().any(|b| b.item == item);
        items.has_external_dependents(owns)
            || parts.has_external_dependents(owns)
            || wheels.has_external_dependents(owns)
    }
    pub fn control_heap_bytes(&self) -> usize {
        placement_heap(&self.content.placement)
            + self.bindings.capacity() * std::mem::size_of::<ContentBinding>()
            + self
                .bindings
                .iter()
                .map(|b| b.source.file.capacity())
                .sum::<usize>()
            + self.nodes.capacity() * std::mem::size_of::<crate::reference::NodeId>()
            + self.poses.capacity() * std::mem::size_of::<crate::vehicle::VehiclePose>()
            + self.mechanism_colliders.capacity() * std::mem::size_of::<ContentCollider>()
    }
}
pub(crate) struct SourceContents {
    pub rules: SourceRules,
    pub state: SourceSpawnState,
    cursors: Vec<Cursor>,
    pub active: Vec<ContentInstance>,
    next: usize,
    update: u64,
    deferred: Vec<DeferredBatch>,
}
impl SourceContents {
    pub fn new(reference: &WorldReference) -> Self {
        Self {
            rules: SourceRules::new(reference),
            state: SourceSpawnState::supplied_controlled_startup(
                &reference.spawn_context,
                SpawnContext {
                    current_car: 1,
                    peaceful_enemies: 0,
                    date: reference.spawn_context.date,
                    chance_bits: None,
                },
            ),
            cursors: Vec::new(),
            active: Vec::new(),
            deferred: Vec::new(),
            update: 0,
            next: 0,
        }
    }
    fn admit_queues(&mut self, limit: usize, staging: usize) -> Result<usize, ReferenceError> {
        fn overlap<T>(values: &Vec<T>, extra: usize) -> usize {
            let required = values.len().saturating_add(extra);
            if required > values.capacity() {
                (required + values.capacity()).saturating_mul(std::mem::size_of::<T>())
            } else {
                0
            }
        }
        let bytes = overlap(&self.active, limit).saturating_add(overlap(&self.deferred, 1));
        if bytes > staging {
            return Err(ReferenceError(
                "Source continuation queues exceed staging".into(),
            ));
        }
        self.active
            .try_reserve_exact(limit)
            .map_err(|_| ReferenceError("Source content queue allocation failed".into()))?;
        self.deferred
            .try_reserve_exact(1)
            .map_err(|_| ReferenceError("Source continuation queue allocation failed".into()))?;
        Ok(staging - bytes)
    }
    pub fn advance(
        &mut self,
        reference: &WorldReference,
        structures: &[Placement],
        focus: [[f64; 3]; 2],
        available_index_bytes: usize,
        limit: usize,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let staging = self.admit_queues(limit, staging)?;
        self.update = self.update.wrapping_add(1);
        self.cursors
            .retain(|c| structures.iter().any(|p| p.id == c.parent.id));
        self.deferred
            .retain(|batch| self.cursors.iter().any(|c| c.parent.id == batch.owner));
        let eligible = self
            .deferred
            .iter()
            .filter(|b| b.eligible <= self.update)
            .map(|b| b.tasks.len() - b.next)
            .sum::<usize>();
        let initial_count = self.active.len();
        let initial_bytes = self.retained_bytes();
        for _ in 0..eligible {
            let remaining = limit.saturating_sub(self.active.len().saturating_sub(initial_count));
            let available =
                staging.saturating_sub(self.retained_bytes().saturating_sub(initial_bytes));
            if remaining == 0 || available == 0 {
                return Ok(());
            }
            self.advance_deferred(reference, available_index_bytes, remaining, available)?;
        }
        let limit = limit.saturating_sub(self.active.len().saturating_sub(initial_count));
        let staging = staging.saturating_sub(self.retained_bytes().saturating_sub(initial_bytes));
        if limit == 0 || staging == 0 {
            return Ok(());
        }
        let spawn = reference.distances["ItemSpawnDist"];
        let cursor_before = self.retained_bytes();
        for instance in structures {
            let p = instance;
            if !p.id.children.is_empty() || self.cursors.iter().any(|c| c.parent.id == p.id) {
                continue;
            }
            let Some(building) = self
                .rules
                .component(reference, &p.prototype, "buildingscript")?
            else {
                continue;
            };
            let bound = self.rules.initialization_bound(
                reference,
                &p.prototype,
                identity_heap(&p.id),
                0,
            )? + (self.cursors.capacity() + self.cursors.len() + 1)
                * std::mem::size_of::<Cursor>();
            if bound > staging.saturating_sub(self.retained_bytes().saturating_sub(cursor_before)) {
                return Err(ReferenceError(
                    "Source caller cursor exceeds staging".into(),
                ));
            }
            self.cursors
                .try_reserve_exact(1)
                .map_err(|_| ReferenceError("Source cursor allocation failed".into()))?;
            let mut callers = Vec::new();
            self.rules.children(
                reference,
                &p.prototype,
                "newRandomStuffSpawnScript",
                true,
                &mut callers,
            )?;
            let mut eligible = Vec::new();
            for source in callers {
                if !flag(self.rules.get(reference, source)?.fields, "dontSpawn")? {
                    eligible.push(source.clone());
                }
            }
            self.cursors.push(Cursor {
                parent: p.clone(),
                building: building.source.clone(),
                callers: eligible,
                next: 0,
                chance: None,
                root: None,
                inputs: super::spawn_state::SpawnInitialization::default(),
            });
        }
        let staging = staging.saturating_sub(self.retained_bytes().saturating_sub(cursor_before));
        if let Some(index) = self.cursors.iter().position(|c| c.root.is_none()) {
            let cursor = &self.cursors[index];
            let parent = &cursor.parent;
            let (mut output, deferred, inputs) = self.state.invoke(
                &parent.id,
                &cursor.building,
                0,
                SpawnPhase::ComponentStart,
                available_index_bytes,
                staging,
                &cursor.inputs,
                |random, context, pools, initialization, staging| {
                    let mut e =
                        SpawnEvaluation::new(reference, &self.rules, random, context, pools, limit);
                    e.staging = staging;
                    e.admit_initialization(parent)?;
                    e.initialization = initialization.clone();
                    e.output.push(SpawnedContent {
                        inputs: super::spawn_state::SpawnInitialization::default(),
                        placement: parent.clone(),
                        appearance: Vec::new(),
                        lights: Vec::new(),
                        fluid: None,
                        tanks: Vec::new(),
                        condition: 1.,
                        buried: false,
                        distance_offset: [0.; 3],
                    });
                    e.queue_start(parent.clone(), None);
                    e.finish_starts()?;
                    e.check_staging(staging)?;
                    Ok((e.output, e.deferred, e.initialization))
                },
            )?;
            self.cursors[index].root = Some(output.remove(0));
            self.cursors[index].inputs = inputs.for_parent(&self.cursors[index].parent.id);
            if !deferred.is_empty() {
                self.deferred.push(DeferredBatch {
                    owner: self.cursors[index].parent.id.clone(),
                    tasks: deferred,
                    inputs,
                    next: 0,
                    eligible: self.update.wrapping_add(1),
                });
            }
            for content in output {
                self.active.push(ContentInstance {
                    content,
                    poses: Vec::new(),
                    mechanism_colliders: Vec::new(),
                    caps: Vec::new(),
                    slots: Vec::new(),
                    bindings: Vec::new(),
                    nodes: Vec::new(),
                    render_body: None,
                    item: None,
                    baseline: None,
                });
            }
            return Ok(());
        }
        if self.cursors.is_empty() {
            return Ok(());
        }
        let before_count = self.active.len();
        let before_bytes = self.retained_bytes();
        for _ in 0..self.cursors.len() {
            let limit = limit.saturating_sub(self.active.len().saturating_sub(before_count));
            let staging =
                staging.saturating_sub(self.retained_bytes().saturating_sub(before_bytes));
            if limit == 0 || staging == 0 {
                break;
            }
            let index = self.next % self.cursors.len();
            self.next = (index + 1) % self.cursors.len();
            let staging = self.admit_queues(limit, staging)?;
            let cursor = &self.cursors[index];
            if focus
                .iter()
                .all(|f| distance(*f, cursor.parent.pose.position) >= spawn * spawn)
                || self.deferred.iter().any(|b| b.owner == cursor.parent.id)
                || cursor.next >= cursor.callers.len().max(1)
            {
                continue;
            }
            let first = cursor.next == 0;
            let source = if first {
                &cursor.building
            } else {
                &cursor.callers[cursor.next]
            };
            let parent = &cursor.parent;
            let chance = cursor.chance;
            self.state.context.chance_bits = chance.map(f32::to_bits);
            self.active
                .try_reserve_exact(limit)
                .map_err(|_| ReferenceError("Nested content allocation failed".into()))?;
            self.deferred
                .try_reserve_exact(1)
                .map_err(|_| ReferenceError("Source continuation allocation failed".into()))?;
            let (output, chance, deferred, initialization) = self.state.invoke(
                &parent.id,
                source,
                cursor.next as u32,
                SpawnPhase::SynchronousAndStarts,
                available_index_bytes,
                staging,
                &cursor.inputs,
                |random, context, pools, initialization, staging| {
                    let mut evaluation =
                        SpawnEvaluation::new(reference, &self.rules, random, context, pools, limit);
                    evaluation.staging = staging;
                    evaluation.admit_initialization(parent)?;
                    evaluation.initialization = initialization.clone();
                    if let Some(root) = &cursor.root {
                        evaluation.output.push(root.clone());
                    }
                    let chance = if first {
                        evaluation.building(parent, &cursor.building)?
                    } else {
                        context.chance_bits.map(f32::from_bits)
                    };
                    if let Some(caller) = cursor.callers.get(cursor.next) {
                        let f = self.rules.get(reference, caller)?.fields;
                        evaluation.spawn(
                            parent,
                            caller,
                            if flag(f, "buildingAffectsThis")? {
                                chance
                            } else {
                                None
                            },
                        )?;
                    }
                    evaluation.finish_starts()?;
                    evaluation.check_staging(staging)?;
                    Ok((
                        evaluation.output,
                        chance,
                        evaluation.deferred,
                        evaluation.initialization,
                    ))
                },
            )?;
            let owner = parent.id.clone();
            self.cursors[index].inputs = initialization.for_parent(&self.cursors[index].parent.id);
            if !deferred.is_empty() {
                self.deferred.push(DeferredBatch {
                    owner,
                    tasks: deferred,
                    inputs: initialization,
                    next: 0,
                    eligible: self.update.wrapping_add(1),
                });
            }
            self.cursors[index].next += 1;
            self.cursors[index].chance = chance;
            for content in output {
                if content.placement.id == self.cursors[index].parent.id {
                    self.cursors[index].root = Some(content);
                    continue;
                }
                if !self
                    .active
                    .iter()
                    .any(|c| c.content.placement.id == content.placement.id)
                {
                    self.active.push(ContentInstance {
                        content,
                        poses: Vec::new(),
                        mechanism_colliders: Vec::new(),
                        caps: Vec::new(),
                        slots: Vec::new(),
                        bindings: Vec::new(),
                        nodes: Vec::new(),
                        render_body: None,
                        item: None,
                        baseline: None,
                    });
                }
            }
        }
        Ok(())
    }
    fn advance_deferred(
        &mut self,
        reference: &WorldReference,
        available_index_bytes: usize,
        limit: usize,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let staging = self.admit_queues(limit, staging)?;
        let Some(index) = self.deferred.iter().position(|b| b.eligible <= self.update) else {
            return Ok(());
        };
        self.deferred
            .try_reserve_exact(1)
            .map_err(|_| ReferenceError("Source continuation allocation failed".into()))?;
        let batch = &self.deferred[index];
        let (parent, source) = &batch.tasks[batch.next];
        self.active
            .try_reserve_exact(limit)
            .map_err(|_| ReferenceError("Deferred content allocation failed".into()))?;
        let (output, deferred, initialization) = self.state.invoke(
            &parent.id,
            source,
            batch.next as u32,
            SpawnPhase::ComponentStart,
            available_index_bytes,
            staging,
            &batch.inputs,
            |random, context, pools, initialization, staging| {
                let mut evaluation =
                    SpawnEvaluation::new(reference, &self.rules, random, context, pools, limit);
                evaluation.staging = staging;
                evaluation.admit_initialization(parent)?;
                evaluation.initialization = initialization.clone();
                // The continuation can change its existing instance's selector and condition.
                if let Some(existing) = self
                    .active
                    .iter()
                    .find(|c| c.content.placement.id == parent.id)
                {
                    evaluation.output.push(existing.content.clone());
                } else if let Some(root) = self
                    .cursors
                    .iter()
                    .find(|c| c.parent.id == parent.id)
                    .and_then(|c| c.root.as_ref())
                {
                    evaluation.output.push(root.clone());
                }
                evaluation.spawn_slot(parent, source)?;
                evaluation.finish_starts()?;
                evaluation.check_staging(staging)?;
                Ok((
                    evaluation.output,
                    evaluation.deferred,
                    evaluation.initialization,
                ))
            },
        )?;
        let owner = &self.deferred[index].owner;
        if let Some(cursor) = self.cursors.iter_mut().find(|c| &c.parent.id == owner) {
            cursor.inputs = initialization.for_parent(&cursor.parent.id);
        }
        let batch = &mut self.deferred[index];
        batch.next += 1;
        batch.inputs = initialization;
        if !deferred.is_empty() {
            // A newly started slot always yields past this update.
            let next = DeferredBatch {
                owner: batch.owner.clone(),
                tasks: deferred,
                inputs: batch.inputs.clone(),
                next: 0,
                eligible: self.update.wrapping_add(1),
            };
            self.deferred.push(next);
        }
        if self.deferred[index].next == self.deferred[index].tasks.len() {
            self.deferred.remove(index);
        }
        for content in output {
            if let Some(cursor) = self
                .cursors
                .iter_mut()
                .find(|c| c.parent.id == content.placement.id)
            {
                cursor.root = Some(content);
            } else if let Some(existing) = self
                .active
                .iter_mut()
                .find(|c| c.content.placement.id == content.placement.id)
            {
                existing.content = content;
            } else {
                self.active.push(ContentInstance {
                    content,
                    poses: Vec::new(),
                    mechanism_colliders: Vec::new(),
                    caps: Vec::new(),
                    slots: Vec::new(),
                    bindings: Vec::new(),
                    nodes: Vec::new(),
                    render_body: None,
                    item: None,
                    baseline: None,
                });
            }
        }
        Ok(())
    }
    pub fn lamp_states(&self) -> impl Iterator<Item = &(GeneratedId, SourceId, bool)> {
        self.instances().flat_map(|c| &c.inputs.lamps)
    }
    pub fn set_lamp(
        &mut self,
        reference: &WorldReference,
        parent: &GeneratedId,
        source: &SourceId,
        on: bool,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let current = self
            .instances()
            .find(|c| &c.placement.id == parent)
            .ok_or_else(|| ReferenceError("Generated lamp instance missing".into()))?;
        let required = current.inputs.retained_bytes().saturating_mul(4)
            + current.appearance.capacity() * std::mem::size_of::<super::NodeAppearance>() * 2
            + current.lights.capacity() * std::mem::size_of::<super::WorldLight>() * 2
            + placement_heap(&current.placement) * 2;
        if required > staging {
            return Err(ReferenceError(
                "Generated lamp refresh exceeds shared staging".into(),
            ));
        }
        let mut inputs = current.inputs.clone();
        let current = current.clone();
        inputs
            .lamps
            .iter_mut()
            .find(|(p, s, _)| p == parent && s == source)
            .unwrap()
            .2 = on;
        let mut random = super::UnityRandom::restore(self.state.live_state());
        let mut pools = Vec::new();
        let mut evaluation = SpawnEvaluation::new(
            reference,
            &self.rules,
            &mut random,
            self.state.context,
            &mut pools,
            1,
        );
        evaluation.staging = staging;
        evaluation.admit_initialization(&current.placement)?;
        evaluation.initialization = inputs;
        evaluation.output.push(current);
        evaluation.bake_appearance()?;
        evaluation.bake_lamps()?;
        evaluation.check_staging(staging)?;
        let content = evaluation.output.pop().unwrap();
        if let Some(cursor) = self.cursors.iter_mut().find(|c| &c.parent.id == parent) {
            cursor.inputs = evaluation.initialization;
        }
        for batch in &mut self.deferred {
            if let Some(state) = batch
                .inputs
                .lamps
                .iter_mut()
                .find(|(p, s, _)| p == parent && s == source)
            {
                state.2 = on;
            }
        }
        if let Some(active) = self
            .active
            .iter_mut()
            .find(|c| &c.content.placement.id == parent)
        {
            active.content = content;
        } else {
            self.cursors
                .iter_mut()
                .find(|c| &c.parent.id == parent)
                .unwrap()
                .root = Some(content);
        }
        Ok(())
    }
    pub fn pending_bytes(&self) -> usize {
        self.deferred.capacity() * std::mem::size_of::<DeferredBatch>()
            + self
                .deferred
                .iter()
                .map(|b| {
                    identity_heap(&b.owner)
                        + b.tasks.capacity() * std::mem::size_of::<(Placement, SourceId)>()
                        + b.inputs.retained_bytes()
                        + b.tasks
                            .iter()
                            .map(|(p, s)| placement_heap(p) + s.file.capacity())
                            .sum::<usize>()
                })
                .sum::<usize>()
    }
    pub fn pending_requests(&self) -> usize {
        self.deferred.len()
    }
    pub fn initialized(&self, id: &GeneratedId) -> bool {
        self.cursors
            .iter()
            .find(|c| &c.parent.id == id)
            .is_none_or(|c| c.root.is_some())
            && !self
                .deferred
                .iter()
                .any(|b| b.tasks[b.next..].iter().any(|(p, _)| &p.id == id))
    }
    pub fn selected_slot_part<'a>(
        &'a self,
        parent: &GeneratedId,
        source: &SourceId,
        reference: &'a WorldReference,
    ) -> Result<Option<&'a SourceId>, ReferenceError> {
        for content in self.instances() {
            if let Some((_, _, part)) = content
                .inputs
                .slot_parts
                .iter()
                .find(|(p, s, _)| p == parent && s == source)
            {
                return Ok(Some(part));
            }
        }
        optional(self.rules.get(reference, source)?.fields, "part")
    }
    pub fn instances(&self) -> impl Iterator<Item = &SpawnedContent> {
        self.cursors
            .iter()
            .filter_map(|c| c.root.as_ref())
            .chain(self.active.iter().map(|c| &c.content))
    }
    pub fn appearance(&self, id: &GeneratedId) -> &[super::NodeAppearance] {
        self.active
            .iter()
            .find(|c| &c.content.placement.id == id)
            .map(|c| c.content.appearance.as_slice())
            .or_else(|| {
                self.cursors
                    .iter()
                    .find(|c| &c.parent.id == id)
                    .and_then(|c| c.root.as_ref().map(|c| c.appearance.as_slice()))
            })
            .unwrap_or(&[])
    }
    pub fn contains(&self, id: &GeneratedId) -> bool {
        self.active.iter().any(|c| c.content.placement.id == *id)
    }
    pub fn retained_bytes(&self) -> usize {
        self.cursors.capacity() * std::mem::size_of::<Cursor>()
            + self
                .cursors
                .iter()
                .map(|c| {
                    placement_heap(&c.parent)
                        + c.building.file.capacity()
                        + c.inputs.retained_bytes()
                        + c.root.as_ref().map_or(0, |root| {
                            root.inputs.retained_bytes() + placement_heap(&root.placement)
                                + root.tanks.capacity() * std::mem::size_of::<(SourceId, crate::fluids::FluidContainer)>()
                                + root.tanks.iter().map(|(s, _)| s.file.capacity()).sum::<usize>()
                                + root.lights.capacity() * std::mem::size_of::<super::WorldLight>()
                                + root
                                    .lights
                                    .iter()
                                    .map(super::WorldLight::heap_bytes)
                                    .sum::<usize>()
                                + root.appearance.capacity()
                                    * std::mem::size_of::<super::NodeAppearance>()
                                + root
                                    .appearance
                                    .iter()
                                    .map(super::NodeAppearance::heap_bytes)
                                    .sum::<usize>()
                        })
                        + c.callers.capacity() * std::mem::size_of::<SourceId>()
                        + c.callers.iter().map(|s| s.file.capacity()).sum::<usize>()
                })
                .sum::<usize>()
            + self.active.capacity() * std::mem::size_of::<ContentInstance>()
            + self
                .active
                .iter()
                .map(|c| {
                    c.caps.capacity()*std::mem::size_of::<GeneratedCapVisual>() + c.caps.iter().map(|c|(c.visual.open_nodes.capacity()+c.visual.closed_nodes.capacity())*std::mem::size_of::<crate::reference::NodeId>()).sum::<usize>() + c.poses.capacity()*std::mem::size_of::<crate::vehicle::VehiclePose>() + c.mechanism_colliders.capacity()*std::mem::size_of::<ContentCollider>() + c.slots.capacity()*std::mem::size_of::<ContentSlot>() + c.slots.iter().map(|s|s.source.file.capacity()).sum::<usize>() + c.bindings.capacity() * std::mem::size_of::<ContentBinding>()
                        + c.nodes.capacity() * std::mem::size_of::<crate::reference::NodeId>()
                        + c.bindings.iter().map(|b| b.source.file.capacity()).sum::<usize>()
                        + c.content.inputs.retained_bytes() + placement_heap(&c.content.placement)
                        + c.content.tanks.capacity() * std::mem::size_of::<(SourceId, crate::fluids::FluidContainer)>()
                        + c.content.tanks.iter().map(|(s, _)| s.file.capacity()).sum::<usize>()
                        + c.content.lights.capacity() * std::mem::size_of::<super::WorldLight>()
                        + c.content
                            .lights
                            .iter()
                            .map(super::WorldLight::heap_bytes)
                            .sum::<usize>()
                        + c.content.appearance.capacity()
                            * std::mem::size_of::<super::NodeAppearance>()
                        + c.content
                            .appearance
                            .iter()
                            .map(super::NodeAppearance::heap_bytes)
                            .sum::<usize>()
                })
                .sum::<usize>()
    }
}
pub(crate) fn identity_heap(id: &GeneratedId) -> usize {
    id.generator.file.capacity()
        + id.children.capacity() * std::mem::size_of::<(SourceId, u32)>()
        + id.children
            .iter()
            .map(|(s, _)| s.file.capacity())
            .sum::<usize>()
}
pub(crate) fn placement_heap(p: &Placement) -> usize {
    identity_heap(&p.id) + p.prototype.file.capacity()
}
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f32 {
    let x = (a[0] - b[0]) as f32;
    let z = (a[2] - b[2]) as f32;
    x * x + z * z
}
