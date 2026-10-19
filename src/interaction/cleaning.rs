use super::{
    items::{ItemLocation, ItemStore},
    parts::PartAttachments,
};
use crate::conditions::{ConditionStore, RepairResult};
use crate::particles::ParticleState;
use crate::reference::{
    ColliderId, ConditionColliderBinding, ConditionId, ItemId, ReferenceError, ReferenceManifest,
    WashId,
};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};
use rapier3d::{
    math::{Pose, Rotation, Vector},
    prelude::{ColliderHandle, QueryFilter, Ray},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ToolFrame {
    pub held: Option<ItemId>,
    pub lmb: bool,
    pub active: bool,
    pub dt: f32,
    pub view: VehiclePose,
    pub origin: [f64; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct ToolSignals {
    pub item: ItemId,
    pub pose: VehiclePose,
    pub force: f32,
    pub doing_work: bool,
    pub playing: bool,
    pub voice_gain: f32,
    pub emission: [f32; 2],
    pub color: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Surface(ConditionId),
    Wash(WashId),
    Solid(ColliderHandle),
}
struct WashEffect {
    particles: Vec<ParticleState>,
    durations: Vec<f32>,
    elapsed: Option<f32>,
    emitter: [f64; 3],
}
struct Spray {
    nozzle: Pose,
    emitter: Vector,
    rotation: Rotation,
    particles: Vec<ParticleState>,
}
struct Contact {
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
    contacts: Vec<Contact>,
    sprays: Vec<Spray>,
    signals: Vec<ToolSignals>,
    bindings: HashMap<ColliderId, ConditionColliderBinding>,
    item_conditions: HashMap<ItemId, ConditionId>,
    texts: HashMap<ItemId, u32>,
    washes: Vec<WashEffect>,
    wash_indices: HashMap<WashId, usize>,
    wash_colliders: Vec<(ColliderHandle, WashId, bool)>,
    collider_enabled: HashMap<ColliderId, bool>,
}
impl CleaningTools {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        reference.conditions.validate(reference)?;
        let mut contacts = Vec::with_capacity(reference.conditions.brushes.len());
        let mut signals = Vec::with_capacity(reference.conditions.brushes.len());
        for b in &reference.conditions.brushes {
            let item = reference.items.iter().find(|i| i.id == b.item).unwrap();
            let root = to_rapier(super::items::node_pose(reference, item.node)?);
            let equip =
                root.inverse() * to_rapier(super::items::node_pose(reference, b.model_equip)?);
            let default =
                root.inverse() * to_rapier(super::items::node_pose(reference, b.model_default)?);
            let model = b.model.map_or(Ok(root), |n| {
                super::items::node_pose(reference, n).map(to_rapier)
            })?;
            contacts.push(Contact {
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
                particles: b
                    .particles
                    .iter()
                    .enumerate()
                    .map(|(i, p)| ParticleState::new(p, b.source.path_id as u64 + i as u64))
                    .collect(),
            });
            signals.push(ToolSignals {
                item: b.item,
                pose: from_rapier(&model),
                force: 0.,
                doing_work: false,
                playing: false,
                voice_gain: 0.,
                emission: [0.; 2],
                color: [1.; 4],
            });
        }
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
                item: b.item,
                pose: from_rapier(&(root * nozzle)),
                force: 0.,
                doing_work: false,
                playing: false,
                voice_gain: 0.,
                emission: [0.; 2],
                color: b.startup_color,
            });
        }
        Ok(Self {
            contacts,
            sprays,
            signals,
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
                .filter(|b| b.show_text)
                .map(|b| (b.item, b.text))
                .collect(),
            bindings: reference
                .conditions
                .collider_bindings
                .iter()
                .map(|b| (b.collider, b.clone()))
                .collect(),
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
                    disabled: t.disabled,
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
    pub fn reserves_lmb(&self, held: Option<ItemId>) -> bool {
        held.is_some_and(|item| self.signals.iter().any(|s| s.item == item))
    }
    pub fn prompt(&self, item: ItemId) -> Option<u32> {
        self.get(item)
            .filter(|t| !t.disabled)
            .and_then(|_| self.texts.get(&item).copied())
    }
    pub fn signals(&self) -> &[ToolSignals] {
        &self.signals
    }
    pub fn model_pose(&self, index: usize, items: &ItemStore) -> VehiclePose {
        let c = &self.contacts[index];
        if c.motion_ready {
            self.signals[index].pose
        } else {
            from_rapier(&(to_rapier(items.get(self.signals[index].item).unwrap().pose) * c.default))
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
        for c in &mut self.contacts {
            c.target = None;
            c.collider = None;
            c.sampled = false;
            for p in &mut c.particles {
                p.reset_emitter();
            }
        }
        for spray in &mut self.sprays {
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
        collider: ColliderId,
        tool: u8,
        reference: &ReferenceManifest,
        conditions: &ConditionStore,
        items: &ItemStore,
        parts: &PartAttachments,
    ) -> Option<Target> {
        let b = self.bindings.get(&collider)?;
        if let Some(id) = b.wash.filter(|id| conditions.wash(*id).visible()) {
            return Some(Target::Wash(id));
        }
        let installed = b
            .installed_part_slot
            .and_then(|slot| parts.slot_item(slot))
            .or_else(|| {
                b.installed_wheel_slot.and_then(|slot| {
                    items
                        .iter()
                        .find(|i| i.location == ItemLocation::WheelSlot(slot))
                        .map(|i| i.id)
                })
            })
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
        collider: ColliderId,
        items: &ItemStore,
        parts: &PartAttachments,
        reference: &ReferenceManifest,
    ) -> Option<ConditionId> {
        let b = self.bindings.get(&collider)?;
        b.ancestor_condition
            .or_else(|| {
                if let Some(slot) = b.installed_part_slot {
                    return parts
                        .slot_item(slot)
                        .and_then(|i| self.item_conditions.get(&i).copied());
                }
                if let Some(slot) = b.installed_wheel_slot {
                    return items
                        .iter()
                        .find(|i| i.location == ItemLocation::WheelSlot(slot))
                        .and_then(|i| self.item_conditions.get(&i.id).copied());
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
    pub fn advance(
        &mut self,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) -> bool {
        let reserved = self.reserves_lmb(frame.held);
        if !frame.active || !frame.dt.is_finite() || frame.dt <= 0. {
            self.reset_contact();
            return reserved;
        }
        let view = to_rapier(frame.view);
        let ray = Ray::new(view.translation, view.rotation * -Vector::Z);
        for index in 0..self.contacts.len() {
            let b = &reference.conditions.brushes[index];
            let held = frame.held == Some(b.item);
            let root = items
                .get(b.item)
                .map(|i| to_rapier(i.pose))
                .unwrap_or(self.contacts[index].model);
            let own_body = items.get(b.item).and_then(|i| i.body);
            let filter = |_: ColliderHandle, c: &rapier3d::prelude::Collider| {
                c.is_enabled()
                    && (own_body.is_none() || c.parent() != own_body)
                    && c.collision_groups().memberships.bits() & b.mask != 0
                    && !c.shape().contains_point(c.position(), ray.origin)
            };
            let hit = if held && frame.lmb {
                physics
                    .queries(QueryFilter::default().predicate(&filter))
                    .cast_ray_and_get_normal(&ray, b.distance, true)
            } else {
                None
            };
            let mut selected = hit.and_then(|(handle, _)| {
                self.target(
                    ColliderId((physics.rapier().colliders[handle].user_data >> 64) as u32),
                    b.cleaning_type,
                    reference,
                    conditions,
                    items,
                    parts,
                )
            });
            if let Some((_, hit)) = hit {
                if !matches!(selected, Some(Target::Wash(_))) && !self.states[index].disabled {
                    let point = ray.point_at(hit.time_of_impact);
                    // Only checked wash collider links can participate in the nearby trigger search.
                    for &(handle, wash, _) in &self.wash_colliders {
                        let collider = &physics.rapier().colliders[handle];
                        if !collider.is_enabled()
                            || collider.collision_groups().memberships.bits() & b.wash_mask == 0
                            || !conditions.wash(wash).visible()
                        {
                            continue;
                        }
                        let projected =
                            collider
                                .shape()
                                .project_point(collider.position(), point, true);
                        if projected.is_inside
                            || projected.point.distance_squared(point)
                                <= b.wash_radius * b.wash_radius
                        {
                            selected = Some(Target::Wash(wash));
                            break;
                        }
                    }
                }
            }
            let c = &mut self.contacts[index];
            let s = &mut self.signals[index];
            s.doing_work = false;
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
            if let Some((handle, hit)) = hit {
                let target = selected.unwrap_or(Target::Solid(handle));
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
                // Use reads the retained force before objSlideOnColliderScript.Update samples this frame.
                if !reset && s.force > b.min_force && !self.states[index].disabled {
                    let gain = b.repair_modifier * reference.conditions.repair_rate * frame.dt;
                    let mut completed = false;
                    match target {
                        Target::Surface(id) => {
                            s.doing_work = conditions.can_repair(id, b.cleaning_type);
                            completed =
                                conditions.repair(id, b.cleaning_type, gain) != RepairResult::None;
                        }
                        Target::Wash(id) => {
                            let wi = self.wash_indices[&id];
                            let w = &reference.conditions.washes[wi];
                            if reference.conditions.clean_tables[w.permission_index as usize]
                                [b.cleaning_type as usize]
                            {
                                s.doing_work = true;
                                let state = &mut conditions.washes[wi];
                                state.repair_progress += gain;
                                if state.repair_progress > 1. && !w.never_repair {
                                    state.repair_progress = 0.;
                                    state.active = false;
                                    state.on = false;
                                    state.destroyed = w.destroy_on_repair;
                                    completed = true;
                                    if w.destroy_on_repair {
                                        let effect = &mut self.washes[wi];
                                        effect.elapsed = Some(0.);
                                    }
                                }
                            }
                        }
                        Target::Solid(_) => {}
                    }
                    if completed && b.can_ruin {
                        self.states[index].disabled = conditions.wear(b.own_condition);
                    }
                }
                c.target = Some(target);
                c.collider = Some(handle);
                c.relative = relative;
                c.ray_relative = ray_relative;
                c.sampled = true;
                s.force = force;
                s.playing = b.has_sound && (b.sound_when_disabled || !self.states[index].disabled);
                if s.playing {
                    s.voice_gain += (force * b.max_volume - s.voice_gain)
                        * (b.volume_lerp * frame.dt).clamp(0., 1.);
                } else {
                    s.voice_gain = 0.;
                }
                if !self.states[index].disabled {
                    for (pi, p) in b.particles.iter().enumerate() {
                        let first = p.role.ends_with("-particle");
                        s.emission[pi] = if (first && (!s.doing_work || b.both_work))
                            || (!first && s.doing_work)
                        {
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
                let desired = if held && frame.lmb {
                    Pose::from_parts(
                        view.translation + view.rotation * Vector::new(0., -0.25, -0.75),
                        root.rotation * c.equip.rotation,
                    )
                } else {
                    Pose::from_parts(
                        (root * c.equip).translation,
                        (root * if held { c.equip } else { c.default }).rotation,
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
                if held && frame.lmb {
                    s.voice_gain += (0. - s.voice_gain) * (b.volume_lerp * frame.dt).clamp(0., 1.);
                } else {
                    s.playing = false;
                    s.voice_gain = 0.;
                }
            }
            c.motion_ready = true;
            s.pose = from_rapier(&c.model);
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
        self.advance_sprays(frame, reference, conditions, items, physics, parts);
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
        reserved
    }
    fn advance_sprays(
        &mut self,
        frame: ToolFrame,
        reference: &ReferenceManifest,
        conditions: &mut ConditionStore,
        items: &ItemStore,
        physics: &PhysicsWorld,
        parts: &PartAttachments,
    ) {
        let offset = self.contacts.len();
        for index in 0..self.sprays.len() {
            let b = &reference.conditions.sprays[index];
            let root = to_rapier(items.get(b.item).expect("checked spray owner").pose);
            let nozzle = root * self.sprays[index].nozzle;
            let emitter = root * self.sprays[index].emitter;
            let firing = frame.held == Some(b.item) && frame.lmb;
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
                    let ray = Ray::new(nozzle.translation, nozzle.rotation * -Vector::Z);
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
                        if let Some(target) = self.paint_target(id, items, parts, reference) {
                            let previous = conditions.get(target).color;
                            let falloff = ((hit.time_of_impact - b.distance) / (1. - b.distance))
                                .clamp(0., 1.);
                            let factor = (b.color_lerp * frame.dt * falloff).clamp(0., 1.);
                            painted = conditions.paint(
                                target,
                                std::array::from_fn(|j| {
                                    previous[j] + (color[j] - previous[j]) * factor
                                }),
                            );
                        }
                        let forward =
                            (ray.point_at(hit.time_of_impact) - emitter).normalize_or_zero();
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
            let state = &mut self.states[ti];
            let signal = &mut self.signals[ti];
            // Update reads the open valve before it performs release interpolation.
            let emitting = state.quantity.unwrap() > 0. && state.valve > 0.;
            signal.pose = from_rapier(&Pose::from_parts(emitter, rotation));
            signal.color = color;
            signal.doing_work = painted;
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
            if !firing {
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
                    .map(|b| &b.source)
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
                .map(|b| &b.source)
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
                .find(|t| t.item == signal.item)
                .and_then(|t| t.color)
            {
                signal.color = color;
            }
        }
        Ok(())
    }
}
pub(crate) fn validate_tools(
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
            .filter(|v| v.source == SourceKey::from(&r.source));
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
