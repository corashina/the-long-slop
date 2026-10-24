//! Source audio signals and bounded completed-action events.
use crate::interaction::{InteractionAction, WheelAction};
use crate::reference::{AudioReference, ColliderId, ItemId, NodeId, ReferenceManifest};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, to_rapier};
use rapier3d::prelude::{Ball, QueryFilter};
use std::collections::{BTreeMap, HashMap};

pub mod ambience;
pub mod footsteps;
pub mod impacts;

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
/// Browser emitter ids of portable radio voice slots: base + slot.
pub const PORTABLE_RADIO_EMITTER: u32 = 0x5000_0000;
pub const PORTABLE_RADIO_SLOTS: usize = 4;
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
    pub looping: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct VehicleAudioInput {
    pub rpm: f32,
    pub ignition: bool,
    pub running: bool,
    pub starter: bool,
    pub horn: bool,
    pub ambient_temperature: f32,
    pub electronics_output: f32,
}
fn xorshift(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}
/// A full queue drops its last event with the highest priority number for a lower one.
fn insert_event(queue: &mut [Option<AudioEvent>; 64], length: &mut usize, event: AudioEvent) {
    if *length == 64 {
        let worst = (0..64).max_by_key(|&i| queue[i].unwrap().priority).unwrap();
        if event.priority >= queue[worst].unwrap().priority {
            return;
        }
        queue.copy_within(worst + 1.., worst);
        *length -= 1;
    }
    queue[*length] = Some(event);
    *length += 1;
}
fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    ((value - a) / (b - a)).clamp(0., 1.)
}
struct MechanismSound {
    item: ItemId,
    index: usize,
    previous: [f32; 2],
    turn: u8,
    limits: [[f32; 2]; 2],
    slide: bool,
    remaining: f32,
    last_slide_time: f64,
}
struct FluidSound {
    opening: crate::reference::OpeningId,
    index: usize,
    latch: u32,
    gain: f32,
}
pub struct AudioState {
    pub survival_events: Vec<SurvivalAudioEvent>,
    pub survival_loops: Vec<SurvivalAudioEvent>,
    pub sleep_radio_advanced: bool,
    pub vehicle_index: usize,
    pub signals: Vec<AudioSignal>,
    crafted: HashMap<ItemId, u32>,
    lamp_controls: HashMap<crate::reference::LightControlId, (usize, bool)>,
    lamp_voices: Vec<(crate::reference::LightControlId, u32, bool)>,
    mechanism_sounds: Vec<MechanismSound>,
    mechanism_items: HashMap<ItemId, usize>,
    fluid_sounds: Vec<FluidSound>,
    fluid_openings: HashMap<crate::reference::OpeningId, usize>,
    positions: HashMap<u32, [f32; 3]>,
    removed: std::collections::HashSet<u32>,
    random: u32,
    pub starter_enabled: bool,
    horn_gain: f32,
    last_running: bool,
    engine_position: [f32; 3],
    origin_shift: [f32; 3],
    engine_formula: [f32; 11],
    voice_indices: HashMap<u32, usize>,
    emitter_anchors: HashMap<u32, (Option<NodeId>, rapier3d::math::Pose)>,
    engine_item: Option<ItemId>,
    engine_emitters: Vec<(u32, rapier3d::math::Pose)>,
    pub reference: AudioReference,
    pub session: u32,
    pub game_seconds: f64,
    pub outside_gain: f32,
    pub current_interior: Option<usize>,
    pub sensor: SensorLatch,
    pub route_mixers: Vec<u32>,
    /// Vehicle index of each car route, for car hit sounds.
    pub route_vehicles: Vec<Option<u32>>,
    pub engine_constants: BTreeMap<String, f32>,
    /// The Lada radio's controls.
    pub radio: crate::simulation::radio::RadioState,
    pub broadcasts: crate::simulation::radio::RadioBroadcasts,
    /// Portable radios holding the browser's voice slots.
    pub portable_radio_slots: [Option<ItemId>; PORTABLE_RADIO_SLOTS],
    pub radio_frame: crate::simulation::radio::RadioFrame,
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
    impacts: impacts::ImpactState,
    rocks: ambience::RockAmbience,
    footsteps: footsteps::FootstepState,
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
        let c = &reference.engine.constants;
        let engine_formula = [
            c["SMinAlapRpm"],
            c["SMaxAlapRpm"],
            c["SMinMotorRpm"],
            c["SMaxMotorRpm"],
            c["SMaxAlapSound"],
            c["SMaxMotorSound"],
            c["minAlapPitch"],
            c["maxAlapPitch"],
            c["minMotorPitch"],
            c["maxMotorPitch"],
            c["maxRpm"],
        ];
        Self {
            survival_events: Vec::with_capacity(512),
            survival_loops: Vec::with_capacity(64),
            sleep_radio_advanced: false,
            crafted: HashMap::new(),
            lamp_controls: HashMap::new(),
            lamp_voices: Vec::new(),
            mechanism_sounds: Vec::new(),
            mechanism_items: HashMap::new(),
            fluid_sounds: Vec::new(),
            fluid_openings: HashMap::new(),
            positions: reference.emitters.iter().map(|e| (e.id, [0.; 3])).collect(),
            removed: std::collections::HashSet::with_capacity(reference.emitters.len()),
            random: session.wrapping_add(0x9e3779b9),
            starter_enabled: false,
            horn_gain: 0.,
            last_running: false,
            vehicle_index: 0,
            engine_position: [0.; 3],
            origin_shift: [0.; 3],
            engine_formula,
            signals: reference
                .voices
                .iter()
                .map(|v| AudioSignal {
                    id: v.id,
                    emitter: v.emitter,
                    playing: false,
                    gain: v.gain,
                    pitch: v.pitch,
                    looping: v.r#loop,
                })
                .collect(),
            voice_indices: reference
                .voices
                .iter()
                .enumerate()
                .map(|(i, v)| (v.id, i))
                .collect(),
            emitter_anchors: HashMap::new(),
            engine_item: None,
            engine_emitters: Vec::new(),
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
            route_vehicles: vec![None; reference.routes.len()],
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
            radio: crate::simulation::radio::RadioState::lada(&reference.radio),
            broadcasts: crate::simulation::radio::RadioBroadcasts::new(
                &reference.radio,
                &reference.clips,
                [0, 0],
            ),
            radio_frame: crate::simulation::radio::RadioFrame::default(),
            portable_radio_slots: [None; PORTABLE_RADIO_SLOTS],
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
            impacts: impacts::ImpactState::default(),
            rocks: ambience::RockAmbience::default(),
            footsteps: footsteps::FootstepState::default(),
            reference,
        }
    }
    pub fn bind_game(
        &mut self,
        game: &ReferenceManifest,
        area: &StartingArea,
        physics: &PhysicsWorld,
    ) {
        self.vehicle_index = game
            .vehicles
            .iter()
            .position(|v| v.engine.source == self.reference.engine.source)
            .expect("validated audio vehicle");
        let engine_item = game.vehicles[self.vehicle_index].engine.item;
        let engine_node = game
            .items
            .iter()
            .find(|item| item.id == engine_item)
            .unwrap()
            .node;
        let engine_pose =
            to_rapier(crate::interaction::items::node_pose(game, engine_node).unwrap());
        self.engine_item = Some(engine_item);
        self.engine_emitters.clear();
        self.emitter_anchors.clear();
        for emitter in &self.reference.emitters {
            let world = to_rapier(
                crate::interaction::items::node_pose(game, emitter.node)
                    .expect("validated emitter node"),
            );
            if emitter.item == Some(engine_item) {
                self.engine_emitters
                    .push((emitter.id, engine_pose.inverse() * world));
                continue;
            }
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
        self.lamp_controls = self
            .reference
            .lamps
            .iter()
            .enumerate()
            .filter_map(|(index, l)| {
                game.lighting
                    .controllers
                    .iter()
                    .find(|c| c.id == l.control)
                    .map(|c| (l.control, (index, c.startup_on)))
            })
            .collect();
        self.lamp_voices.clear();
        for lamp in &self.reference.lamps {
            let c = game
                .lighting
                .controllers
                .iter()
                .find(|c| c.id == lamp.control)
                .unwrap();
            for &id in &lamp.voices {
                let voice = &self.reference.voices[self.voice_indices[&id]];
                let node = self
                    .reference
                    .emitters
                    .iter()
                    .find(|e| e.id == voice.emitter)
                    .unwrap()
                    .node;
                let state = c.states.iter().find(|s| {
                    s.renderers.iter().any(|&renderer| {
                        let mut cursor = Some(renderer);
                        while let Some(n) = cursor {
                            if n == node {
                                return true;
                            }
                            cursor = game.nodes.iter().find(|v| v.id == n).and_then(|v| v.parent);
                        }
                        false
                    })
                });
                let enabled = state.is_some_and(|s| !s.broken)
                    && (c.electronics_source.is_none()
                        || c.electronics_output > c.electronics_threshold);
                self.lamp_voices.push((lamp.control, id, enabled));
                self.signals[self.voice_indices[&id]].playing = c.startup_on && enabled;
            }
        }
        self.crafted = self
            .reference
            .bindings
            .iter()
            .filter(|b| b.trigger == "crafted")
            .filter_map(|b| {
                game.items
                    .iter()
                    .find(|i| i.pickup.as_ref().is_some_and(|p| p.source == b.source))
                    .map(|i| (i.id, b.id))
            })
            .collect();
        self.mechanism_sounds = self
            .reference
            .mechanisms
            .iter()
            .enumerate()
            .filter_map(|(index, a)| {
                let m = game.mechanisms.iter().find(|m| m.source == a.source)?;
                let (previous, turn) = area.interaction.mechanisms.audio_values(m.item)?;
                Some(MechanismSound {
                    item: m.item,
                    index,
                    previous,
                    turn,
                    limits: m
                        .rotation
                        .as_ref()
                        .map_or([[0., 1.], [0., 0.]], |r| r.limits_degrees),
                    slide: m.slide.is_some(),
                    remaining: 0.,
                    last_slide_time: 0.,
                })
            })
            .collect();
        for a in &self.reference.mechanisms {
            if let Some(id) = a.movement_voice {
                self.signals[self.voice_indices[&id]].gain = 0.;
            }
        }
        self.mechanism_items = self
            .mechanism_sounds
            .iter()
            .enumerate()
            .map(|(i, m)| (m.item, i))
            .collect();
        self.fluid_sounds = self
            .reference
            .fluids
            .iter()
            .enumerate()
            .filter_map(|(index, a)| {
                let o = game.openings.iter().find(|o| o.source == a.source)?;
                Some(FluidSound {
                    opening: o.id,
                    index,
                    latch: 0,
                    gain: 0.,
                })
            })
            .collect();
        self.fluid_openings = self
            .fluid_sounds
            .iter()
            .enumerate()
            .map(|(i, f)| (f.opening, i))
            .collect();
        self.part_usables = game
            .parts
            .iter()
            .filter_map(|p| p.usable.map(|u| (p.item, u)))
            .collect();
        self.control_items = game.mechanisms.iter().map(|m| (m.id, m.item)).collect();
        self.collider_nodes = game.colliders.iter().map(|c| (c.id, c.node)).collect();
        self.impacts.bind(game);
        for (route, vehicle) in self.reference.routes.iter().zip(&mut self.route_vehicles) {
            let inside = |root| {
                route.interiors.iter().any(|id| {
                    self.reference.interiors.iter().any(|i| {
                        i.id == *id && crate::reference::clothing::descendant(game, i.node, root)
                    })
                })
            };
            *vehicle = route
                .has_car
                .then(|| game.vehicles.iter().position(|v| inside(v.root)))
                .flatten()
                .map(|v| v as u32);
        }
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
    pub fn sync_lamps(
        &mut self,
        reference: &crate::reference::LightingRecord,
        lighting: &crate::lighting::LightingState,
    ) {
        for (control, (_, on)) in &mut self.lamp_controls {
            let index = reference
                .controllers
                .iter()
                .position(|c| c.id == *control)
                .expect("validated lamp controller");
            *on = lighting.controllers[index].on;
        }
        for &(control, id, enabled) in &self.lamp_voices {
            let signal = &mut self.signals[self.voice_indices[&id]];
            signal.playing = self.lamp_controls[&control].1
                && enabled
                && !self.removed.contains(&signal.emitter);
        }
    }
    pub fn voice(&mut self, id: u32, playing: bool, gain: f32, pitch: f32) {
        if let Some(&index) = self.voice_indices.get(&id) {
            let signal = &mut self.signals[index];
            signal.playing = playing && !self.removed.contains(&signal.emitter);
            signal.gain = gain;
            signal.pitch = pitch;
        }
    }
    fn sample(&mut self) -> f32 {
        xorshift(&mut self.random)
    }
    fn emit(&mut self, id: u32, gain: f32, simulation_seconds: f64) {
        self.emit_at(id, gain, simulation_seconds, None);
    }
    fn emit_at(&mut self, id: u32, gain: f32, simulation_seconds: f64, at: Option<[f32; 3]>) {
        let Some(&index) = self.bindings.get(&id) else {
            return;
        };
        let range = self.reference.bindings[index].pitch_range;
        let pitch = range[0] + (range[1] - range[0]) * self.sample();
        let b = &self.reference.bindings[index];
        if self.removed.contains(&b.emitter) {
            return;
        }
        self.sequence = self.sequence.wrapping_add(1);
        let position = if let Some(at) = at {
            at
        } else if b.trigger == "radio_power_band" {
            let voice = &self.reference.voices[self.voice_indices[&self.reference.radio.voices[0]]];
            self.positions
                .get(&voice.emitter)
                .copied()
                .unwrap_or([0.; 3])
        } else {
            self.positions.get(&b.emitter).copied().unwrap_or([0.; 3])
        };
        self.push_event(AudioEvent {
            session: self.session,
            sequence: self.sequence,
            binding: id,
            emitter: b.emitter,
            position,
            gain,
            pitch,
            time_seconds: simulation_seconds,
            priority: b.priority,
        });
    }
    /// Submit only completed operations. Detachment does not remove an owner.
    pub fn action(&mut self, action: InteractionAction, simulation_seconds: f64) {
        let item = match action {
            InteractionAction::LightControl(control) => {
                if let Some((index, on)) = self.lamp_controls.get_mut(&control) {
                    *on = !*on;
                    let lamp = &self.reference.lamps[*index];
                    for &(owner, id, enabled) in &self.lamp_voices {
                        if owner == control {
                            self.signals[self.voice_indices[&id]].playing = *on
                                && enabled
                                && !self
                                    .removed
                                    .contains(&self.signals[self.voice_indices[&id]].emitter);
                        }
                    }
                    let binding = if *on {
                        lamp.on_binding
                    } else {
                        lamp.off_binding
                    };
                    if let Some(id) = binding {
                        self.emit(id, 1., simulation_seconds);
                    }
                }
                None
            }
            InteractionAction::PartInstall(item)
            | InteractionAction::PartRemove(item)
            | InteractionAction::Wheel(WheelAction::Remove(item))
            | InteractionAction::Wheel(WheelAction::Install { item, .. }) => Some(item),
            InteractionAction::Wheel(WheelAction::Join { tire, .. }) => Some(tire),
            _ => None,
        };
        if let Some(id) = item.and_then(|item| self.crafted.get(&item)).copied() {
            self.emit(id, 1., simulation_seconds);
        }
    }
    pub fn light_switch(&mut self) {
        if let Some(b) = self
            .reference
            .bindings
            .iter()
            .find(|b| b.trigger == "light_switch")
            .map(|b| b.id)
        {
            self.emit(b, 1., self.game_seconds);
        }
    }
    pub fn cap(
        &mut self,
        id: crate::reference::OpeningId,
        previous: f32,
        current: f32,
        tune: Option<f32>,
    ) {
        let Some(&i) = self.fluid_openings.get(&id) else {
            return;
        };
        let a = &self.reference.fluids[self.fluid_sounds[i].index];
        let (binding, gain) = if let Some(delta) = tune {
            if previous <= 0. || previous >= 1. || delta == 0. {
                return;
            }
            (a.tune_binding, delta.abs())
        } else if previous != current {
            (
                if current > 0. {
                    a.open_binding
                } else {
                    a.close_binding
                },
                1.,
            )
        } else {
            return;
        };
        if let Some(b) = binding {
            self.emit(b, gain, self.game_seconds);
        }
    }
    pub fn mechanism(&mut self, item: ItemId, previous: f32, current: f32, dt: f32) {
        self.mechanism_channels(
            item,
            [previous, 0.],
            [current, 0.],
            [(current - previous).abs(), 0.],
            dt,
        );
    }
    fn mechanism_channels(
        &mut self,
        item: ItemId,
        previous: [f32; 2],
        current: [f32; 2],
        requested: [f32; 2],
        dt: f32,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let Some(&i) = self.mechanism_items.get(&item) else {
            return;
        };
        let index = self.mechanism_sounds[i].index;
        let slide = self.mechanism_sounds[i].slide;
        let limits = self.mechanism_sounds[i].limits;
        let delta = [
            (current[0] - previous[0]).abs(),
            (current[1] - previous[1]).abs(),
        ];
        for axis in 0..if slide { 1 } else { 2 } {
            if delta[axis] == 0. {
                continue;
            }
            let [min, max] = limits[axis];
            for kind in 0..4 {
                let trigger = ["startPosEnd", "endPosEnd", "startPosStart", "endPosStart"][kind];
                let edge = [
                    current[axis] == min && previous[axis] != min,
                    current[axis] == max && previous[axis] != max,
                    current[axis] != min && previous[axis] == min,
                    current[axis] != max && previous[axis] == max,
                ][kind];
                if !edge {
                    continue;
                }
                for n in 0..self.reference.mechanisms[index].endpoints.len() {
                    let id = self.reference.mechanisms[index].endpoints[n];
                    if self.binding(id).is_some_and(|b| b.trigger == trigger) {
                        self.emit(id, requested[axis].abs() / dt * 0.01, self.game_seconds);
                    }
                }
            }
        }
        let a = &self.reference.mechanisms[index];
        let Some(id) = a.movement_voice else {
            return;
        };
        let difference = delta[0] + delta[1];
        if slide {
            self.mechanism_sounds[i].last_slide_time = self.game_seconds;
        }
        let target = a.audio_multiplier
            * difference
            * if slide {
                a.slide_multiplier * a.slide_gain
            } else {
                a.rotation_gain
            };
        let smoothing = (dt * a.gain_smoothing).clamp(0., 1.);
        let chance = a.squeak_chance;
        let pitch_range = a.pitch_range;
        let voice = self.voice_indices[&id];
        let gain = self.signals[voice].gain + (target - self.signals[voice].gain) * smoothing;
        self.mechanism_sounds[i].remaining = if difference == 0. {
            0.
        } else {
            (self.mechanism_sounds[i].remaining - dt).max(0.)
        };
        let mut playing = difference > 0. && self.mechanism_sounds[i].remaining > 0.;
        let mut pitch = self.signals[voice].pitch;
        if difference > 0. && !playing && ((!slide && delta[0] > 0.) || self.sample() > chance) {
            playing = true;
            pitch = pitch_range[0] + (pitch_range[1] - pitch_range[0]) * self.sample();
            let clip = self.reference.voices[voice].clip.unwrap();
            let duration = self
                .reference
                .clips
                .iter()
                .find(|c| c.id == clip)
                .unwrap()
                .source_duration_seconds as f32;
            self.mechanism_sounds[i].remaining = duration / pitch;
        }
        self.voice(id, playing, gain.clamp(0., 1.), pitch);
    }
    fn fluid(&mut self, i: usize, flow: Option<(bool, f32)>, fill_called: bool, empty_pour: bool) {
        let a = &self.reference.fluids[self.fluid_sounds[i].index];
        let Some(id) = a.fill_voice else {
            return;
        };
        if fill_called {
            self.signals[self.voice_indices[&id]].looping = a.fill_loop;
        }
        if let Some((fill, value)) = flow {
            if fill {
                self.signals[self.voice_indices[&id]].looping = a.fill_loop;
            }
            self.fluid_sounds[i].gain = if fill {
                a.max_gain * value
            } else {
                inverse_lerp(a.pour_speed_min, a.pour_speed_max, value)
            };
            if fill_called || fill {
                self.fluid_sounds[i].latch = a.fill_latch_steps;
            }
        }
        if empty_pour {
            self.fluid_sounds[i].latch = 0;
        }
        let playing = !empty_pour && (flow.is_some() || self.fluid_sounds[i].latch > 0);
        if !fill_called && self.fluid_sounds[i].latch > 0 {
            self.fluid_sounds[i].latch -= 1;
        }
        self.voice(id, playing, self.fluid_sounds[i].gain, 1.);
    }
    pub fn remove_owner(&mut self, emitter: u32) {
        self.removed.insert(emitter);
        for signal in &mut self.signals {
            if signal.emitter == emitter {
                signal.playing = false;
            }
        }
        let mut count = 0;
        for i in 0..self.length {
            if self.queue[i].is_some_and(|e| e.emitter != emitter) {
                self.queue[count] = self.queue[i];
                count += 1;
            }
        }
        self.queue[count..self.length].fill(None);
        self.length = count;
    }
    pub fn owner_alive(&self, emitter: u32) -> bool {
        !self.removed.contains(&emitter)
    }
    pub fn vehicle(&mut self, input: VehicleAudioInput, dt: f32, simulation_seconds: f64) {
        let [
            idle_min,
            idle_max,
            motor_min,
            motor_max,
            idle_gain,
            motor_gain,
            ip_min,
            ip_max,
            mp_min,
            mp_max,
            max_rpm,
        ] = self.engine_formula;
        let running = input.running;
        self.voice(
            self.reference.engine.idle_voice,
            running,
            inverse_lerp(idle_max, idle_min, input.rpm) * idle_gain,
            ip_min
                + (ip_max - ip_min)
                    * (inverse_lerp(0., idle_max, input.rpm)
                        + inverse_lerp(idle_max, idle_max * 2., input.rpm)),
        );
        self.voice(
            self.reference.engine.motor_voice,
            running,
            inverse_lerp(motor_min, motor_max, input.rpm) * motor_gain,
            mp_min
                + (mp_max - mp_min)
                    * (inverse_lerp(motor_min, max_rpm, input.rpm)
                        + inverse_lerp(max_rpm, max_rpm * 2., input.rpm)),
        );
        self.starter_enabled = input.starter;
        if self.last_running && !running {
            if let Some(id) = self.reference.engine.stop_binding {
                let b = self.binding(id).unwrap();
                let event = AudioEvent {
                    session: self.session,
                    sequence: self.sequence.wrapping_add(1),
                    binding: id,
                    emitter: b.emitter,
                    position: self.engine_position,
                    gain: 1.,
                    pitch: 1.,
                    time_seconds: simulation_seconds,
                    priority: b.priority,
                };
                self.sequence = event.sequence;
                self.push_event(event);
            }
        }
        self.last_running = running;
        let h = &self.reference.horn;
        let target = if input.horn { 1. } else { 0. };
        let step = h.attack_release_rate * dt;
        self.horn_gain += (target - self.horn_gain).clamp(-step, step);
        let pitch = h.pitch_min
            + (h.pitch_max - h.pitch_min)
                * inverse_lerp(
                    h.temperature_min,
                    h.temperature_max,
                    input.ambient_temperature,
                );
        self.voice(h.voice, self.horn_gain > 0., self.horn_gain, pitch);
    }
    pub(crate) fn restore_engine(&mut self, running: bool) {
        self.last_running = running;
        self.starter_enabled = false;
    }
    pub fn event_count(&self) -> usize {
        self.length
    }
    pub fn push_event(&mut self, event: AudioEvent) {
        if event.session != self.session || self.removed.contains(&event.emitter) {
            return;
        }
        insert_event(&mut self.queue, &mut self.length, event);
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
        self.survival_events.clear();
        self.survival_loops.clear();
        self.sleep_radio_advanced = false;
        // Controls belong to the committed session (fresh or restored); playlists restart.
        self.broadcasts = crate::simulation::radio::RadioBroadcasts::new(
            &self.reference.radio,
            &self.reference.clips,
            [0, 0],
        );
        self.radio_frame = crate::simulation::radio::RadioFrame::default();
        self.portable_radio_slots = [None; PORTABLE_RADIO_SLOTS];
        self.session = session;
        self.queue.fill(None);
        self.length = 0;
        self.sequence = 0;
        self.game_seconds = 0.;
        self.origin_shift = [0.; 3];
        self.random = session.wrapping_add(0x9e3779b9);
        self.removed.clear();
        self.impacts.reset();
        self.rocks.reset();
        self.footsteps = footsteps::FootstepState::default();
        for m in &mut self.mechanism_sounds {
            m.remaining = 0.;
        }
        for f in &mut self.fluid_sounds {
            f.latch = 0;
            f.gain = 0.;
        }
        self.starter_enabled = false;
        self.horn_gain = 0.;
        self.last_running = false;
        self.outside_gain = self.reference.mixer_controls.outside.initial_gain;
        self.sensor.sensed = false;
        self.sensor.count = 0;
        self.sensor.hits.clear();
        self.current_interior = None;
        for (signal, voice) in self.signals.iter_mut().zip(&self.reference.voices) {
            signal.playing = false;
            signal.gain = voice.gain;
            signal.pitch = voice.pitch;
            signal.looping = voice.r#loop;
        }
        for a in &self.reference.mechanisms {
            if let Some(id) = a.movement_voice {
                self.signals[self.voice_indices[&id]].gain = 0.;
            }
        }
        for (_, latch) in &mut self.route_sensors {
            latch.sensed = false;
            latch.count = 0;
            latch.hits.clear();
        }
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        for (total, delta) in self.origin_shift.iter_mut().zip(shift) {
            *total += delta;
        }
        for position in self.positions.values_mut() {
            for (p, d) in position.iter_mut().zip(shift) {
                *p -= d;
            }
        }
        for (p, d) in self.engine_position.iter_mut().zip(shift) {
            *p -= d;
        }
        for event in self.queue.iter_mut().flatten() {
            for (p, d) in event.position.iter_mut().zip(shift) {
                *p -= d;
            }
        }
    }
    pub fn take_origin_shift(&mut self) -> [f32; 3] {
        std::mem::take(&mut self.origin_shift)
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
        self.broadcasts.advance(self.game_seconds);
        self.radio_frame = self.broadcasts.frame(
            &self.radio,
            self.game_seconds,
            self.reference.radio.electronics_output,
            self.reference.radio.electronics_cutoff,
        );
        for (&id, &(anchor, local)) in &self.emitter_anchors {
            let mut world = anchor
                .and_then(|node| area.interaction.mechanisms.node_pose(node))
                .map_or(local, |pose| to_rapier(pose) * local);
            if anchor.is_none() {
                world.translation -=
                    rapier3d::math::Vector::from_array(area.origin.map(|v| v as f32));
            }
            *self.positions.get_mut(&id).unwrap() = crate::vehicle::from_rapier(&world).position;
        }
        if let Some(item) = self.engine_item {
            // Item poses already use the current world origin.
            let engine = to_rapier(area.items.get(item).expect("bound engine item").pose);
            for &(id, local) in &self.engine_emitters {
                *self.positions.get_mut(&id).unwrap() =
                    crate::vehicle::from_rapier(&(engine * local)).position;
            }
            self.engine_position = self.positions[&self.reference.engine.emitter];
        }
        for i in 0..self.mechanism_sounds.len() {
            let item = self.mechanism_sounds[i].item;
            let Some((current, turn)) = area.interaction.mechanisms.audio_values(item) else {
                continue;
            };
            let (active, requested) = area.interaction.mechanisms.audio_input(item);
            if active {
                self.mechanism_channels(
                    item,
                    self.mechanism_sounds[i].previous,
                    current,
                    requested,
                    dt,
                );
            } else if let Some(id) =
                self.reference.mechanisms[self.mechanism_sounds[i].index].movement_voice
            {
                self.mechanism_sounds[i].remaining =
                    (self.mechanism_sounds[i].remaining - dt).max(0.);
                let a = &self.reference.mechanisms[self.mechanism_sounds[i].index];
                let signal = &mut self.signals[self.voice_indices[&id]];
                if self.game_seconds - self.mechanism_sounds[i].last_slide_time
                    > f64::from(a.slide_stop_seconds)
                {
                    signal.gain *= 1. - (dt * 5.).clamp(0., 1.);
                }
                signal.playing &= self.mechanism_sounds[i].remaining > 0. && signal.gain > 0.;
            }
            if turn != self.mechanism_sounds[i].turn {
                if let Some(id) =
                    self.reference.mechanisms[self.mechanism_sounds[i].index].turn_binding
                {
                    self.emit(id, 1., self.game_seconds);
                }
            }
            self.mechanism_sounds[i].previous = current;
            self.mechanism_sounds[i].turn = turn;
        }
        for i in 0..self.fluid_sounds.len() {
            let flags = area
                .interaction
                .opening_audio_flags(self.fluid_sounds[i].opening);
            self.fluid(
                i,
                area.interaction.opening_flow(self.fluid_sounds[i].opening),
                flags.0,
                flags.1,
            );
        }
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
        let mut pose = to_rapier(area.player.view(physics).head_pose);
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
    pub fn radio_control(&mut self, control: crate::simulation::radio::RadioControl) {
        if !self.owner_alive(self.reference.radio.emitter) {
            return;
        }
        let on = self.radio.on;
        let am = self.radio.am;
        self.radio.control(control);
        if (on != self.radio.on || am != self.radio.am)
            && let Some(id) = self.reference.radio.click_binding
        {
            self.emit(id, 1., self.game_seconds);
        }
    }
    /// Keeps each playing portable radio in its slot and gives free slots to the nearest
    /// playing radios within the voices' maximum distance.
    pub fn assign_portable_radios(
        &mut self,
        listener: [f32; 3],
        radios: impl Iterator<Item = (ItemId, [f32; 3], bool)> + Clone,
    ) {
        let range = self
            .reference
            .portable_radio
            .voices
            .iter()
            .fold(0_f32, |m, v| m.max(v.max_distance));
        let audible = |position: [f32; 3]| {
            let d: f32 = (0..3).map(|i| (position[i] - listener[i]).powi(2)).sum();
            (d <= range * range).then_some(d)
        };
        for slot in &mut self.portable_radio_slots {
            if let Some(id) = *slot
                && !radios
                    .clone()
                    .any(|(item, p, on)| item == id && on && audible(p).is_some())
            {
                *slot = None;
            }
        }
        for i in 0..PORTABLE_RADIO_SLOTS {
            if self.portable_radio_slots[i].is_some() {
                continue;
            }
            let slots = self.portable_radio_slots;
            self.portable_radio_slots[i] = radios
                .clone()
                .filter(|(item, _, on)| *on && !slots.contains(&Some(*item)))
                .filter_map(|(item, p, _)| audible(p).map(|d| (item, d)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(item, _)| item);
        }
    }
    /// radioscript.TurnRadio and the band switch on a portable radio share Cturned.
    pub fn radio_click_at(&mut self, position: [f32; 3]) {
        if let Some(id) = self.reference.radio.click_binding {
            self.emit_at(id, 1., self.game_seconds, Some(position));
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioKind {
    Eat = 0,
    Burp = 1,
    Fart = 2,
    Urinate = 3,
    Sleep = 4,
    Death = 5,
    AmbientDay = 7,
    AmbientNight = 8,
    Shit = 9,
    EatCrunchy = 10,
    ItemHit = 11,
    CarHit = 12,
    RockHeat = 13,
    Footstep = 14,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioAction {
    Start = 0,
    Update = 1,
    Stop = 2,
}
#[derive(Clone, Copy, Debug)]
pub struct SurvivalAudioEvent {
    pub session: u32,
    pub emitter: u32,
    pub kind: AudioKind,
    pub action: AudioAction,
    pub clip: u32,
    pub gain: f32,
    pub pitch: f32,
    pub position: [f64; 3],
}

impl AudioState {
    pub fn survival_event(&mut self, mut event: SurvivalAudioEvent) {
        event.session = self.session;
        if self.survival_events.len() < self.survival_events.capacity() {
            self.survival_events.push(event);
        }
    }
    pub fn source_loop(&mut self, kind: AudioKind, emitter: u32, gain: f32, position: [f64; 3]) {
        let b = self
            .reference
            .survival
            .iter()
            .find(|b| b.kind == kind as u32)
            .expect("validated survival playback");
        let event = SurvivalAudioEvent {
            session: self.session,
            emitter,
            kind,
            action: AudioAction::Update,
            clip: b.clips[0],
            gain,
            pitch: b.pitch[0],
            position,
        };
        if let Some(old) = self
            .survival_loops
            .iter_mut()
            .find(|v| v.kind == kind && v.emitter == emitter)
        {
            *old = event;
        } else if self.survival_loops.len() < self.survival_loops.capacity() {
            self.survival_loops.push(event);
        }
    }
    pub fn stop_survival_loop(&mut self, kind: AudioKind, emitter: u32) {
        self.survival_loops
            .retain(|v| v.kind != kind || v.emitter != emitter);
    }
}

/// Day and night ambient channels, independent of particle effects.
pub struct AmbientState {
    modifier: f32,
    only_modifier: f32,
}

impl AmbientState {
    pub fn new(reference: &crate::reference::AmbientReference) -> Self {
        Self {
            modifier: reference.modifier,
            only_modifier: reference.only_modifier,
        }
    }

    pub fn gains(
        &mut self,
        reference: &crate::reference::AmbientReference,
        cycle: &crate::lighting::CycleOutput,
        dt: f32,
    ) -> [f32; 2] {
        use crate::lighting::CyclePhase;
        let night = match cycle.phase {
            CyclePhase::Night => 1.,
            CyclePhase::Day => 0.,
            CyclePhase::Dawn => 1. - cycle.progress,
            CyclePhase::Dusk => cycle.progress,
        };
        let modifier =
            self.modifier * self.only_modifier * (cycle.biome_color[0] - cycle.biome_color[2]);
        let gains = [reference.day_volume, reference.night_volume]
            .map(|v| ((v[0] + (v[1] - v[0]) * night) * modifier).clamp(0., 1.));
        self.modifier = (self.modifier + dt * reference.recovery_per_second).clamp(0., 1.);
        self.only_modifier =
            (self.only_modifier + dt * reference.recovery_per_second).clamp(0., 1.);
        gains
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::save::{CameraSave, SourceHash, SourceStamp};
    use crate::simulation::FIXED_DT;
    use crate::simulation::engine::EngineInput;
    use crate::starting_area::GarageRuntime;

    #[test]
    fn full_queue_drops_hits_for_engine_and_living_sounds() {
        let event = |priority, sequence| AudioEvent {
            session: 1,
            sequence,
            binding: 0,
            emitter: 0,
            position: [0.; 3],
            gain: 1.,
            pitch: 1.,
            time_seconds: 0.,
            priority,
        };
        let mut queue = [None; 64];
        let mut length = 0;
        for i in 0..64 {
            insert_event(&mut queue, &mut length, event(100, i));
        }
        insert_event(&mut queue, &mut length, event(80, 64));
        insert_event(&mut queue, &mut length, event(55, 65));
        insert_event(&mut queue, &mut length, event(100, 66));
        let count = |p| queue.iter().flatten().filter(|e| e.priority == p).count();
        assert_eq!((length, count(100), count(80), count(55)), (64, 62, 1, 1));
        assert_eq!(queue[61].unwrap().sequence, 61, "the last hits leave");
        assert!(queue.iter().flatten().all(|e| e.sequence != 66));
    }

    fn installed_source() -> (ReferenceManifest, SourceStamp) {
        let path = "web/assets/reference/manifest.json";
        let bytes = std::fs::read(path).unwrap();
        let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
        assert_eq!(reference.build_id, 11140890);
        assert_eq!(reference.vehicles.len(), 1);
        assert_eq!(reference.vehicles[0].engine.source.file, "level1");
        assert_eq!(reference.vehicles[0].engine.source.path_id, 68282);
        let digest = std::process::Command::new("sha256sum")
            .arg(path)
            .output()
            .unwrap();
        assert!(digest.status.success());
        let stamp = SourceStamp {
            app_id: reference.app_id,
            build_id: reference.build_id,
            manifest_sha256: String::from_utf8(digest.stdout)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .into(),
            source_hashes: reference
                .source_hashes
                .iter()
                .map(|hash| SourceHash {
                    file: hash.file.clone(),
                    sha256: hash.sha256.clone(),
                })
                .collect(),
        };
        (reference, stamp)
    }

    fn seed_running(run: &mut GarageRuntime) {
        let mut saved = run.vehicles[0].engine.capture_save();
        saved.rpm = 2000.;
        saved.elapsed = 1.;
        saved.last_run_time = 1.;
        saved.supplied_fuel = 0.4;
        run.vehicles[0].engine_output = run.vehicles[0].engine.restore_save(&saved).unwrap();
    }

    fn publish(run: &mut GarageRuntime) {
        run.area.items.sync_poses(&run.physics);
        let output = run.vehicles[0].engine_output;
        let audio = run.audio.as_mut().unwrap();
        audio.fixed_step(&run.area, &run.physics, FIXED_DT);
        audio.vehicle(
            VehicleAudioInput {
                rpm: output.rpm,
                running: output.running,
                starter: output.starter,
                ignition: run.vehicles[0].ignition.ignition,
                ..Default::default()
            },
            FIXED_DT,
            audio.game_seconds,
        );
    }

    fn detach(
        run: &mut GarageRuntime,
        reference: &ReferenceManifest,
    ) -> crate::reference::PartSlotId {
        let item = reference.vehicles[0].engine.item;
        let slot = reference
            .part_slots
            .iter()
            .find(|slot| slot.part == Some(item))
            .unwrap()
            .id;
        run.area
            .interaction
            .parts
            .remove(item, &mut run.area.items, &mut run.physics)
            .unwrap();
        run.area.interaction.parts.detach_mechanism(
            item,
            &mut run.area.interaction.mechanisms,
            &mut run.physics,
        );
        run.area
            .interaction
            .sync_parts(&mut run.area.items, &mut run.physics)
            .unwrap();
        slot
    }

    fn advance_detached(run: &mut GarageRuntime, reference: &ReferenceManifest) {
        run.vehicles[0].engine_output = run.vehicles[0].engine.step(
            EngineInput::default(),
            run.area.items.vehicle_fluids(0).unwrap(),
            &reference.vehicles[0].engine,
            |_| panic!("detached engine cannot request a start draw"),
            FIXED_DT,
        );
        publish(run);
    }

    fn assert_voices(audio: &AudioState, playing: bool) {
        for id in [
            audio.reference.engine.idle_voice,
            audio.reference.engine.motor_voice,
        ] {
            assert_eq!(audio.signals[audio.voice_indices[&id]].playing, playing);
        }
    }

    #[test]
    #[ignore = "requires the installed build 11140890 export"]
    fn engine_audio_follows_detached_state() {
        // Source enginescript.Update, 12105-12213: audio follows running, not attachment.
        let (reference, stamp) = installed_source();
        let mut run = GarageRuntime::new(&reference).unwrap();
        seed_running(&mut run);
        publish(&mut run);
        assert_voices(run.audio.as_ref().unwrap(), true);
        while run.audio.as_mut().unwrap().pop_event().is_some() {}
        let slot = detach(&mut run, &reference);
        let item = reference.vehicles[0].engine.item;
        let body = run.area.items.get(item).unwrap().body.unwrap();
        run.physics.edit_body(body, |body| {
            body.set_translation(rapier3d::math::Vector::new(13., 4., -9.), true)
        });
        advance_detached(&mut run, &reference);
        assert!(
            run.vehicles[0].engine_output.running,
            "source run grace remains active"
        );
        assert_voices(run.audio.as_ref().unwrap(), true);
        assert_eq!(run.audio.as_ref().unwrap().event_count(), 0);
        assert!(!run.audio.as_ref().unwrap().starter_enabled);
        let stop_binding_id = run
            .audio
            .as_ref()
            .unwrap()
            .reference
            .engine
            .stop_binding
            .unwrap();
        for _ in 0..20 {
            if !run.vehicles[0].engine_output.running {
                break;
            }
            advance_detached(&mut run, &reference);
        }
        assert!(!run.vehicles[0].engine_output.running);
        let audio = run.audio.as_mut().unwrap();
        let emitter = audio
            .reference
            .emitters
            .iter()
            .find(|e| e.id == audio.binding(stop_binding_id).unwrap().emitter)
            .unwrap();
        let original_engine = to_rapier(
            crate::interaction::items::node_pose(
                &reference,
                reference.items.iter().find(|i| i.id == item).unwrap().node,
            )
            .unwrap(),
        );
        let local = original_engine.inverse()
            * to_rapier(crate::interaction::items::node_pose(&reference, emitter.node).unwrap());
        let expected_emitter_position = crate::vehicle::from_rapier(
            &(to_rapier(run.area.items.get(item).unwrap().pose) * local),
        )
        .position;
        let stop = audio.pop_event().unwrap();
        assert_eq!(stop.binding, stop_binding_id);
        assert_eq!(stop.position, expected_emitter_position);
        assert!(audio.pop_event().is_none());
        assert_voices(audio, false);
        advance_detached(&mut run, &reference);
        assert_eq!(run.audio.as_ref().unwrap().event_count(), 0);
        run.vehicles[0].engine_output.starter = true;
        publish(&mut run);
        assert!(run.audio.as_ref().unwrap().starter_enabled);
        run.clear_input();
        assert!(
            !run.audio.as_ref().unwrap().starter_enabled,
            "cancellation clears published starter immediately"
        );
        publish(&mut run);
        assert!(!run.audio.as_ref().unwrap().starter_enabled);
        assert_eq!(run.audio.as_ref().unwrap().event_count(), 0);
        // A chassis ignition switch still emits while its engine slot is empty.
        run.audio.as_mut().unwrap().ignition_changed(&run.area);
        let audio = run.audio.as_mut().unwrap();
        let ignition = audio.reference.bindings[audio.ignition.unwrap()].id;
        assert_eq!(audio.pop_event().unwrap().binding, ignition);
        assert!(audio.pop_event().is_none());
        // Restore stopped mechanics, then reattach the same engine without replaying a stop.
        let saved = run
            .capture_save(
                &reference,
                &stamp,
                CameraSave::FirstPerson,
                "2026-10-20T12:00:00Z",
            )
            .unwrap();
        let mut restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
        assert!(!restored.audio.as_ref().unwrap().last_running);
        restored
            .area
            .interaction
            .parts
            .install(item, slot, &mut restored.area.items, &mut restored.physics)
            .unwrap();
        restored
            .area
            .interaction
            .parts
            .mount_mechanism(item, &mut restored.area.interaction.mechanisms);
        restored
            .area
            .interaction
            .sync_parts(&mut restored.area.items, &mut restored.physics)
            .unwrap();
        publish(&mut restored);
        assert_eq!(restored.audio.as_ref().unwrap().event_count(), 0);
        assert!(!restored.audio.as_ref().unwrap().starter_enabled);
    }

    #[test]
    #[ignore = "requires the installed build 11140890 export"]
    fn engine_emitters_follow_item_pose_and_restore() {
        let (reference, stamp) = installed_source();
        let mut run = GarageRuntime::new(&reference).unwrap();
        seed_running(&mut run);
        publish(&mut run);
        detach(&mut run, &reference);
        let item = reference.vehicles[0].engine.item;
        let engine_node = reference.items.iter().find(|i| i.id == item).unwrap().node;
        let original =
            to_rapier(crate::interaction::items::node_pose(&reference, engine_node).unwrap());
        let audio = run.audio.as_ref().unwrap();
        let emitter_ids = [
            audio.reference.engine.emitter,
            audio.reference.voices[audio.voice_indices[&audio.reference.engine.idle_voice]].emitter,
            audio.reference.voices[audio.voice_indices[&audio.reference.engine.motor_voice]]
                .emitter,
            audio
                .binding(audio.reference.engine.starter_binding.unwrap())
                .unwrap()
                .emitter,
            audio
                .binding(audio.reference.engine.stop_binding.unwrap())
                .unwrap()
                .emitter,
        ];
        let locals: Vec<_> = emitter_ids
            .iter()
            .map(|id| {
                let emitter = audio
                    .reference
                    .emitters
                    .iter()
                    .find(|e| e.id == *id)
                    .unwrap();
                original.inverse()
                    * to_rapier(
                        crate::interaction::items::node_pose(&reference, emitter.node).unwrap(),
                    )
            })
            .collect();
        let body = run.area.items.get(item).unwrap().body.unwrap();
        let moved = to_rapier(crate::vehicle::VehiclePose {
            position: [21., 8., -14.],
            rotation: [0., 0.70710677, 0., 0.70710677],
        });
        run.physics
            .edit_body(body, |body| body.set_position(moved, true));
        publish(&mut run);
        for (&id, local) in emitter_ids.iter().zip(&locals) {
            let expected = crate::vehicle::from_rapier(
                &(to_rapier(run.area.items.get(item).unwrap().pose) * local),
            )
            .position;
            assert_eq!(
                run.audio.as_ref().unwrap().positions[&id],
                expected,
                "emitter {id} follows engine item"
            );
        }
        // Item poses already use the current origin. Do not subtract its shift twice.
        let shift = [16., 0., -8.];
        run.area.items.rebase(shift);
        run.area.origin = shift.map(f64::from);
        run.audio.as_mut().unwrap().rebase(shift);
        let shifted = to_rapier(run.area.items.get(item).unwrap().pose);
        run.physics
            .edit_body(body, |body| body.set_position(shifted, true));
        publish(&mut run);
        for (&id, local) in emitter_ids.iter().zip(&locals) {
            assert_eq!(
                run.audio.as_ref().unwrap().positions[&id],
                crate::vehicle::from_rapier(&(shifted * local)).position
            );
        }
        // Restore must synchronize the prior running edge before its first audio update.
        let saved = run
            .capture_save(
                &reference,
                &stamp,
                CameraSave::FirstPerson,
                "2026-10-20T12:00:00Z",
            )
            .unwrap();
        let mut restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
        assert!(restored.vehicles[0].engine_output.running);
        assert!(restored.audio.as_ref().unwrap().last_running);
        publish(&mut restored);
        assert_eq!(restored.audio.as_ref().unwrap().event_count(), 0);
    }
}
