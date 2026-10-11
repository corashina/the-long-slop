//! Source sound rules. Each step sets the looping voices and queues one-shots.
//! The browser plays one flat frame of floats per render frame.

use crate::reference::{
    AudioReference, ClipId, InteriorDoor, ItemId, ReferenceManifest, Surface, VoiceReference,
};
use crate::starting_area::{GarageRuntime, start_sample};
use crate::vehicle::{
    Impact, SLOT_SENSOR_FLAG, TERRAIN_COLLIDER_FLAG, VEHICLE_COLLIDER_FLAG, VehiclePose, to_rapier,
};
use rapier3d::math::Vector;
use rapier3d::prelude::{QueryFilter, Ray};

/// Floats per sound: clip, volume, pitch, position, distances, spatial blend, bus.
pub const SOUND_FLOATS: usize = 10;
/// Listener position and rotation, two bus gains, voice count, one-shot count.
pub const HEADER_FLOATS: usize = 11;
/// `mainscript.maxAudioCount`.
pub const MAX_ONE_SHOTS: usize = 32;
const WHEEL_SAMPLES: usize = 5;
const SEED: u64 = 0x5eed_a0d1_0000_0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    /// The source `Main` mixer group.
    Main = 0,
    /// `OutSideCar`. It follows the exposed `Outside` volume.
    Outside = 1,
    /// `CarSounds`. It follows the exposed `CarSounds` volume.
    Car = 2,
}

/// Looping voice slots in frame order.
pub mod slot {
    pub const IDLE: usize = 0;
    pub const MOTOR: usize = 1;
    pub const REVERSE: usize = 2;
    pub const BRAKE: usize = 3;
    pub const BOIL: usize = 4;
    pub const GROUND: usize = 5;
    pub const SPRING: usize = 9;
    pub const SLIDE: usize = 13;
    pub const WIND: usize = 17;
    pub const DAY: usize = 18;
    pub const NIGHT: usize = 21;
    pub const COUNT: usize = 24;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sound {
    pub clip: Option<ClipId>,
    pub volume: f32,
    pub pitch: f32,
    pub position: [f32; 3],
    pub min_distance: f32,
    pub max_distance: f32,
    pub spatial: f32,
    pub bus: u8,
}

pub struct AudioState {
    samples: u64,
    voices: [Sound; slot::COUNT],
    one_shots: Vec<Sound>,
    frame: Vec<f32>,
    outside: f32,
    doors: f32,
    car_bus: Bus,
    wind: f32,
    ambient_fade: f32,
    running: bool,
    handbrake: bool,
    start_elapsed: f32,
    cool_elapsed: f32,
    cool_next: f32,
    suspension: [f32; 4],
    spring_pitch: [f32; 4],
    wheel_rpm: [[f32; WHEEL_SAMPLES]; 4],
    hinges: Vec<f32>,
    valves: Vec<f32>,
    grounded: bool,
    bob: f32,
    bob_down: bool,
    player: [f32; 3],
    player_speed: f32,
    sensed: Vec<usize>,
    car_sensed: Vec<usize>,
    item_impacts: Vec<(ItemId, Impact)>,
}

impl AudioState {
    pub fn new(reference: &ReferenceManifest, runtime: &GarageRuntime) -> Self {
        let audio = &reference.audio;
        Self {
            samples: 0,
            voices: [Sound::default(); slot::COUNT],
            one_shots: Vec::with_capacity(MAX_ONE_SHOTS),
            frame: Vec::with_capacity(HEADER_FLOATS + (slot::COUNT + MAX_ONE_SHOTS) * SOUND_FLOATS),
            outside: 1.0,
            doors: 1.0,
            car_bus: Bus::Outside,
            wind: 0.0,
            ambient_fade: 0.0,
            running: runtime.engine_output.running,
            handbrake: runtime.handbrake,
            start_elapsed: f32::MAX,
            cool_elapsed: 0.0,
            cool_next: 0.0,
            suspension: [0.0; 4],
            spring_pitch: [1.0; 4],
            wheel_rpm: [[0.0; WHEEL_SAMPLES]; 4],
            hinges: audio
                .hinges
                .iter()
                .map(|h| runtime.area.interaction.hinge_angle(h.item).unwrap_or(0.0))
                .collect(),
            valves: audio
                .openings
                .iter()
                .map(|o| runtime.area.interaction.opening_valve(o.opening))
                .collect(),
            grounded: runtime.area.player.grounded(),
            bob: 0.0,
            bob_down: false,
            player: runtime.area.player.body_pose(&runtime.physics).position,
            player_speed: 0.0,
            sensed: Vec::with_capacity(audio.interiors.len()),
            car_sensed: Vec::with_capacity(audio.interiors.len()),
            item_impacts: Vec::with_capacity(16),
        }
    }

    /// Runs after each fixed step of the runtime. `chase` is the outside car view.
    pub fn step(
        &mut self,
        reference: &ReferenceManifest,
        runtime: &GarageRuntime,
        chase: bool,
        dt: f32,
    ) {
        let audio = &reference.audio;
        let listener = runtime.area.player.view_pose(&runtime.physics).position;
        let position = runtime.area.player.body_pose(&runtime.physics).position;
        let moved = distance(position, self.player);
        // A world rebase moves the player by hundreds of meters in one step.
        self.player_speed = if moved < 10.0 { moved / dt } else { 0.0 };
        self.player = position;
        self.buses(reference, runtime, listener, dt);
        self.engine(reference, runtime, dt);
        self.car(reference, runtime, dt);
        self.impacts(reference, runtime);
        self.steps(reference, runtime, dt);
        self.objects(reference, runtime, dt);
        self.ambient_fade = (self.ambient_fade + dt * audio.ambient.fade_rate).clamp(0.0, 1.0);
        self.wind(reference, runtime, listener, chase, dt);
        self.ambient(reference, runtime, listener);
    }

    /// Writes the frame for the browser and clears the one-shots.
    pub fn frame(&mut self, reference: &ReferenceManifest, listener: VehiclePose) -> &[f32] {
        let outside = mixer_gain(self.outside);
        let car = mixer_gain(self.outside * reference.audio.buses.car_multiplier);
        self.frame.clear();
        self.frame.extend_from_slice(&listener.position);
        self.frame.extend_from_slice(&listener.rotation);
        self.frame.extend_from_slice(&[
            outside,
            car,
            slot::COUNT as f32,
            self.one_shots.len() as f32,
        ]);
        for sound in self.voices.iter().chain(&self.one_shots) {
            self.frame.extend_from_slice(&[
                sound.clip.map_or(-1.0, |clip| clip.0 as f32),
                sound.volume,
                sound.pitch,
                sound.position[0],
                sound.position[1],
                sound.position[2],
                sound.min_distance,
                sound.max_distance,
                sound.spatial,
                sound.bus as f32,
            ]);
        }
        self.one_shots.clear();
        &self.frame
    }

    pub fn one_shots(&self) -> &[Sound] {
        &self.one_shots
    }

    pub fn voice(&self, index: usize) -> Sound {
        self.voices[index]
    }

    /// Mixer value before `mainscript.SetVolume`.
    pub fn outside(&self) -> f32 {
        self.outside
    }

    pub fn car_bus(&self) -> Bus {
        self.car_bus
    }

    fn sample(&mut self) -> f32 {
        self.samples = self.samples.wrapping_add(1);
        start_sample(SEED, self.samples)
    }

    fn range(&mut self, range: [f32; 2]) -> f32 {
        mix(range[0], range[1], self.sample())
    }

    /// `mainscript.PlayClipAtPoint`. A volume above one widens the distances.
    fn play(
        &mut self,
        audio: &AudioReference,
        clip: ClipId,
        position: [f32; 3],
        volume: f32,
        pitch: f32,
        bus: Bus,
    ) {
        let (gain, scale) = if volume > 1.0 {
            (1.0, volume - 1.0)
        } else {
            (volume, 1.0)
        };
        if self.one_shots.len() >= MAX_ONE_SHOTS || gain <= 0.0 || !gain.is_finite() {
            return;
        }
        self.one_shots.push(Sound {
            clip: Some(clip),
            volume: gain,
            pitch,
            position,
            min_distance: audio.one_shot.min_distance * scale,
            max_distance: audio.one_shot.max_distance * scale,
            spatial: audio.one_shot.spatial,
            bus: bus as u8,
        });
    }

    fn random_pitch(&mut self, audio: &AudioReference) -> f32 {
        self.range(audio.one_shot.random_pitch)
    }

    /// `fpscontroller.SetOutSideAudio` and `audioBasedOnInteriorsensscript`.
    fn buses(
        &mut self,
        reference: &ReferenceManifest,
        runtime: &GarageRuntime,
        listener: [f32; 3],
        dt: f32,
    ) {
        let audio = &reference.audio;
        sense(audio, runtime, listener, &mut self.sensed);
        let player_car = self.sensed.iter().any(|&i| audio.interiors[i].vehicle);
        let current = self
            .sensed
            .iter()
            .copied()
            .find(|&i| audio.interiors[i].vehicle)
            .or_else(|| self.sensed.first().copied());
        let mut target = 1.0;
        self.doors = 1.0;
        if let Some(current) = current {
            target *= audio.buses.default_interior;
            self.doors = self
                .sensed
                .iter()
                .map(|&i| {
                    let value = interior_value(reference, runtime, i, None, 0);
                    if audio.interiors[i].inverted {
                        1.0 - value
                    } else {
                        value
                    }
                })
                .fold(f32::MAX, f32::min);
            let value = audio.interiors[current]
                .value
                .unwrap_or(audio.buses.default_interior);
            target += (1.0 - value) * self.doors;
        }
        self.outside += (target - self.outside) * (audio.buses.interior_lerp * dt).clamp(0.0, 1.0);
        if player_car {
            self.car_bus = Bus::Car;
            return;
        }
        let car = runtime.physics.snapshot().pose.position;
        sense(audio, runtime, car, &mut self.car_sensed);
        let around = self
            .car_sensed
            .iter()
            .copied()
            .find(|&i| !audio.interiors[i].vehicle);
        let parent = |i: usize| audio.interiors[i].parent;
        self.car_bus = match (around, current) {
            (None, _) => Bus::Outside,
            (Some(_), None) => Bus::Main,
            (Some(car), Some(player))
                if car == player
                    || parent(car) == Some(player)
                    || (parent(car).is_some() && parent(car) == parent(player))
                    || (parent(car).is_none() && parent(player) == Some(car)) =>
            {
                Bus::Main
            }
            _ => Bus::Outside,
        };
    }

    /// `enginescript` sound section.
    fn engine(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime, dt: f32) {
        let audio = &reference.audio;
        let sounds = &audio.engine;
        let output = runtime.engine_output;
        let rpm = output.rpm;
        let bus = self.car_bus;
        let position = item_point(runtime, reference.engine.item, [0.0; 3]);
        let (idle, motor) = if output.running {
            let [min, max] = sounds.idle_rpm;
            let idle = (
                inverse_lerp(max, min, rpm) * sounds.idle_volume,
                lerp_unclamped(
                    sounds.idle_pitch[0],
                    sounds.idle_pitch[1],
                    inverse_lerp(0.0, max, rpm) + inverse_lerp(max, max * 2.0, rpm),
                ),
            );
            let top = reference.engine.max_rpm;
            let motor = (
                inverse_lerp(sounds.motor_rpm[0], sounds.motor_rpm[1], rpm) * sounds.motor_volume,
                lerp_unclamped(
                    sounds.motor_pitch[0],
                    sounds.motor_pitch[1],
                    inverse_lerp(sounds.motor_rpm[0], top, rpm) + inverse_lerp(top, top * 2.0, rpm),
                ),
            );
            (idle, motor)
        } else {
            ((0.0, 1.0), (0.0, 1.0))
        };
        self.voices[slot::IDLE] = voice(&sounds.idle, position, idle.0, idle.1, bus);
        self.voices[slot::MOTOR] = voice(&sounds.motor, position, motor.0, motor.1, bus);
        self.start_elapsed += dt;
        let start_seconds = audio.clips[sounds.start.0 as usize].seconds
            * reference.engine.engine_start_time_modifier;
        if output.starter && self.start_elapsed > start_seconds {
            self.start_elapsed = 0.0;
            self.play(
                audio,
                sounds.start,
                position,
                sounds.start_volume,
                1.0,
                Bus::Main,
            );
        }
        if self.running && !output.running {
            self.play(
                audio,
                sounds.stop,
                position,
                sounds.stop_volume,
                1.0,
                Bus::Main,
            );
        }
        self.running = output.running;
        let cooling = &audio.cooling;
        self.cool_elapsed += dt;
        if output.temperature > cooling.temperature[0]
            && !output.running
            && self.cool_elapsed >= self.cool_next
        {
            let t = inverse_lerp(
                cooling.temperature[0],
                cooling.temperature[1],
                output.temperature,
            );
            let between = |pairs: [[f32; 2]; 2]| {
                [
                    mix(pairs[0][0], pairs[1][0], t),
                    mix(pairs[0][1], pairs[1][1], t),
                ]
            };
            self.cool_next = self.range(between(cooling.time));
            self.cool_elapsed = 0.0;
            let index = ((self.sample() * cooling.clips.len() as f32) as usize)
                .min(cooling.clips.len() - 1);
            let volume = self.range(between(cooling.volume));
            let pitch = self.range(between(cooling.pitch));
            self.play(audio, cooling.clips[index], position, volume, pitch, bus);
        }
    }

    /// `carscript` and `talajnezo` sounds.
    fn car(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime, dt: f32) {
        let audio = &reference.audio;
        let sounds = &audio.car;
        let bus = self.car_bus;
        let car = reference.vehicle.item;
        let snapshot = runtime.physics.snapshot();
        let velocity = Vector::from_array(snapshot.linear_velocity);
        let speed = velocity.length();
        let output = runtime.engine_output;
        let engine = runtime.area.items.mounted(car, reference.engine.item);
        let reverse = if engine && reference.vehicle.gear_ratios[runtime.gear] < 0.0 {
            let value = inverse_lerp(0.0, reference.engine.max_rpm, output.rpm.abs());
            (
                value * audio.engine.reverse_volume * runtime.clutch,
                mix(0.9, 1.1, value),
            )
        } else {
            (0.0, 1.0)
        };
        self.voices[slot::REVERSE] = voice(
            &sounds.reverse,
            item_point(runtime, car, sounds.reverse.offset),
            reverse.0,
            reverse.1,
            bus,
        );
        let brake = if runtime.physics.braking() {
            speed * 3.6 * sounds.brake_modifier
        } else {
            0.0
        };
        self.voices[slot::BRAKE] = voice(
            &sounds.brake,
            item_point(runtime, car, sounds.brake.offset),
            brake,
            1.0,
            bus,
        );
        let [steam, hottest] = [
            reference.engine.globals.min_steam_temperature,
            reference.engine.globals.car_temperature_bounds[1],
        ];
        let coolant = runtime
            .area
            .items
            .get(reference.engine.coolant_tank)
            .and_then(|item| item.fluid.as_ref())
            .map_or(0.0, |fluid| fluid.mixture.total());
        let boil = if engine && output.temperature > steam && coolant > 0.0 {
            mix(
                0.0,
                sounds.boil_volume,
                inverse_lerp(steam, hottest, output.temperature),
            )
        } else {
            0.0
        };
        self.voices[slot::BOIL] = voice(
            &sounds.boil,
            item_point(runtime, car, sounds.boil.offset),
            boil,
            1.0,
            bus,
        );
        if runtime.handbrake != self.handbrake && self.one_shots.len() < MAX_ONE_SHOTS {
            let source = &sounds.handbrake_voice;
            self.one_shots.push(Sound {
                clip: Some(sounds.handbrake),
                volume: 1.0,
                pitch: 1.0,
                position: item_point(runtime, car, source.offset),
                min_distance: source.min_distance,
                max_distance: source.max_distance,
                spatial: source.spatial,
                bus: bus as u8,
            });
        }
        self.handbrake = runtime.handbrake;
        let wheels = runtime.physics.wheel_contacts();
        let speed_kmh = speed * 3.6;
        for (i, wheel) in wheels.iter().enumerate() {
            let position = snapshot.wheels[i].position;
            let road = !dirt(audio, wheel.ground);
            let ground = if wheel.contact && speed > 1.0 {
                inverse_lerp(0.0, 5.0, speed)
                    * if road { 0.5 } else { 1.0 }
                    * sounds.ground_modifier
            } else {
                0.0
            };
            self.voices[slot::GROUND + i] = voice(&sounds.wheel.ground, position, ground, 1.0, bus);
            let mut spring = self.voices[slot::SPRING + i].volume;
            if wheel.contact {
                let change = wheel.suspension_length - self.suspension[i];
                self.suspension[i] = wheel.suspension_length;
                let pitch = (1.0 + change * sounds.spring_pitch_modifier).clamp(0.95, 1.05);
                self.spring_pitch[i] += (pitch - self.spring_pitch[i]) * (dt * 5.0).min(1.0);
                let target = change.abs() * sounds.spring_modifier * dt;
                spring += (target - spring) * (dt * 3.0).min(1.0);
            } else {
                spring = 0.0;
            }
            self.voices[slot::SPRING + i] = voice(
                &sounds.wheel.spring,
                position,
                spring,
                self.spring_pitch[i],
                bus,
            );
            let mut slide = 0.0;
            if wheel.contact {
                let forward = Vector::from_array(wheel.forward);
                let angle = angle(velocity, forward).min(angle(velocity, -forward));
                if angle > sounds.slide_angle {
                    slide = inverse_lerp(sounds.slide_angle, 90.0, angle)
                        * inverse_lerp(0.0, sounds.max_slide_volume_speed, speed);
                }
                // `talajnezo` shifts the history with a forward copy loop.
                let history = &mut self.wheel_rpm[i];
                for j in 0..WHEEL_SAMPLES - 1 {
                    history[j + 1] = history[j];
                }
                history[0] = wheel.rpm;
                let rpm = history.iter().sum::<f32>() / WHEEL_SAMPLES as f32;
                let rolling = rpm.abs() * (wheel.radius * std::f32::consts::TAU) / 60.0 * 3.6;
                let difference = (speed_kmh - rolling).abs();
                if difference > sounds.speed_difference[0] {
                    slide += inverse_lerp(
                        sounds.speed_difference[0],
                        sounds.speed_difference[1],
                        difference,
                    );
                }
            }
            let skid = if road {
                &sounds.skid_road
            } else {
                &sounds.skid_dirt
            };
            let mut sound = voice(
                &sounds.wheel.slide,
                position,
                slide * skid.volume,
                mix(skid.pitch[0], skid.pitch[1], slide),
                bus,
            );
            sound.clip = Some(skid.clip);
            self.voices[slot::SLIDE + i] = sound;
        }
    }

    /// `carscript.OnCollisionEnter` and `pickupable.OnCollisionEnter`.
    fn impacts(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime) {
        let audio = &reference.audio;
        let mut car: Option<Impact> = None;
        self.item_impacts.clear();
        for impact in runtime.physics.impacts().iter() {
            let [a, b] = impact.user_data;
            if (a & VEHICLE_COLLIDER_FLAG != 0) != (b & VEHICLE_COLLIDER_FLAG != 0)
                && car.is_none_or(|car| impact.speed > car.speed)
            {
                car = Some(*impact);
            }
            for data in [a, b] {
                let low = data as u32;
                if data & (VEHICLE_COLLIDER_FLAG | SLOT_SENSOR_FLAG | TERRAIN_COLLIDER_FLAG) != 0
                    || low == 0
                {
                    continue;
                }
                let item = ItemId(low - 1);
                match self.item_impacts.iter_mut().find(|(id, _)| *id == item) {
                    Some((_, best)) if impact.speed > best.speed => *best = *impact,
                    Some(_) => {}
                    None => self.item_impacts.push((item, *impact)),
                }
            }
        }
        if let Some(impact) = car {
            let sounds = &audio.car;
            let magnitude = impact.speed * sounds.collision_modifier * 0.3;
            let (tier, volume) = if magnitude > 0.7 {
                (3, magnitude)
            } else if magnitude > 0.5 {
                (2, magnitude)
            } else if magnitude > 0.2 {
                (1, magnitude)
            } else if magnitude > 0.05 {
                (0, 1.0)
            } else {
                (4, 0.0)
            };
            if tier < 4 {
                let clips = &sounds.collide[tier];
                let index = ((self.sample() * clips.len() as f32) as usize).min(clips.len() - 1);
                let pitch = self.random_pitch(audio);
                self.play(
                    audio,
                    clips[index],
                    impact.point,
                    volume,
                    pitch,
                    self.car_bus,
                );
            }
        }
        let held = runtime.area.interaction.held();
        for index in 0..self.item_impacts.len() {
            let (item, impact) = self.item_impacts[index];
            let Some(sound) = audio.items.iter().find(|sound| sound.item == item) else {
                continue;
            };
            let value = impact.speed * sound.modifier * if held == Some(item) { 1.0 } else { 0.3 };
            if value > 0.075 {
                self.play(
                    audio,
                    sound.clip,
                    impact.point,
                    value - 0.075,
                    1.0,
                    Bus::Main,
                );
            }
        }
    }

    /// `fpscontroller` head bob steps and `stepSoundScript`.
    fn steps(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime, dt: f32) {
        let steps = &reference.audio.steps;
        let player = &runtime.area.player;
        let grounded = player.grounded();
        if player.seat().is_some() {
            self.grounded = grounded;
            return;
        }
        let duck = if player.crouched() {
            reference.player.ctrl_duck_value
        } else {
            1.0
        };
        let modifier = duck
            + if player.running() {
                steps.run_bonus
            } else {
                0.0
            };
        if grounded != self.grounded {
            self.footstep(reference, runtime, modifier);
        }
        self.grounded = grounded;
        if grounded && self.player_speed > 0.01 {
            self.bob += dt * steps.bob_speed + self.player_speed * steps.bob_speed_velocity * dt;
            if self.bob.sin() * steps.bob_height <= -steps.bob_threshold {
                if !self.bob_down {
                    self.bob_down = true;
                    self.footstep(reference, runtime, modifier);
                }
            } else {
                self.bob_down = false;
            }
        }
    }

    fn footstep(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime, modifier: f32) {
        let audio = &reference.audio;
        let steps = &audio.steps;
        let player = &runtime.area.player;
        let center = Vector::from_array(player.body_pose(&runtime.physics).position);
        let half = 0.5
            * if player.crouched() {
                reference.player.crouch_height
            } else {
                reference.player.standing_height
            };
        let filter = QueryFilter::default()
            .exclude_sensors()
            .exclude_rigid_body(player.body_handle());
        let Some((handle, _)) = runtime.physics.queries(filter).cast_ray(
            &Ray::new(center, -Vector::Y),
            half + steps.ray_distance,
            true,
        ) else {
            return;
        };
        let data = runtime.physics.rapier().colliders[handle].user_data;
        let surface = match surface(audio, data) {
            Surface::Terrain => match crate::world::generation::biome_of(runtime.terrain_color) {
                1 => Surface::Winter,
                2 => Surface::Grass,
                _ => Surface::Desert,
            },
            surface => surface,
        };
        let walk = &steps.surfaces[match surface {
            Surface::Road | Surface::Terrain => 0,
            Surface::Desert => 1,
            Surface::Winter => 2,
            Surface::Grass => 3,
            Surface::Wood => 4,
        }];
        let index = ((self.sample() * walk.clips.len() as f32) as usize).min(walk.clips.len() - 1);
        let pitch = self.random_pitch(audio);
        let feet = (center - Vector::Y * half).to_array();
        let volume = walk.volume * modifier * steps.modifier;
        self.play(audio, walk.clips[index], feet, volume, pitch, Bus::Main);
    }

    /// `usablescript.RotSound` limit clips and `tankcapscript.USE`.
    fn objects(&mut self, reference: &ReferenceManifest, runtime: &GarageRuntime, dt: f32) {
        let audio = &reference.audio;
        for index in 0..audio.hinges.len() {
            let sound = &audio.hinges[index];
            let Some(angle) = runtime.area.interaction.hinge_angle(sound.item) else {
                continue;
            };
            let last = std::mem::replace(&mut self.hinges[index], angle);
            if angle == last {
                continue;
            }
            let hinge = reference
                .hinges
                .iter()
                .find(|hinge| hinge.item == sound.item)
                .expect("validated hinge sound");
            let at = |value: f32, limit: f32| (value - limit).abs() < 1.0e-3;
            let volume = ((angle - last) * sound.modifier).abs() / dt * 0.01;
            let position = item_point(runtime, sound.item, [0.0; 3]);
            let (min, max) = (hinge.min_degrees, hinge.max_degrees);
            for (clip, play) in [
                (sound.min_close, at(angle, min) && !at(last, min)),
                (sound.max_close, at(angle, max) && !at(last, max)),
                (sound.min_open, !at(angle, min) && at(last, min)),
                (sound.max_open, !at(angle, max) && at(last, max)),
            ] {
                if let (Some(clip), true) = (clip, play) {
                    let pitch = self.random_pitch(audio);
                    self.play(audio, clip, position, volume, pitch, Bus::Main);
                }
            }
        }
        for index in 0..audio.openings.len() {
            let sound = &audio.openings[index];
            let valve = runtime.area.interaction.opening_valve(sound.opening);
            let last = std::mem::replace(&mut self.valves[index], valve);
            let position = item_point(runtime, sound.item, [0.0; 3]);
            let (clip, volume) = if last <= 0.0 && valve > 0.0 {
                (sound.open, sound.open_volume)
            } else if last > 0.0 && valve <= 0.0 {
                (sound.close, sound.close_volume)
            } else {
                (None, 0.0)
            };
            if let Some(clip) = clip {
                self.play(audio, clip, position, volume, 1.0, Bus::Main);
            }
        }
    }

    /// `fpscontroller` wind source.
    fn wind(
        &mut self,
        reference: &ReferenceManifest,
        runtime: &GarageRuntime,
        listener: [f32; 3],
        chase: bool,
        dt: f32,
    ) {
        let wind = &reference.audio.wind;
        let seated = runtime.area.player.seat().is_some();
        let speed = if seated {
            Vector::from_array(runtime.physics.snapshot().linear_velocity).length()
        } else {
            self.player_speed
        };
        let view = if chase { wind.view_modifier } else { 1.0 };
        let target =
            view * if seated && self.doors <= 0.01 {
                0.0
            } else {
                inverse_lerp(0.0, wind.max_speed_kmh, speed * 3.6)
            } * self.ambient_fade;
        self.wind += (target - self.wind) * (wind.lerp * dt).min(1.0);
        let volume = if self.wind < 0.05 { 0.0 } else { self.wind };
        self.voices[slot::WIND] = voice(&wind.voice, listener, volume, 1.0, Bus::Outside);
    }

    /// `napszakvaltakozas` ambient sources.
    fn ambient(
        &mut self,
        reference: &ReferenceManifest,
        runtime: &GarageRuntime,
        listener: [f32; 3],
    ) {
        let ambient = &reference.audio.ambient;
        let night = crate::simulation::day::night_weight(&reference.day_cycle, runtime.day_seconds);
        let [r, b] = [runtime.terrain_color[0], runtime.terrain_color[2]];
        let biome = |values: &[f32; 3]| mix(mix(values[2], values[0], r), values[1], b);
        let modifier = self.ambient_fade * self.ambient_fade;
        let day = mix(
            biome(&ambient.day_volume[0]),
            biome(&ambient.day_volume[1]),
            night,
        ) * modifier;
        let dark = mix(
            biome(&ambient.night_volume[0]),
            biome(&ambient.night_volume[1]),
            night,
        ) * modifier;
        let shares = [r - b, b, (1.0 - r).min(1.0 - b)];
        for (i, share) in shares.into_iter().enumerate() {
            self.voices[slot::DAY + i] =
                voice(&ambient.day[i], listener, day * share, 1.0, Bus::Outside);
            self.voices[slot::NIGHT + i] =
                voice(&ambient.night[i], listener, dark * share, 1.0, Bus::Outside);
        }
    }
}

/// Interiors whose collider holds the point.
fn sense(audio: &AudioReference, runtime: &GarageRuntime, point: [f32; 3], out: &mut Vec<usize>) {
    out.clear();
    for (_, collider) in runtime
        .physics
        .queries(QueryFilter::default())
        .intersect_point(Vector::from_array(point))
    {
        let id = (collider.user_data >> 64) as u32;
        if let Some(index) = audio.interiors.iter().position(|i| i.collider.0 == id)
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
}

/// `interiorscript.GetValue`. `caller` is the child that asks its parent.
fn interior_value(
    reference: &ReferenceManifest,
    runtime: &GarageRuntime,
    index: usize,
    caller: Option<usize>,
    depth: u8,
) -> f32 {
    let interior = &reference.audio.interiors[index];
    let use_parent = caller.is_none() || caller != interior.parent;
    let mut open: f32 = interior
        .doors
        .iter()
        .map(|door| (door_value(reference, runtime, door) * 2.0).clamp(0.0, 1.0))
        .sum();
    let all = interior.doors.len() as f32;
    if use_parent
        && depth < 8
        && let Some(parent) = interior.parent
    {
        let door = interior.parent_door.map_or(1.0, |door| {
            (door_value(reference, runtime, &door) * 2.0).clamp(0.0, 1.0)
        });
        open += interior_value(reference, runtime, parent, Some(index), depth + 1) * door;
    }
    if interior.min_doors_to_max != 0 && open >= interior.min_doors_to_max as f32 {
        return if interior.max_value != 0.0 {
            interior.max_value.min(1.0)
        } else {
            1.0
        };
    }
    let value = inverse_lerp(0.0, all.clamp(1.0, 100.0), open);
    if interior.max_value != 0.0 {
        value.min(interior.max_value)
    } else {
        value
    }
}

/// `usablescript.GetValue` for a door. A removed door is fully open.
fn door_value(reference: &ReferenceManifest, runtime: &GarageRuntime, door: &InteriorDoor) -> f32 {
    let Some(item) = door.item else { return 0.0 };
    if door.removable && !runtime.area.items.mounted(reference.vehicle.item, item) {
        return 1.0;
    }
    let (Some(hinge), Some(angle)) = (
        reference.hinges.iter().find(|hinge| hinge.item == item),
        runtime.area.interaction.hinge_angle(item),
    ) else {
        return if door.removable { 1.0 } else { 0.0 };
    };
    let inverted = reference
        .audio
        .hinges
        .iter()
        .find(|sound| sound.item == item)
        .is_some_and(|sound| sound.inverted);
    if inverted {
        inverse_lerp(hinge.max_degrees, hinge.min_degrees, angle)
    } else {
        inverse_lerp(hinge.min_degrees, hinge.max_degrees, angle)
    }
}

fn surface(audio: &AudioReference, data: u128) -> Surface {
    if data & TERRAIN_COLLIDER_FLAG != 0 {
        return Surface::Terrain;
    }
    let id = (data >> 64) as u32;
    audio
        .steps
        .colliders
        .iter()
        .find(|record| record.collider.0 == id)
        .map_or(Surface::Road, |record| record.surface)
}

/// `talajnezo` reads the sand layer as dirt.
fn dirt(audio: &AudioReference, data: u128) -> bool {
    surface(audio, data) == Surface::Terrain
}

fn item_point(runtime: &GarageRuntime, item: ItemId, offset: [f32; 3]) -> [f32; 3] {
    let pose = runtime
        .area
        .items
        .get(item)
        .map_or(runtime.physics.snapshot().pose, |item| item.pose);
    (to_rapier(pose) * Vector::from_array(offset)).to_array()
}

fn voice(source: &VoiceReference, position: [f32; 3], volume: f32, pitch: f32, bus: Bus) -> Sound {
    Sound {
        clip: source.clip,
        // Unity clamps an AudioSource volume to the unit range.
        volume: if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        },
        pitch,
        position,
        min_distance: source.min_distance,
        max_distance: source.max_distance,
        spatial: source.spatial,
        bus: bus as u8,
    }
}

/// `mainscript.SetVolume` writes `20 ln(v)` decibels. The linear gain is `v ^ ln 10`.
pub fn mixer_gain(value: f32) -> f32 {
    value.clamp(0.001, 1.0).powf(std::f32::consts::LN_10)
}

/// Unity `Vector3.Angle` in degrees.
fn angle(a: Vector, b: Vector) -> f32 {
    let denominator = (a.length_squared() * b.length_squared()).sqrt();
    if denominator < 1.0e-15 {
        return 0.0;
    }
    (a.dot(b) / denominator)
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    Vector::from_array(a).distance(Vector::from_array(b))
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn lerp_unclamped(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}
