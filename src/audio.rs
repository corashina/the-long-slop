//! Source audio signals and bounded completed-action events.
use crate::reference::{AudioReference, ColliderId, ItemId, NodeId, ReferenceManifest};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, to_rapier};
use rapier3d::prelude::{Ball, QueryFilter};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug)]
pub struct AudioEvent {
    pub session: u32,
    pub sequence: u32,
    pub binding: u32,
    pub emitter: u32,
    pub position: [f32; 3],
    pub gain: f32,
    pub pitch: f32,
    pub time_seconds: f64,
    pub priority: u8,
}
#[derive(Clone, Debug)]
pub struct SensorLatch {
    pub sensed: bool,
    pub count: u32,
    pub hits: Vec<Option<usize>>,
}
impl SensorLatch {
    pub fn new(sensed: bool, count: u32, capacity: usize) -> Self {
        Self {
            sensed,
            count,
            hits: Vec::with_capacity(capacity),
        }
    }
    pub fn begin_step(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        } else {
            self.sensed = false;
            self.hits.clear();
        }
    }
    pub fn hit(&mut self) {
        self.sensed = true;
        self.count = 3;
    }
}
pub fn source_gain_db(gain: f32) -> f32 {
    gain.clamp(0.001, 1.).ln() * 20.
}
pub fn interior_transmission(total: f32, doors: usize, minimum: u32, maximum: f32) -> f32 {
    let value = if minimum != 0 && total >= minimum as f32 {
        1.
    } else {
        (total / (doors as f32).clamp(1., 100.)).clamp(0., 1.)
    };
    if maximum != 0. {
        value.min(maximum)
    } else {
        value
    }
}
#[derive(Clone, Copy, Debug)]
pub struct AudioSignal {
    pub id: u32,
    pub emitter: u32,
    pub playing: bool,
    pub gain: f32,
    pub pitch: f32,
}
pub struct AudioState {
    pub signals: Vec<AudioSignal>,
    voice_indices: HashMap<u32, usize>,
    head_offset: f32,
    emitter_anchors: HashMap<u32, (Option<NodeId>, rapier3d::math::Pose)>,
    pub reference: AudioReference,
    pub session: u32,
    pub game_seconds: f64,
    pub outside_gain: f32,
    pub current_interior: Option<usize>,
    pub sensor: SensorLatch,
    pub route_mixers: Vec<u32>,
    pub engine_constants: BTreeMap<String, f32>,
    pub radio_constants: BTreeMap<String, f32>,
    queue: [Option<AudioEvent>; 64],
    length: usize,
    sequence: u32,
    bindings: BTreeMap<u32, usize>,
    ignition: Option<usize>,
    colliders: HashMap<ColliderId, usize>,
    collider_nodes: HashMap<ColliderId, NodeId>,
    control_items: HashMap<crate::reference::MechanismId, ItemId>,
    part_usables: HashMap<ItemId, ItemId>,
    route_sensors: Vec<(Option<crate::vehicle::ColliderHandle>, SensorLatch)>,
    route_nodes: Vec<Option<NodeId>>,
    outside_mixer: u32,
    car_mixer: u32,
    sphere: Option<Ball>,
}
impl AudioState {
    pub fn new(reference: AudioReference, session: u32) -> Self {
        let outside = &reference.mixer_controls.outside;
        let sphere = outside.sensor.radius.map(Ball::new);
        let sensor = SensorLatch::new(
            outside.sensor.initial_sensed,
            outside.sensor.initial_count,
            reference.interiors.len() + 256,
        );
        Self {
            signals: reference
                .voices
                .iter()
                .map(|v| AudioSignal {
                    id: v.id,
                    emitter: v.emitter,
                    playing: false,
                    gain: v.gain,
                    pitch: v.pitch,
                })
                .collect(),
            voice_indices: reference
                .voices
                .iter()
                .enumerate()
                .map(|(i, v)| (v.id, i))
                .collect(),
            head_offset: 0.,
            emitter_anchors: HashMap::new(),
            session,
            game_seconds: 0.,
            outside_gain: outside.initial_gain,
            current_interior: None,
            sensor,
            queue: [None; 64],
            length: 0,
            sequence: 0,
            bindings: reference
                .bindings
                .iter()
                .enumerate()
                .map(|(i, b)| (b.id, i))
                .collect(),
            ignition: reference
                .bindings
                .iter()
                .position(|b| b.trigger == "ignition"),
            colliders: reference
                .interiors
                .iter()
                .enumerate()
                .map(|(i, v)| (v.collider, i))
                .collect(),
            route_mixers: reference.routes.iter().map(|r| r.main_mixer).collect(),
            outside_mixer: reference
                .mixers
                .iter()
                .find(|m| m.name == "OutSideCar")
                .map_or(0, |m| m.id),
            car_mixer: reference
                .mixers
                .iter()
                .find(|m| m.name == "CarSounds")
                .map_or(0, |m| m.id),
            engine_constants: reference.engine.constants.clone(),
            radio_constants: reference.radio.constants.clone(),
            collider_nodes: HashMap::new(),
            route_sensors: reference
                .routes
                .iter()
                .map(|r| {
                    (
                        None,
                        SensorLatch::new(r.sensor.initial_sensed, r.sensor.initial_count, 1),
                    )
                })
                .collect(),
            route_nodes: vec![None; reference.routes.len()],
            part_usables: HashMap::new(),
            control_items: HashMap::new(),
            sphere,
            reference,
        }
    }
    pub fn bind_game(
        &mut self,
        game: &ReferenceManifest,
        area: &StartingArea,
        physics: &PhysicsWorld,
    ) {
        self.head_offset = game.player.camera_height - game.player.standing_height * 0.5;
        for emitter in &self.reference.emitters {
            let world = to_rapier(
                crate::interaction::items::node_pose(game, emitter.node)
                    .expect("validated emitter node"),
            );
            let mut node = Some(emitter.node);
            while let Some(id) = node {
                if area.interaction.mechanisms.node_pose(id).is_some() {
                    break;
                }
                node = game
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .and_then(|n| n.parent);
            }
            let local = node.map_or(world, |id| {
                to_rapier(crate::interaction::items::node_pose(game, id).unwrap()).inverse() * world
            });
            self.emitter_anchors.insert(emitter.id, (node, local));
        }
        self.part_usables = game
            .parts
            .iter()
            .filter_map(|p| p.usable.map(|u| (p.item, u)))
            .collect();
        self.control_items = game.mechanisms.iter().map(|m| (m.id, m.item)).collect();
        self.collider_nodes = game.colliders.iter().map(|c| (c.id, c.node)).collect();
        self.sensor.hits.reserve(game.colliders.len());
        for (i, r) in self.reference.routes.iter().enumerate() {
            self.route_sensors[i].0 = game
                .colliders
                .iter()
                .find(|c| Some(c.node) == r.sensor.node && c.sensor)
                .and_then(|c| {
                    physics
                        .rapier()
                        .colliders
                        .iter()
                        .find(|(_, collider)| ColliderId((collider.user_data >> 64) as u32) == c.id)
                        .map(|(handle, _)| handle)
                });
        }
    }
    pub fn voice(&mut self, id: u32, playing: bool, gain: f32, pitch: f32) {
        if let Some(&index) = self.voice_indices.get(&id) {
            let signal = &mut self.signals[index];
            signal.playing = playing;
            signal.gain = gain;
            signal.pitch = pitch;
        }
    }
    pub fn event_count(&self) -> usize {
        self.length
    }
    pub fn push_event(&mut self, event: AudioEvent) {
        if event.session != self.session {
            return;
        }
        if self.length == 64 {
            let worst = (0..64)
                .max_by_key(|&i| self.queue[i].unwrap().priority)
                .unwrap();
            if event.priority >= self.queue[worst].unwrap().priority {
                return;
            }
            for i in worst..63 {
                self.queue[i] = self.queue[i + 1];
            }
            self.length -= 1;
        }
        self.queue[self.length] = Some(event);
        self.length += 1;
    }
    pub fn pop_event(&mut self) -> Option<AudioEvent> {
        if self.length == 0 {
            return None;
        }
        let event = self.queue[0];
        self.length -= 1;
        for i in 0..self.length {
            self.queue[i] = self.queue[i + 1];
        }
        self.queue[self.length] = None;
        event
    }
    pub fn reset(&mut self, session: u32) {
        self.session = session;
        self.queue.fill(None);
        self.length = 0;
        self.sequence = 0;
        self.game_seconds = 0.;
        self.outside_gain = self.reference.mixer_controls.outside.initial_gain;
        self.sensor.sensed = false;
        self.sensor.count = 0;
        self.sensor.hits.clear();
        self.current_interior = None;
        for (signal, voice) in self.signals.iter_mut().zip(&self.reference.voices) {
            signal.playing = false;
            signal.gain = voice.gain;
            signal.pitch = voice.pitch;
        }
        for (_, latch) in &mut self.route_sensors {
            latch.sensed = false;
            latch.count = 0;
            latch.hits.clear();
        }
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        for event in self.queue.iter_mut().flatten() {
            for (p, d) in event.position.iter_mut().zip(shift) {
                *p -= d;
            }
        }
    }
    pub fn binding(&self, id: u32) -> Option<&crate::reference::AudioEventBinding> {
        self.bindings.get(&id).map(|&i| &self.reference.bindings[i])
    }
    pub fn ignition_changed(&mut self, area: &StartingArea) {
        let Some(index) = self.ignition else {
            return;
        };
        let binding = &self.reference.bindings[index];
        let spatial_emitter = binding
            .voice
            .and_then(|id| self.reference.voices.iter().find(|v| v.id == id))
            .map_or(binding.emitter, |v| v.emitter);
        let emitter = self
            .reference
            .emitters
            .iter()
            .find(|e| e.id == spatial_emitter)
            .unwrap();
        let (anchor, local) = self.emitter_anchors[&emitter.id];
        let mut world = anchor
            .and_then(|node| area.interaction.mechanisms.node_pose(node))
            .map_or(local, |pose| to_rapier(pose) * local);
        if anchor.is_none() {
            world.translation -= rapier3d::math::Vector::from_array(area.origin.map(|v| v as f32));
        }
        let pose = crate::vehicle::from_rapier(&world);
        self.sequence = self.sequence.wrapping_add(1);
        let event = AudioEvent {
            session: self.session,
            sequence: self.sequence,
            binding: binding.id,
            emitter: binding.emitter,
            position: pose.position,
            gain: 1.,
            pitch: binding.pitch_range[0],
            time_seconds: self.game_seconds,
            priority: binding.priority,
        };
        self.push_event(event);
    }
    pub fn fixed_step(&mut self, area: &StartingArea, physics: &PhysicsWorld, dt: f32) {
        self.game_seconds += f64::from(dt);
        self.sensor.begin_step();
        for (index, (id, latch)) in self.route_sensors.iter_mut().enumerate() {
            latch.begin_step();
            if !latch.sensed {
                self.route_nodes[index] = None;
            }
            if let Some(handle) = *id {
                if let Some(sensor) = physics.rapier().colliders.get(handle) {
                    for (_, other) in physics
                        .queries(QueryFilter {
                            exclude_collider: Some(handle),
                            exclude_rigid_body: sensor.parent(),
                            groups: Some(sensor.collision_groups()),
                            ..QueryFilter::default()
                        })
                        .intersect_shape(*sensor.position(), sensor.shape())
                    {
                        let id = ColliderId((other.user_data >> 64) as u32);
                        self.route_nodes[index] = self.collider_nodes.get(&id).copied();
                        latch.hits.clear();
                        latch.hits.push(self.colliders.get(&id).copied());
                        latch.hit();
                    }
                }
            }
        }
        let Some(sphere) = &self.sphere else {
            return;
        };
        let settings = &self.reference.mixer_controls.outside.sensor;
        let mut pose = to_rapier(area.player.view_pose(physics));
        pose.translation += to_rapier(area.player.body_pose(physics)).rotation
            * rapier3d::math::Vector::Y
            * self.head_offset;
        pose.translation += pose.rotation
            * rapier3d::math::Vector::from_array(settings.local_position.unwrap_or([0.; 3]));
        let Some(mask) = settings.mask else {
            return;
        };
        let mut first = true;
        for (_, collider) in physics
            .queries(QueryFilter::default())
            .intersect_shape(pose, sphere)
        {
            if collider.collision_groups().memberships.bits() & mask == 0 {
                continue;
            }
            if first {
                self.sensor.hits.clear();
                first = false;
            }
            self.sensor.hits.push(
                self.colliders
                    .get(&ColliderId((collider.user_data >> 64) as u32))
                    .copied(),
            );
        }
        if !first {
            self.sensor.hit();
        }
    }
    fn control(
        &self,
        control: &crate::reference::AudioControlReference,
        area: &StartingArea,
    ) -> f32 {
        control
            .mechanism
            .and_then(|id| self.control_items.get(&id))
            .and_then(|&id| area.interaction.mechanisms.value(id, true))
            .unwrap_or(1.)
    }
    fn slot(&self, slot: crate::reference::PartSlotId, area: &StartingArea) -> f32 {
        area.interaction
            .parts
            .slot_item(slot)
            .and_then(|id| self.part_usables.get(&id))
            .and_then(|&id| area.interaction.mechanisms.value(id, true))
            .unwrap_or(1.)
    }
    pub fn interior_value(
        &self,
        index: usize,
        area: &StartingArea,
        caller: Option<usize>,
        depth: usize,
    ) -> f32 {
        if depth > self.reference.interiors.len() {
            return 0.;
        }
        let interior = &self.reference.interiors[index];
        let mut total = 0.;
        let mut count = interior.doors.len();
        if interior.removable_doors {
            count += interior.door_slots.len();
            for (i, &slot) in interior.door_slots.iter().enumerate() {
                let value = self.slot(slot, area);
                total += (2.
                    * if interior.invert_slots[i] {
                        1. - value
                    } else {
                        value
                    })
                .clamp(0., 1.);
            }
        }
        for (i, door) in interior.doors.iter().enumerate() {
            let value = self.control(door, area);
            total += (2.
                * if interior.invert_doors[i] {
                    1. - value
                } else {
                    value
                })
            .clamp(0., 1.);
        }
        if let Some(parent) = interior
            .parent
            .and_then(|id| self.reference.interiors.iter().position(|i| i.id == id))
        {
            if Some(parent) != caller {
                let value = if interior.removable_doors {
                    interior
                        .parent_slot
                        .map_or(1., |slot| self.slot(slot, area))
                } else {
                    interior
                        .parent_door
                        .as_ref()
                        .map_or(1., |door| self.control(door, area))
                };
                total += self.interior_value(parent, area, Some(index), depth + 1)
                    * (2. * value).clamp(0., 1.);
            }
        }
        interior_transmission(
            total,
            count,
            interior.minimum_doors_to_max,
            interior.maximum_transmission,
        )
    }
    pub fn update_mixers(&mut self, area: &StartingArea, dt: f32) {
        let mut current = if self.sensor.sensed {
            self.sensor.hits.first().copied().flatten()
        } else {
            None
        };
        for (ri, route) in self.reference.routes.iter().enumerate() {
            let inside = self
                .sensor
                .sensed
                .then(|| {
                    self.sensor
                        .hits
                        .iter()
                        .flatten()
                        .copied()
                        .find(|&i| route.interiors.contains(&self.reference.interiors[i].id))
                })
                .flatten();
            if let Some(interior) = inside {
                current = Some(interior);
                self.route_mixers[ri] = route.car_mixer.unwrap_or(route.main_mixer);
                continue;
            }
            let latch = &self.route_sensors[ri].1;
            let sensed = if latch.sensed {
                latch.hits.first().copied().flatten()
            } else {
                None
            };
            let same = latch.sensed
                && current.is_none_or(|p| {
                    self.route_nodes[ri] == Some(self.reference.interiors[p].node)
                        || sensed.is_some_and(|i| {
                            i == p
                                || self.reference.interiors[i].parent
                                    == Some(self.reference.interiors[p].id)
                                || self.reference.interiors[p].parent
                                    == Some(self.reference.interiors[i].id)
                                || self.reference.interiors[i].parent.is_some()
                                    && self.reference.interiors[i].parent
                                        == self.reference.interiors[p].parent
                        })
                });
            self.route_mixers[ri] = if same {
                route.main_mixer
            } else {
                route.outside_mixer
            };
        }
        self.current_interior = current;
        let settings = &self.reference.mixer_controls.outside;
        let target = if let Some(index) = current {
            let doors = self
                .sensor
                .hits
                .iter()
                .map(|hit| {
                    hit.map_or(0., |i| {
                        let v = self.interior_value(i, area, None, 0);
                        if self.reference.interiors[i].invert {
                            1. - v
                        } else {
                            v
                        }
                    })
                })
                .fold(f32::INFINITY, f32::min);
            let interior = &self.reference.interiors[index];
            settings.default_interior_gain
                + (1.
                    - if interior.use_value {
                        interior.value
                    } else {
                        settings.default_interior_gain
                    })
                    * doors
        } else {
            1.
        };
        self.outside_gain +=
            (target - self.outside_gain) * (settings.gain_smoothing * dt).clamp(0., 1.);
    }
    pub fn mixer_outputs(&self) -> [(u32, f32); 2] {
        [
            (self.outside_mixer, source_gain_db(self.outside_gain)),
            (
                self.car_mixer,
                source_gain_db(
                    self.outside_gain * self.reference.mixer_controls.outside.car_sound_multiplier,
                ),
            ),
        ]
    }
}
