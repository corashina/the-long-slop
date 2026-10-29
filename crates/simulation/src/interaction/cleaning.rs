use super::{
    items::{ItemLocation, ItemStore},
    parts::PartAttachments,
};
use crate::conditions::{ConditionStore, RepairResult};
use crate::particles::ParticleState;
use crate::reference::{
    BrushReference, BrushTool, ColliderId, ConditionColliderBinding, ConditionId, ItemId, NodeId,
    ReferenceError, ReferenceManifest, WashId,
};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};
use rapier3d::{
    math::{Pose, Rotation, Vector},
    prelude::{ColliderHandle, QueryFilter, Ray},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ToolFrame {
    pub lmb: bool,
    pub active: bool,
    pub dt: f32,
    pub view: VehiclePose,
    pub origin: [f64; 3],
}
/// The world state a tool ray reads.
#[derive(Clone, Copy)]
pub struct ToolWorld<'a> {
    pub conditions: &'a ConditionStore,
    pub items: &'a ItemStore,
    pub physics: &'a PhysicsWorld,
    pub parts: &'a PartAttachments,
}
#[derive(Clone, Copy, Debug)]
pub enum ToolOwner {
    Equipped(ItemId),
    Carried(ItemId),
}
#[derive(Clone, Copy, Debug)]
pub struct ToolSignals {
    /// None: the player brush.
    pub item: Option<ItemId>,
    pub pose: VehiclePose,
    pub force: f32,
    pub doing_work: bool,
    pub playing: bool,
    pub voice_gain: f32,
    pub emission: [f32; 2],
    pub color: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Surface(ConditionId),
    Wash(WashId),
    /// A washtriggerscript on generated content.
    Generated(WashOwner),
    Solid(ColliderHandle),
}
/// The owner of a generated washtriggerscript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WashOwner {
    /// The genie lamp item.
    Item(ItemId),
    /// A building cobweb, by its stream id.
    Cobweb(u32),
}
/// A washtriggerscript on generated content, such as the genie lamp or a cobweb.
struct GeneratedWash {
    owner: WashOwner,
    permission: u8,
    never_repair: bool,
    repair: f32,
    colliders: Vec<(ColliderId, ColliderHandle)>,
}
/// Registered generated wash triggers and the ones that got clean.
#[derive(Default)]
struct GeneratedWashes {
    washes: Vec<GeneratedWash>,
    /// GotClean: the trigger is inactive and no longer registered.
    cleaned: Vec<WashOwner>,
}
impl GeneratedWashes {
    fn register(
        &mut self,
        owner: WashOwner,
        permission: u8,
        never_repair: bool,
        colliders: Vec<(ColliderId, ColliderHandle)>,
    ) {
        self.remove(owner);
        self.washes.push(GeneratedWash {
            owner,
            permission,
            never_repair,
            repair: 0.,
            colliders,
        });
    }
    fn remove(&mut self, owner: WashOwner) {
        self.washes.retain(|w| w.owner != owner);
    }
    /// washtriggerscript.Wash. Returns None when the tool cannot wash, else whether it got clean.
    fn wash(
        &mut self,
        owner: WashOwner,
        tables: &[[bool; 7]],
        tool: u8,
        gain: f32,
    ) -> Option<bool> {
        let i = self.washes.iter().position(|w| w.owner == owner)?;
        let w = &mut self.washes[i];
        if !tables[w.permission as usize][tool as usize] {
            return None;
        }
        w.repair += gain;
        let clean = w.repair > 1. && !w.never_repair;
        if clean {
            // GotClean deactivates the trigger object.
            self.washes.swap_remove(i);
            self.cleaned.push(owner);
        }
        Some(clean)
    }
}
struct WashEffect {
    particles: Vec<ParticleState>,
    durations: Vec<f32>,
    elapsed: Option<f32>,
    emitter: [f64; 3],
}
struct Spray {
    owned: bool,
    firing: bool,
    painted: bool,
    nozzle: Pose,
    emitter: Vector,
    rotation: Rotation,
    particles: Vec<ParticleState>,
}
struct Contact {
    equipped: bool,
    hit: Option<(
        ColliderHandle,
        rapier3d::parry::query::RayIntersection,
        Target,
    )>,
    target: Option<Target>,
    collider: Option<ColliderHandle>,
    relative: Vector,
    ray_relative: Vector,
    motion_ready: bool,
    model: Pose,
    origin: [f64; 3],
    sampled: bool,
    equip: Pose,
    default: Pose,
    particles: Vec<ParticleState>,
}

use crate::save::{SaveError, SourceKey, ToolSnapshot};
#[derive(Clone, Debug)]
pub struct ToolState {
    pub item: ItemId,
    pub quantity: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub disabled: bool,
    pub valve: f32,
}
pub struct CleaningTools {
    pub states: Vec<ToolState>,
    /// Item brushes in reference order, then the player brush (`fpscontroller.pointKefe`).
    contacts: Vec<Contact>,
    /// Item brush count: the player brush index.
    brushes: usize,
    /// The player pose: the player brush root.
    player: Pose,
    sprays: Vec<Spray>,
    /// Item brushes, sprays, then the player brush.
    signals: Vec<ToolSignals>,
    bindings: HashMap<ColliderId, ConditionColliderBinding>,
    /// World part colliders and their nearest parent condition (`GetComponentInParent`).
    world_conditions: HashMap<ColliderHandle, ConditionId>,
    item_conditions: HashMap<ItemId, ConditionId>,
    texts: HashMap<ItemId, u32>,
    washes: Vec<WashEffect>,
    wash_indices: HashMap<WashId, usize>,
    wash_colliders: Vec<(ColliderHandle, WashId, bool)>,
    collider_enabled: HashMap<ColliderId, bool>,
    generated_washes: GeneratedWashes,
    /// Generated items that reserve LMB while equipped, with their LMB text.
    lmb_items: HashMap<ItemId, u32>,
}
impl CleaningTools {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        reference.conditions.validate(reference)?;
        let mut contacts = Vec::with_capacity(reference.conditions.brushes.len());
        let mut signals = Vec::with_capacity(reference.conditions.brushes.len());
        let contact = |t: &BrushTool, root: NodeId| {
            let root = to_rapier(super::items::node_pose(reference, root)?);
            // A missing position keeps the model at the tool root.
            let relative = |node: Option<NodeId>| {
                node.map_or(Ok(Pose::IDENTITY), |n| {
                    super::items::node_pose(reference, n).map(|p| root.inverse() * to_rapier(p))
                })
            };
            let equip = relative(t.model_equip)?;
            let default = relative(t.model_default)?;
            let model = t.model.map_or(Ok(root), |n| {
                super::items::node_pose(reference, n).map(to_rapier)
            })?;
            Ok::<_, ReferenceError>(Contact {
                equipped: false,
                hit: None,
                target: None,
                collider: None,
                relative: Vector::ZERO,
                ray_relative: Vector::ZERO,
                motion_ready: false,
                model,
                origin: [0.; 3],
                sampled: false,
                equip,
                default,
                particles: t
                    .particles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| ParticleState::new(p, t.source.path_id as u64 + i as u64))
                    .collect(),
            })
        };
        let signal = |item: Option<ItemId>, model: Pose| ToolSignals {
            item,
            pose: from_rapier(&model),
            force: 0.,
            doing_work: false,
            playing: false,
            voice_gain: 0.,
            emission: [0.; 2],
            color: [1.; 4],
        };
        for b in &reference.conditions.brushes {
            let item = reference.items.iter().find(|i| i.id == b.item).unwrap();
            let c = contact(&b.tool, item.node)?;
            signals.push(signal(Some(b.item), c.model));
            contacts.push(c);
        }
        let player = contact(
            &reference.conditions.player_brush,
            reference.clothing.pose.root,
        )?;
        let player_signal = signal(None, player.model);
        contacts.push(player);
        let mut sprays = Vec::with_capacity(reference.conditions.sprays.len());
        for b in &reference.conditions.sprays {
            let item = reference.items.iter().find(|i| i.id == b.item).unwrap();
            let root = to_rapier(super::items::node_pose(reference, item.node)?);
            let nozzle = root.inverse() * to_rapier(super::items::node_pose(reference, b.nozzle)?);
            let emitter = Vector::from_array(b.particles[0].local_pose.translation);
            let particles = b
                .particles
                .iter()
                .enumerate()
                .map(|(i, r)| ParticleState::new(r, b.source.path_id as u64 + i as u64))
                .collect();
            sprays.push(Spray {
                owned: false,
                firing: false,
                painted: false,
                nozzle,
                emitter,
                rotation: Rotation::from_xyzw(
                    b.particles[0].local_pose.rotation[0],
                    b.particles[0].local_pose.rotation[1],
                    b.particles[0].local_pose.rotation[2],
                    b.particles[0].local_pose.rotation[3],
                ),
                particles,
            });
            signals.push(ToolSignals {
                item: Some(b.item),
                pose: from_rapier(&(root * nozzle)),
                force: 0.,
                doing_work: false,
                playing: false,
                voice_gain: 0.,
                emission: [0.; 2],
                color: b.startup_color,
            });
        }
        signals.push(player_signal);
        Ok(Self {
            brushes: reference.conditions.brushes.len(),
            player: Pose::IDENTITY,
            contacts,
            sprays,
            signals,
            generated_washes: GeneratedWashes::default(),
            lmb_items: HashMap::new(),
            wash_colliders: Vec::with_capacity(
                reference
                    .conditions
                    .washes
                    .iter()
                    .map(|w| w.colliders.len())
                    .sum(),
            ),
            collider_enabled: reference
                .colliders
                .iter()
                .map(|c| (c.id, c.enabled))
                .collect(),
            washes: reference
                .conditions
                .washes
                .iter()
                .map(|w| WashEffect {
                    particles: w
                        .particles
                        .iter()
                        .enumerate()
                        .map(|(i, p)| ParticleState::new(p, w.source.path_id as u64 + i as u64))
                        .collect(),
                    durations: w.particles.iter().map(|p| p.duration).collect(),
                    elapsed: None,
                    emitter: super::items::node_pose(reference, w.node)
                        .expect("checked wash pose")
                        .position
                        .map(f64::from),
                })
                .collect(),
            wash_indices: reference
                .conditions
                .washes
                .iter()
                .enumerate()
                .map(|(i, w)| (w.id, i))
                .collect(),
            texts: reference
                .conditions
                .brushes
                .iter()
                .filter(|b| b.tool.show_text)
                .map(|b| (b.item, b.tool.text))
                .collect(),
            bindings: reference
                .conditions
                .collider_bindings
                .iter()
                .map(|b| (b.collider, b.clone()))
                .collect(),
            world_conditions: HashMap::new(),
            item_conditions: reference
                .conditions
                .surfaces
                .iter()
                .filter_map(|s| s.item.map(|i| (i, s.id)))
                .collect(),
            states: reference
                .conditions
                .brushes
                .iter()
                .map(|t| ToolState {
                    item: t.item,
                    quantity: None,
                    color: None,
                    disabled: t.tool.disabled,
                    valve: 0.,
                })
                .chain(reference.conditions.sprays.iter().map(|t| ToolState {
                    item: t.item,
                    quantity: Some(t.startup_quantity),
                    color: Some(t.startup_color),
                    disabled: false,
                    valve: t.startup_valve,
                }))
                .collect(),
        })
    }
    pub fn bind_collider(&mut self, id: ColliderId, handle: ColliderHandle) {
        if let Some(wash) = self.bindings.get(&id).and_then(|b| b.wash) {
            self.wash_colliders
                .push((handle, wash, self.collider_enabled[&id]));
        }
    }
    pub fn bind_world_condition(&mut self, handle: ColliderHandle, id: ConditionId) {
        self.world_conditions.insert(handle, id);
    }
    pub fn unbind_world_condition(&mut self, handle: ColliderHandle) {
        self.world_conditions.remove(&handle);
    }
    pub fn sync_wash_colliders(&self, conditions: &ConditionStore, physics: &mut PhysicsWorld) {
        let mut changed = false;
        for &(handle, wash, authored) in &self.wash_colliders {
            let active = physics.rapier().colliders[handle]
                .parent()
                .is_none_or(|body| {
                    physics.body_active(body) && physics.rapier().bodies[body].is_enabled()
                });
            physics.edit_collider(handle, |collider| {
                let enabled = authored && active && conditions.wash(wash).visible();
                if collider.is_enabled() != enabled {
                    crate::vehicle::set_collider_enabled(collider, enabled);
                    changed = true;
                }
            });
        }
        if changed {
            physics.refresh_queries();
        }
    }
    pub fn reserves_lmb(&self, equipped: Option<ItemId>, _carried: Option<ItemId>) -> bool {
        equipped.is_some_and(|item| {
            self.lmb_items.contains_key(&item)
                || self.signals[..self.brushes]
                    .iter()
                    .any(|s| s.item == Some(item))
        })
    }
    pub fn prompt(&self, item: ItemId) -> Option<u32> {
        if let Some(text) = self.lmb_items.get(&item) {
            return Some(*text);
        }
        self.get(item)
            .filter(|t| !t.disabled)
            .and_then(|_| self.texts.get(&item).copied())
    }
    /// A generated tool that uses LMB in the hand and shows `text`.
    pub fn reserve_lmb(&mut self, item: ItemId, text: Option<u32>) {
        match text {
            Some(text) => {
                self.lmb_items.insert(item, text);
            }
            None => {
                self.lmb_items.remove(&item);
            }
        }
    }
    pub fn register_generated_wash(
        &mut self,
        owner: WashOwner,
        permission: u8,
        never_repair: bool,
        colliders: Vec<(ColliderId, ColliderHandle)>,
    ) {
        self.generated_washes
            .register(owner, permission, never_repair, colliders);
    }
    pub fn remove_generated_wash(&mut self, owner: WashOwner) {
        self.generated_washes.remove(owner);
    }
    /// Generated triggers that got clean since the last call.
    pub fn take_cleaned_washes(&mut self) -> std::vec::Drain<'_, WashOwner> {
        self.generated_washes.cleaned.drain(..)
    }
    /// ginlampscript reads a positive repairValue as a rub and resets it to 0.
    pub fn take_generated_rub(&mut self, owner: ItemId) -> bool {
        let Some(w) = self
            .generated_washes
            .washes
            .iter_mut()
            .find(|w| w.owner == WashOwner::Item(owner))
        else {
            return false;
        };
        let rubbed = w.repair > 0.;
        if rubbed {
            w.repair = 0.;
        }
        rubbed
    }
    /// washtriggerscript.Wash on a generated trigger. Returns true when it is clean.
    fn wash_generated(
        &mut self,
        owner: WashOwner,
        tool: u8,
        gain: f32,
        reference: &ReferenceManifest,
    ) -> Option<bool> {
        self.generated_washes
            .wash(owner, &reference.conditions.clean_tables, tool, gain)
    }
    /// washtriggerscript.Wash on a checked trigger. Returns true when it is clean.
    fn wash_fixed(
        &mut self,
        id: WashId,
        tool: u8,
        gain: f32,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
    ) -> Option<bool> {
        let wi = self.wash_indices[&id];
        let w = &reference.conditions.washes[wi];
        if !reference.conditions.clean_tables[w.permission_index as usize][tool as usize] {
            return None;
        }
        let state = &mut conditions.washes[wi];
        state.repair_progress += gain;
        if state.repair_progress > 1. && !w.never_repair {
            state.repair_progress = 0.;
            state.active = false;
            state.on = false;
            state.destroyed = w.destroy_on_repair;
            if w.destroy_on_repair {
                self.washes[wi].elapsed = Some(0.);
            }
            return Some(true);
        }
        Some(false)
    }
    /// Physics.OverlapSphere at `point` for the first active wash trigger on `mask`.
    fn nearby_wash(
        &self,
        point: Vector,
        mask: u32,
        radius: f32,
        conditions: &ConditionStore,
        physics: &PhysicsWorld,
    ) -> Option<Target> {
        let near = |handle: ColliderHandle| {
            let collider = &physics.rapier().colliders[handle];
            if !collider.is_enabled() || collider.collision_groups().memberships.bits() & mask == 0
            {
                return false;
            }
            let projected = collider
                .shape()
                .project_point(collider.position(), point, true);
            projected.is_inside || projected.point.distance_squared(point) <= radius * radius
        };
        // Only checked wash collider links can participate in the nearby trigger search.
        for &(handle, wash, _) in &self.wash_colliders {
            if conditions.wash(wash).visible() && near(handle) {
                return Some(Target::Wash(wash));
            }
        }
        self.generated_washes
            .washes
            .iter()
            .find(|w| {
                w.colliders
                    .iter()
                    .any(|&(_, h)| physics.rapier().colliders.get(h).is_some() && near(h))
            })
            .map(|w| Target::Generated(w.owner))
    }
    /// spricniscript.Pressing target order: hit trigger, nearby trigger, slot part, condition.
    pub fn spray_target(
        &self,
        (handle, point): (ColliderHandle, Vector),
        reference: &ReferenceManifest,
        world: ToolWorld<'_>,
    ) -> Option<Target> {
        let ToolWorld {
            conditions,
            items,
            physics,
            parts,
        } = world;
        let r = &reference.small_items.spray_bottle;
        let collider = ColliderId((physics.rapier().colliders[handle].user_data >> 64) as u32);
        let selected = self.target(
            handle,
            collider,
            r.cleaning_type,
            reference,
            conditions,
            items,
            parts,
        );
        spray_order(selected, || {
            self.nearby_wash(point, r.wash_mask, r.wash_radius, conditions, physics)
        })
    }
    /// spricniscript.Fire on a target. Returns true when the splash spawns.
    pub fn spray(
        &mut self,
        target: Target,
        tool: u8,
        gain: f32,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
    ) -> bool {
        match target {
            Target::Wash(id) => self
                .wash_fixed(id, tool, gain, reference, conditions)
                .is_some(),
            Target::Generated(owner) => self.wash_generated(owner, tool, gain, reference).is_some(),
            Target::Surface(id) if conditions.can_repair(id, tool) => {
                conditions.repair(id, tool, gain);
                true
            }
            Target::Surface(_) | Target::Solid(_) => false,
        }
    }
    pub fn signals(&self) -> &[ToolSignals] {
        &self.signals
    }
    pub fn model_pose(&self, index: usize, items: &ItemStore) -> VehiclePose {
        let c = &self.contacts[index];
        if c.motion_ready {
            self.signals[index].pose
        } else {
            from_rapier(
                &(to_rapier(items.get(self.signals[index].item.unwrap()).unwrap().pose)
                    * c.default),
            )
        }
    }
    pub fn particles(&self, tool: usize, particle: usize) -> &[crate::particles::ParticleOutput] {
        self.contacts[tool].particles[particle].particles()
    }
    pub fn spray_particles(
        &self,
        tool: usize,
        particle: usize,
    ) -> &[crate::particles::ParticleOutput] {
        self.sprays[tool].particles[particle].particles()
    }
    pub fn wash_particles(
        &self,
        wash: usize,
        particle: usize,
    ) -> &[crate::particles::ParticleOutput] {
        self.washes[wash].particles[particle].particles()
    }
    pub fn reset_contact(&mut self) {
        for state in &mut self.states {
            state.valve = 0.;
        }
        for c in &mut self.contacts {
            c.equipped = false;
            c.hit = None;
            c.target = None;
            c.collider = None;
            c.sampled = false;
            for p in &mut c.particles {
                p.reset_emitter();
            }
        }
        for spray in &mut self.sprays {
            spray.owned = false;
            spray.firing = false;
            spray.painted = false;
            for p in &mut spray.particles {
                p.reset_emitter();
            }
        }
        for wash in &mut self.washes {
            for p in &mut wash.particles {
                p.reset_emitter();
            }
        }
        for s in &mut self.signals {
            s.force = 0.;
            s.doing_work = false;
            s.playing = false;
            s.voice_gain = 0.;
            s.emission = [0.; 2];
        }
    }
    fn target(
        &self,
        handle: ColliderHandle,
        collider: ColliderId,
        tool: u8,
        reference: &ReferenceManifest,
        conditions: &ConditionStore,
        items: &ItemStore,
        parts: &PartAttachments,
    ) -> Option<Target> {
        if let Some(w) = self
            .generated_washes
            .washes
            .iter()
            .find(|w| w.colliders.iter().any(|&(id, _)| id == collider))
        {
            return Some(Target::Generated(w.owner));
        }
        if let Some(id) = self.world_conditions.get(&handle) {
            return Some(Target::Surface(*id));
        }
        let b = self.bindings.get(&collider)?;
        if let Some(id) = b.wash.filter(|id| conditions.wash(*id).visible()) {
            return Some(Target::Wash(id));
        }
        let installed = b
            .installed_part_slot
            .and_then(|slot| parts.slot_item(slot))
            .or_else(|| {
                b.installed_tire_slot.and_then(|rim| {
                    items
                        .iter()
                        .find(|i| {
                            i.location == ItemLocation::Attached(rim)
                                && reference.items.iter().any(|p| {
                                    p.id == i.id
                                        && matches!(p.kind, crate::reference::ItemKind::Tire)
                                })
                        })
                        .map(|i| i.id)
                })
            });
        let condition = installed
            .and_then(|i| self.item_conditions.get(&i).copied())
            .filter(|id| conditions.can_repair(*id, tool))
            .or(b.ancestor_condition)
            .or(b.forwarded_condition);
        condition.map(Target::Surface)
    }
    fn paint_target(
        &self,
        handle: ColliderHandle,
        collider: ColliderId,
        items: &ItemStore,
        parts: &PartAttachments,
        reference: &ReferenceManifest,
    ) -> Option<ConditionId> {
        if let Some(id) = self.world_conditions.get(&handle) {
            return Some(*id);
        }
        let b = self.bindings.get(&collider)?;
        b.ancestor_condition
            .or_else(|| {
                if let Some(slot) = b.installed_part_slot {
                    return parts
                        .slot_item(slot)
                        .and_then(|i| self.item_conditions.get(&i).copied());
                }
                if let Some(rim) = b.installed_tire_slot {
                    return items
                        .iter()
                        .find(|i| {
                            i.location == ItemLocation::Attached(rim)
                                && reference.items.iter().any(|p| {
                                    p.id == i.id
                                        && matches!(p.kind, crate::reference::ItemKind::Tire)
                                })
                        })
                        .and_then(|i| self.item_conditions.get(&i.id).copied());
                }
                None
            })
            .or(b.pickup_condition)
    }
    /// The signal of brush `index`.
    fn signal_index(&self, index: usize) -> usize {
        if index < self.brushes {
            index
        } else {
            self.signals.len() - 1
        }
    }
    fn brush_disabled(&self, index: usize, reference: &ReferenceManifest) -> bool {
        match self.states.get(index).filter(|_| index < self.brushes) {
            Some(state) => state.disabled,
            None => reference.conditions.player_brush.disabled,
        }
    }
    /// Source `fpscontroller.pointKefe.Use` while the point input is held. `player` is the
    /// player pose. The brush has no item: it ignores the item brush checks.
    #[allow(clippy::too_many_arguments)]
    pub fn advance_player(
        &mut self,
        held: bool,
        player: VehiclePose,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) {
        let index = self.brushes;
        self.player = to_rapier(player);
        if !frame.active || !frame.dt.is_finite() || frame.dt <= 0. {
            return;
        }
        let frame = ToolFrame { lmb: held, ..frame };
        self.contacts[index].equipped = held;
        self.contacts[index].hit = self.brush_hit(
            index, true, frame, reference, conditions, items, physics, parts,
        );
        self.use_brush(index, frame, reference, conditions);
    }
    pub fn begin_frame(&mut self, frame: ToolFrame) {
        for contact in &mut self.contacts {
            contact.equipped = false;
            contact.hit = None;
        }
        for signal in &mut self.signals {
            signal.doing_work = false;
        }
        for spray in &mut self.sprays {
            spray.owned = false;
            spray.firing = false;
            spray.painted = false;
        }
        if !frame.active || !frame.dt.is_finite() || frame.dt <= 0. {
            self.reset_contact();
        }
    }
    pub fn advance_owner(
        &mut self,
        owner: ToolOwner,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) -> bool {
        let (item, equipped) = match owner {
            ToolOwner::Equipped(item) => (item, true),
            ToolOwner::Carried(item) => (item, false),
        };
        let reserved = self.reserves_lmb(equipped.then_some(item), (!equipped).then_some(item));
        if !frame.active || !frame.dt.is_finite() || frame.dt <= 0. {
            return reserved;
        }
        if let Some(index) = reference
            .conditions
            .brushes
            .iter()
            .position(|b| b.item == item)
        {
            self.contacts[index].equipped = equipped;
            self.contacts[index].hit = self.brush_hit(
                index, equipped, frame, reference, conditions, items, physics, parts,
            );
            if equipped {
                self.use_brush(index, frame, reference, conditions);
            } else {
                self.signals[index].doing_work = false;
            }
        }
        if let Some(index) = reference
            .conditions
            .sprays
            .iter()
            .position(|b| b.item == item)
        {
            self.advance_spray(
                index, equipped, frame, reference, conditions, items, physics, parts,
            );
        }
        reserved
    }
    fn brush_hit(
        &self,
        index: usize,
        equipped: bool,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) -> Option<(
        ColliderHandle,
        rapier3d::parry::query::RayIntersection,
        Target,
    )> {
        let view = to_rapier(frame.view);
        let ray = Ray::new(view.translation, view.rotation * -Vector::Z);
        let brushes = &reference.conditions.brushes;
        let b = brush_tool(brushes, &reference.conditions.player_brush, index);
        let own_body = brushes
            .get(index)
            .and_then(|brush| items.get(brush.item))
            .and_then(|i| i.body);
        let filter = |_: ColliderHandle, c: &rapier3d::prelude::Collider| {
            c.is_enabled()
                && (own_body.is_none() || c.parent() != own_body)
                && c.collision_groups().memberships.bits() & b.mask != 0
                && !c.shape().contains_point(c.position(), ray.origin)
        };
        let hit = if equipped && frame.lmb {
            physics
                .queries(QueryFilter::default().predicate(&filter))
                .cast_ray_and_get_normal(&ray, b.distance, true)
        } else {
            None
        };
        let mut selected = hit.and_then(|(handle, _)| {
            self.target(
                handle,
                ColliderId((physics.rapier().colliders[handle].user_data >> 64) as u32),
                b.cleaning_type,
                reference,
                conditions,
                items,
                parts,
            )
        });
        if let Some((_, hit)) = hit
            && !matches!(selected, Some(Target::Wash(_) | Target::Generated(_)))
            && !self.brush_disabled(index, reference)
            && let Some(wash) = self.nearby_wash(
                ray.point_at(hit.time_of_impact),
                b.wash_mask,
                b.wash_radius,
                conditions,
                physics,
            )
        {
            selected = Some(wash);
        }
        hit.map(|(handle, hit)| (handle, hit, selected.unwrap_or(Target::Solid(handle))))
    }
    fn use_brush(
        &mut self,
        index: usize,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
    ) {
        let Some((handle, _, target)) = self.contacts[index].hit else {
            return;
        };
        let c = &self.contacts[index];
        let si = self.signal_index(index);
        let s = &self.signals[si];
        let brushes = &reference.conditions.brushes;
        let b = brush_tool(brushes, &reference.conditions.player_brush, index);
        let disabled = self.brush_disabled(index, reference);
        // Use reads the retained force before objSlideOnColliderScript.Update samples this frame.
        if c.origin == frame.origin
            && c.sampled
            && c.target == Some(target)
            && c.collider == Some(handle)
            && s.force > b.min_force
            && !disabled
        {
            let gain = b.repair_modifier * reference.conditions.repair_rate * frame.dt;
            let tool = b.cleaning_type;
            let mut completed = false;
            match target {
                Target::Surface(id) => {
                    self.signals[si].doing_work = conditions.can_repair(id, tool);
                    completed = conditions.repair(id, tool, gain) != RepairResult::None;
                }
                Target::Wash(id) => {
                    if let Some(clean) = self.wash_fixed(id, tool, gain, reference, conditions) {
                        self.signals[si].doing_work = true;
                        completed = clean;
                    }
                }
                Target::Generated(owner) => {
                    if let Some(clean) = self.wash_generated(owner, tool, gain, reference) {
                        self.signals[si].doing_work = true;
                        completed = clean;
                    }
                }
                Target::Solid(_) => {}
            }
            if completed
                && b.can_ruin
                && let Some(brush) = brushes.get(index)
            {
                self.states[index].disabled = conditions.wear(brush.own_condition);
            }
        }
    }
    fn finish_brush(
        &mut self,
        index: usize,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) {
        let equipped = self.contacts[index].equipped;
        let hit = self.contacts[index].hit;
        let view = to_rapier(frame.view);
        let ray = Ray::new(view.translation, view.rotation * -Vector::Z);
        let brushes = &reference.conditions.brushes;
        let b = brush_tool(brushes, &reference.conditions.player_brush, index);
        let root = match brushes.get(index) {
            Some(brush) => items
                .get(brush.item)
                .map(|i| to_rapier(i.pose))
                .unwrap_or(self.contacts[index].model),
            None => self.player,
        };
        let disabled = self.brush_disabled(index, reference);
        // The player brush works while its own input is held.
        let using = equipped && (frame.lmb || index == self.brushes);
        let si = self.signal_index(index);
        let c = &mut self.contacts[index];
        let s = &mut self.signals[si];
        s.emission = [0.; 2];
        if !c.motion_ready {
            c.model = root * c.default;
            c.origin = frame.origin;
        }
        if c.origin != frame.origin {
            c.sampled = false;
            c.target = None;
            c.model.translation -= Vector::from_array(std::array::from_fn(|j| {
                (frame.origin[j] - c.origin[j]) as f32
            }));
            c.origin = frame.origin;
        }
        if let Some((handle, hit, target)) = hit {
            let target_position = physics.rapier().colliders[handle].position().translation;
            let point = ray.point_at(hit.time_of_impact) + hit.normal * b.contact_offset;
            let forward = -hit.normal;
            let head_forward = view.rotation * -Vector::Z;
            let horizontal = Vector::new(head_forward.x, 0., head_forward.z);
            let up = horizontal.normalize_or_zero();
            let right = forward.cross(up).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            let rotation = if right.length_squared() > 0. {
                Rotation::from_mat3(&rapier3d::math::Matrix::from_cols(right, up, -forward))
            } else {
                Rotation::from_rotation_arc(-Vector::Z, forward)
            };
            let reset = !c.sampled || c.target != Some(target) || c.collider != Some(handle);
            if reset {
                s.force = 0.;
            }
            c.model.translation = c
                .model
                .translation
                .lerp(point, (b.position_lerp * frame.dt).clamp(0., 1.));
            c.model.rotation = c
                .model
                .rotation
                .slerp(rotation, (b.rotation_lerp * frame.dt).clamp(0., 1.));
            let relative = if b.model.is_some() {
                c.model.translation
            } else {
                root.translation
            } - target_position;
            let ray_relative = point - target_position;
            // Port invariant: model settling cannot count as rubbing on a stationary contact ray.
            let force = if reset || ray_relative == c.ray_relative {
                0.
            } else {
                ((relative - c.relative).length() - b.minimum_displacement)
                    / (b.maximum_displacement - b.minimum_displacement)
            }
            .clamp(0., 1.);
            c.target = Some(target);
            c.collider = Some(handle);
            c.relative = relative;
            c.ray_relative = ray_relative;
            c.sampled = true;
            s.force = force;
            s.playing = b.has_sound && (b.sound_when_disabled || !disabled);
            if s.playing {
                s.voice_gain += (force * b.max_volume - s.voice_gain)
                    * (b.volume_lerp * frame.dt).clamp(0., 1.);
            } else {
                s.voice_gain = 0.;
            }
            if !disabled {
                for (pi, p) in b.particles.iter().enumerate() {
                    let first = p.role.ends_with("-particle");
                    s.emission[pi] =
                        if (first && (!s.doing_work || b.both_work)) || (!first && s.doing_work) {
                            force
                        } else {
                            0.
                        };
                }
            }
        } else {
            c.target = None;
            c.collider = None;
            c.sampled = false;
            s.force = 0.;
            let desired = if using {
                Pose::from_parts(
                    view.translation + view.rotation * Vector::new(0., -0.25, -0.75),
                    root.rotation * c.equip.rotation,
                )
            } else {
                Pose::from_parts(
                    (root * c.equip).translation,
                    (root * if equipped { c.equip } else { c.default }).rotation,
                )
            };
            c.model.translation = c.model.translation.lerp(
                desired.translation,
                (b.position_lerp * frame.dt).clamp(0., 1.),
            );
            c.model.rotation = c
                .model
                .rotation
                .slerp(desired.rotation, (b.rotation_lerp * frame.dt).clamp(0., 1.));
            if using {
                s.voice_gain += (0. - s.voice_gain) * (b.volume_lerp * frame.dt).clamp(0., 1.);
            } else {
                s.playing = false;
                s.voice_gain = 0.;
            }
        }
        c.motion_ready = true;
        s.pose = from_rapier(&c.model);
    }
    pub fn finish_frame(
        &mut self,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        _conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        _parts: &PartAttachments,
    ) {
        if !frame.active || !frame.dt.is_finite() || frame.dt <= 0. {
            return;
        }
        for index in 0..self.contacts.len() {
            self.finish_brush(index, frame, reference, items, physics);
            let s = &self.signals[self.signal_index(index)];
            let c = &mut self.contacts[index];
            for (pi, p) in c.particles.iter_mut().enumerate() {
                p.rotation = c.model.rotation;
                p.set_origin(frame.origin);
                p.advance(
                    frame.dt,
                    std::array::from_fn(|j| frame.origin[j] + f64::from(c.model.translation[j])),
                    s.emission[pi],
                    physics,
                );
            }
        }
        self.finish_sprays(frame, reference, items, physics);
        for effect in &mut self.washes {
            if let Some(elapsed) = effect.elapsed.as_mut() {
                for (index, p) in effect.particles.iter_mut().enumerate() {
                    p.set_origin(frame.origin);
                    p.advance(
                        frame.dt,
                        effect.emitter,
                        if *elapsed < effect.durations[index] {
                            1.
                        } else {
                            0.
                        },
                        physics,
                    );
                }
                *elapsed += frame.dt;
            }
        }
    }
    fn advance_spray(
        &mut self,
        index: usize,
        equipped: bool,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) {
        let offset = self.brushes;
        let b = &reference.conditions.sprays[index];
        let root = to_rapier(items.get(b.item).expect("checked spray owner").pose);
        let nozzle = root * self.sprays[index].nozzle;
        let emitter = root * self.sprays[index].emitter;
        let firing = frame.lmb;
        self.sprays[index].owned = true;
        self.sprays[index].firing |= firing;
        let ti = offset + index;
        let color = self.states[ti].color.expect("spray color");
        let mut rotation = root.rotation * self.sprays[index].rotation;
        let mut painted = false;
        if firing {
            let state = &mut self.states[ti];
            state.valve += (1. - state.valve) * (b.valve_speed * frame.dt).clamp(0., 1.);
            let quantity = state.quantity.as_mut().expect("spray quantity");
            if *quantity > 0. {
                // Fire checks ammo, consumes it, then paints even on the last shot.
                *quantity -= (b.paint_loss * state.valve * frame.dt).clamp(0., *quantity);
                rotation = nozzle.rotation;
                let direction = if equipped {
                    to_rapier(frame.view)
                } else {
                    nozzle
                };
                let ray = Ray::new(direction.translation, direction.rotation * -Vector::Z);
                let own = items.get(b.item).and_then(|i| i.body);
                let filter = |_: ColliderHandle, c: &rapier3d::prelude::Collider| {
                    c.is_enabled()
                        && (own.is_none() || c.parent() != own)
                        && c.collision_groups().memberships.bits() & b.mask != 0
                        && !c.shape().contains_point(c.position(), ray.origin)
                };
                if let Some((handle, hit)) = physics
                    .queries(QueryFilter::default().predicate(&filter))
                    .cast_ray_and_get_normal(&ray, b.distance, true)
                {
                    let id =
                        ColliderId((physics.rapier().colliders[handle].user_data >> 64) as u32);
                    if let Some(target) = self.paint_target(handle, id, items, parts, reference) {
                        let previous = conditions.get(target).color;
                        let falloff =
                            ((hit.time_of_impact - b.distance) / (1. - b.distance)).clamp(0., 1.);
                        let factor = (b.color_lerp * frame.dt * falloff).clamp(0., 1.);
                        painted = conditions.paint(
                            target,
                            std::array::from_fn(|j| {
                                previous[j] + (color[j] - previous[j]) * factor
                            }),
                        );
                    }
                    let forward = (ray.point_at(hit.time_of_impact) - emitter).normalize_or_zero();
                    let right = forward
                        .cross(nozzle.rotation * Vector::Y)
                        .normalize_or_zero();
                    if right.length_squared() > 0. {
                        rotation = Rotation::from_mat3(&rapier3d::math::Matrix::from_cols(
                            right,
                            right.cross(forward),
                            -forward,
                        ));
                    }
                }
            }
        }
        self.sprays[index].rotation = root.rotation.inverse() * rotation;
        self.sprays[index].painted |= painted;
    }
    fn finish_sprays(
        &mut self,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) {
        let offset = self.brushes;
        for index in 0..self.sprays.len() {
            let b = &reference.conditions.sprays[index];
            let root = to_rapier(items.get(b.item).expect("checked spray owner").pose);
            let emitter = root * self.sprays[index].emitter;
            let rotation = root.rotation * self.sprays[index].rotation;
            let ti = offset + index;
            let state = &mut self.states[ti];
            let color = state.color.expect("spray color");
            if !self.sprays[index].owned {
                state.valve = 0.;
            }
            let signal = &mut self.signals[ti];
            // Update reads the open valve before it performs release interpolation.
            let emitting = state.quantity.unwrap() > 0. && state.valve > 0.;
            signal.pose = from_rapier(&Pose::from_parts(emitter, rotation));
            signal.color = color;
            signal.doing_work = self.sprays[index].painted;
            signal.playing = emitting && b.voice.is_some();
            signal.voice_gain = if emitting {
                state.valve * b.max_volume
            } else {
                0.
            };
            signal.emission = [if emitting { state.valve } else { 0. }, 0.];
            for particle in &mut self.sprays[index].particles {
                particle.rotation = root.rotation;
                particle.emission_rotation = Some(rotation);
                particle.set_origin(frame.origin);
                particle.start_color = Some(color);
                particle.advance(
                    frame.dt,
                    std::array::from_fn(|j| frame.origin[j] + f64::from(root.translation[j])),
                    signal.emission[0],
                    physics,
                );
            }
            if !self.sprays[index].firing {
                state.valve += (0. - state.valve) * (b.valve_speed * frame.dt).clamp(0., 1.);
            }
        }
    }
    pub fn get(&self, item: ItemId) -> Option<&ToolState> {
        self.states.iter().find(|t| t.item == item)
    }
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
    ) -> Result<Vec<ToolSnapshot>, SaveError> {
        self.states
            .iter()
            .map(|t| {
                let source = reference
                    .conditions
                    .brushes
                    .iter()
                    .find(|b| b.item == t.item)
                    .map(|b| &b.tool.source)
                    .or_else(|| {
                        reference
                            .conditions
                            .sprays
                            .iter()
                            .find(|b| b.item == t.item)
                            .map(|b| &b.source)
                    })
                    .ok_or_else(|| SaveError::InvalidLinks("Unknown tool owner".into()))?;
                Ok(ToolSnapshot {
                    source: SourceKey::from(source),
                    quantity: t.quantity,
                    color: t.color,
                    disabled: t.disabled,
                })
            })
            .collect()
    }
    pub fn restore_save(
        &mut self,
        reference: &ReferenceManifest,
        saved: &[ToolSnapshot],
    ) -> Result<(), SaveError> {
        validate_tools(reference, saved)?;
        self.reset_contact();
        for c in &mut self.contacts {
            c.motion_ready = false;
        }
        for t in &mut self.states {
            let source = reference
                .conditions
                .brushes
                .iter()
                .find(|b| b.item == t.item)
                .map(|b| &b.tool.source)
                .or_else(|| {
                    reference
                        .conditions
                        .sprays
                        .iter()
                        .find(|b| b.item == t.item)
                        .map(|b| &b.source)
                })
                .unwrap();
            let v = saved
                .iter()
                .find(|v| v.source == SourceKey::from(source))
                .unwrap();
            t.quantity = v.quantity;
            t.color = v.color;
            t.disabled = v.disabled;
            t.valve = 0.;
        }
        for signal in &mut self.signals {
            if let Some(color) = self
                .states
                .iter()
                .find(|t| Some(t.item) == signal.item)
                .and_then(|t| t.color)
            {
                signal.color = color;
            }
        }
        Ok(())
    }
}
/// The record of brush `index`: an item brush, else the player brush.
fn brush_tool<'a>(
    brushes: &'a [BrushReference],
    player: &'a BrushTool,
    index: usize,
) -> &'a BrushTool {
    brushes.get(index).map_or(player, |b| &b.tool)
}
/// The hit trigger wins, then a nearby trigger, then the hit part or condition.
fn spray_order(hit: Option<Target>, nearby: impl FnOnce() -> Option<Target>) -> Option<Target> {
    if matches!(hit, Some(Target::Wash(_) | Target::Generated(_))) {
        return hit;
    }
    nearby().or(hit)
}
pub fn validate_tools(
    reference: &ReferenceManifest,
    saved: &[ToolSnapshot],
) -> Result<(), SaveError> {
    let invalid = || SaveError::CorruptData("Invalid tool state or owner".into());
    if saved.len() != reference.conditions.brushes.len() + reference.conditions.sprays.len() {
        return Err(invalid());
    }
    for r in &reference.conditions.brushes {
        let mut matches = saved
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.tool.source));
        let v = matches.next().ok_or_else(invalid)?;
        if matches.next().is_some() || v.quantity.is_some() || v.color.is_some() {
            return Err(invalid());
        }
    }
    for r in &reference.conditions.sprays {
        let mut matches = saved
            .iter()
            .filter(|v| v.source == SourceKey::from(&r.source));
        let v = matches.next().ok_or_else(invalid)?;
        if matches.next().is_some()
            || v.disabled
            || v.quantity
                .is_none_or(|q| !q.is_finite() || !(0. ..=r.maximum_quantity).contains(&q))
            || v.color.is_none_or(|c| !c.iter().all(|v| v.is_finite()))
        {
            return Err(invalid());
        }
    }
    Ok(())
}
