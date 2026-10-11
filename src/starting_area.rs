//! Persistent imported home bodies and preparation state.
use crate::interaction::{
    AimTarget, InteractionAction, InteractionState,
    items::{ItemLocation, ItemStore, node_pose},
    parts::PartSlots,
};
use crate::player::PlayerState;
use crate::reference::{BodyId, ItemId, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::{
    DriveInput, FIXED_DT, FrameInput,
    engine::{EngineInput, EngineOutput, EngineState},
    keys,
};
use crate::vehicle::{
    PhysicsWorld, VEHICLE_COLLIDER_FLAG, VehiclePose, collider_builder, to_rapier,
};
use crate::world::WorldStream;
use rapier3d::prelude::{
    Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder, RigidBodyHandle,
};
use std::collections::HashMap;

/// Seed for the engine start sequence.
const START_SEED: u64 = 42;
/// Seed for crash thresholds and part choices.
const CRASH_SEED: u64 = 7;

struct CompoundCollider {
    handle: crate::vehicle::ColliderHandle,
    body: BodyId,
    fixed_pose: rapier3d::math::Pose,
    shape_pose: rapier3d::math::Pose,
}

impl CompoundCollider {
    fn local_pose(&self, interaction: &InteractionState) -> rapier3d::math::Pose {
        interaction
            .hinge_local_pose(self.body)
            .unwrap_or(self.fixed_pose)
            * self.shape_pose
    }
}

pub struct StartingArea {
    pub bodies: HashMap<BodyId, RigidBodyHandle>,
    pub items: ItemStore,
    pub interaction: InteractionState,
    pub player: PlayerState,
    pub origin: [f64; 3],
    attachments: Vec<(RigidBodyHandle, RigidBodyHandle, rapier3d::math::Pose)>,
    pub render_poses: Vec<(BodyId, VehiclePose, VehiclePose)>,
    pub item_render_poses: Vec<(ItemId, VehiclePose, VehiclePose)>,
    compound_colliders: Vec<CompoundCollider>,
    portable_mass: Vec<(RigidBodyHandle, f32, Vec<ItemId>, f32)>,
}

impl StartingArea {
    pub fn new(
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
        origin: [f64; 3],
    ) -> Result<Self, ReferenceError> {
        let mut items = ItemStore::new(reference)?;
        let mut interaction = InteractionState::new(reference)?;
        let mut bodies = HashMap::with_capacity(reference.bodies.len());
        let mut compound_colliders = Vec::new();
        bodies.insert(reference.vehicle.body, physics.chassis_handle());
        for body in &reference.bodies {
            if body.id == reference.vehicle.body {
                continue;
            }
            let pose = VehiclePose {
                position: std::array::from_fn(|i| body.pose.translation[i] - origin[i] as f32),
                rotation: body.pose.rotation,
            };
            let builder = if body.parent_body.is_some() {
                RigidBodyBuilder::kinematic_position_based()
            } else if body.dynamic {
                RigidBodyBuilder::dynamic()
                    .ccd_enabled(true)
                    .additional_mass(body.mass_kg)
                    .linear_damping(body.linear_damping)
                    .angular_damping(body.angular_damping)
            } else {
                RigidBodyBuilder::fixed()
            };
            let handle = physics
                .rapier_mut()
                .insert_body(builder.pose(to_rapier(pose)));
            bodies.insert(body.id, handle);
        }
        for record in &reference.colliders {
            if reference.vehicle.colliders.contains(&record.id) {
                continue;
            }
            let item = collider_item(reference, record.node);
            let flag = if belongs_to_vehicle(reference, record.body) {
                VEHICLE_COLLIDER_FLAG
            } else {
                0
            };
            let builder = collider_builder(&record.geometry)?
                .friction(record.friction)
                .restitution(record.restitution)
                .sensor(record.sensor)
                .enabled(record.enabled)
                .density(0.0)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(record.membership),
                    Group::from_bits_truncate(record.filter),
                    InteractionTestMode::And,
                ))
                .user_data(
                    flag | ((record.id.0 as u128) << 64)
                        | item.map_or(0, |item| item.0 as u128 + 1),
                );
            let owner = bodies[&record.body];
            let chassis = bodies[&reference.vehicle.body];
            let mut collider = builder.build();
            let local = *collider.position();
            let physical_parent = if flag != 0 && owner != chassis {
                collider.set_position(
                    physics.rapier().bodies[chassis].position().inverse()
                        * *physics.rapier().bodies[owner].position()
                        * local,
                );
                chassis
            } else {
                owner
            };
            let rapier = physics.rapier_mut();
            let handle =
                rapier
                    .colliders
                    .insert_with_parent(collider, physical_parent, &mut rapier.bodies);
            if physical_parent != owner {
                let source = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == record.body)
                    .unwrap();
                let parent = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == reference.vehicle.body)
                    .unwrap();
                compound_colliders.push(CompoundCollider {
                    handle,
                    body: record.body,
                    fixed_pose: to_rapier(VehiclePose {
                        position: parent.pose.translation,
                        rotation: parent.pose.rotation,
                    })
                    .inverse()
                        * to_rapier(VehiclePose {
                            position: source.pose.translation,
                            rotation: source.pose.rotation,
                        }),
                    shape_pose: local,
                });
            }
        }
        // The vehicle constructor owns these shapes. Add only their aim identity.
        for (_, collider) in physics.rapier_mut().colliders.iter_mut() {
            if collider.parent() == Some(bodies[&reference.vehicle.body]) {
                let source = (collider.user_data >> 64) as u32;
                if let Some(record) = reference
                    .colliders
                    .iter()
                    .find(|record| record.id.0 == source)
                {
                    collider.user_data |=
                        collider_item(reference, record.node).map_or(0, |id| id.0 as u128 + 1);
                }
            }
        }
        for (&id, &handle) in &bodies {
            items.bind_body(id, handle);
            interaction.bind_body(id, handle);
        }
        for slot in &reference.wheel_slots {
            let builder = rapier3d::prelude::ColliderBuilder::ball(slot.radius)
                .sensor(true)
                .enabled(false)
                .density(0.0)
                .user_data(
                    VEHICLE_COLLIDER_FLAG
                        | crate::vehicle::SLOT_SENSOR_FLAG
                        | ((slot.id.0 as u128) << 96)
                        | (slot.wheel.0 as u128 + 1),
                );
            let rapier = physics.rapier_mut();
            rapier.colliders.insert(builder);
        }
        interaction.rebase(origin.map(|v| v as f32));
        interaction.sync_hinges(physics);
        for collider in &compound_colliders {
            physics.rapier_mut().colliders[collider.handle]
                .set_position_wrt_parent(collider.local_pose(&interaction));
        }
        physics.refresh_queries();
        items.sync_poses(physics);
        let spawn = &reference.area.player_spawn;
        let mut player = PlayerState::new(
            &reference.player,
            VehiclePose {
                position: std::array::from_fn(|i| spawn.translation[i] - origin[i] as f32),
                rotation: spawn.rotation,
            },
            physics,
        );
        player.configure_seats(&reference.seats);
        interaction.bind_player_body(player.body_handle());
        let attachments = reference
            .bodies
            .iter()
            .filter_map(|record| {
                let parent = record.parent_body?;
                if reference.hinges.iter().any(|hinge| hinge.body == record.id) {
                    return None;
                }
                let parent_record = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == parent)
                    .expect("validated parent");
                let source = to_rapier(VehiclePose {
                    position: record.pose.translation,
                    rotation: record.pose.rotation,
                });
                let parent_source = to_rapier(VehiclePose {
                    position: parent_record.pose.translation,
                    rotation: parent_record.pose.rotation,
                });
                Some((
                    bodies[&record.id],
                    bodies[&parent],
                    parent_source.inverse() * source,
                ))
            })
            .collect();
        let render_poses = bodies
            .iter()
            .map(|(&id, &handle)| {
                let pose = crate::vehicle::from_rapier(physics.rapier().bodies[handle].position());
                (id, pose, pose)
            })
            .collect();
        let item_render_poses = items
            .iter()
            .map(|item| (item.id, item.pose, item.pose))
            .collect();
        let portable_mass = reference
            .bodies
            .iter()
            .filter_map(|body| {
                if !body.dynamic || body.id == reference.vehicle.body {
                    return None;
                }
                let tanks: Vec<_> = reference
                    .tanks
                    .iter()
                    .filter(|tank| {
                        tank.role == crate::reference::TankRole::Portable
                            && reference
                                .items
                                .iter()
                                .any(|item| item.id == tank.item && item.body == body.id)
                    })
                    .map(|tank| tank.item)
                    .collect();
                if tanks.is_empty() {
                    return None;
                }
                let volume: f32 = tanks
                    .iter()
                    .map(|id| {
                        items
                            .get(*id)
                            .unwrap()
                            .fluid
                            .as_ref()
                            .unwrap()
                            .mixture
                            .total()
                    })
                    .sum();
                Some((bodies[&body.id], body.mass_kg - volume, tanks, body.mass_kg))
            })
            .collect();
        Ok(Self {
            bodies,
            items,
            interaction,
            player,
            origin,
            attachments,
            render_poses,
            item_render_poses,
            compound_colliders,
            portable_mass,
        })
    }

    pub fn sync_origin(&mut self, origin: [f64; 3]) {
        let shift = std::array::from_fn(|i| (origin[i] - self.origin[i]) as f32);
        self.player.rebase(shift);
        self.interaction.rebase(shift);
        self.items.rebase(shift);
        for (_, previous, current) in &mut self.render_poses {
            for i in 0..3 {
                previous.position[i] -= shift[i];
                current.position[i] -= shift[i];
            }
        }
        for (_, previous, current) in &mut self.item_render_poses {
            for i in 0..3 {
                previous.position[i] -= shift[i];
                current.position[i] -= shift[i];
            }
        }
        self.origin = origin;
    }

    pub fn begin_step(&mut self) {
        for (_, previous, current) in &mut self.render_poses {
            *previous = *current;
        }
        for (_, previous, current) in &mut self.item_render_poses {
            *previous = *current;
        }
    }
    pub fn sync_portable_mass(&mut self, physics: &mut PhysicsWorld) {
        for (handle, dry, tanks, current) in &mut self.portable_mass {
            let mass = *dry
                + tanks
                    .iter()
                    .map(|id| {
                        self.items
                            .get(*id)
                            .expect("portable tank")
                            .fluid
                            .as_ref()
                            .expect("portable fluid")
                            .mixture
                            .total()
                    })
                    .sum::<f32>();
            if (mass - *current).abs() > 0.0001 {
                physics.rapier_mut().bodies[*handle].set_additional_mass(mass, true);
                *current = mass;
            }
        }
    }
    pub fn sync_compound(&self, physics: &mut PhysicsWorld) {
        let chassis = physics.chassis_handle();
        for record in &self.compound_colliders {
            let local = record.local_pose(&self.interaction);
            let collider = &mut physics.rapier_mut().colliders[record.handle];
            // A removed part carries its shapes on its own body.
            if collider.parent() != Some(chassis) {
                continue;
            }
            if collider.position_wrt_parent() != Some(&local) {
                collider.set_position_wrt_parent(local);
            }
        }
    }
    pub fn commit_poses(&mut self, physics: &PhysicsWorld, teleport: bool) {
        for (id, previous, current) in &mut self.render_poses {
            *current =
                crate::vehicle::from_rapier(physics.rapier().bodies[self.bodies[id]].position());
            if teleport {
                *previous = *current;
            }
        }
        for (id, previous, current) in &mut self.item_render_poses {
            *current = self.items.get(*id).expect("bound visual item").pose;
            if teleport {
                *previous = *current;
            }
        }
    }

    /// Attached bodies without hinges follow the authoritative vehicle pose.
    pub fn sync_attachments(&self, physics: &mut PhysicsWorld, next: bool) {
        for &(handle, parent, local) in &self.attachments {
            let parent_pose = if next {
                *physics.rapier().bodies[parent].next_position()
            } else {
                *physics.rapier().bodies[parent].position()
            };
            let body = &mut physics.rapier_mut().bodies[handle];
            let pose = parent_pose * local;
            if !next {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
        }
    }
}

pub fn belongs_to_vehicle(reference: &ReferenceManifest, mut id: BodyId) -> bool {
    loop {
        if id == reference.vehicle.body {
            return true;
        }
        match reference
            .bodies
            .iter()
            .find(|body| body.id == id)
            .and_then(|body| body.parent_body)
        {
            Some(parent) => id = parent,
            None => return false,
        }
    }
}

pub fn collider_item(reference: &ReferenceManifest, mut node: NodeId) -> Option<ItemId> {
    if let Some(seat) = reference
        .seats
        .iter()
        .find(|seat| seat.hit_nodes.contains(&node))
    {
        return Some(seat.item);
    }
    loop {
        if let Some(item) = reference.items.iter().find(|item| item.node == node) {
            return Some(item.id);
        }
        node = reference
            .nodes
            .iter()
            .find(|record| record.id == node)?
            .parent?;
    }
}

pub fn initial_vehicle_pose(reference: &ReferenceManifest) -> Result<VehiclePose, ReferenceError> {
    node_pose(reference, reference.vehicle.root)
}

pub struct GarageRuntime {
    pub physics: PhysicsWorld,
    pub area: StartingArea,
    pub stream: WorldStream,
    pub engine: EngineState,
    pub engine_output: EngineOutput,
    pub trip_m: f64,
    pub handbrake: bool,
    pub paused: bool,
    pub started: bool,
    pub attempts: u64,
    pub rebases: u32,
    pub notice: &'static str,
    pub target: Option<AimTarget>,
    pub wheels: crate::interaction::wheels::WheelAssemblies,
    pub parts: PartSlots,
    /// Seconds into the source day cycle.
    pub day_seconds: f32,
    /// Terrain color at the player. The source samples it once per second.
    pub terrain_color: [f32; 4],
    /// `carscript.lampak`: off, low beam, or high beam.
    pub lamps: usize,
    pub headlights: Vec<crate::simulation::lights::Headlight>,
    color_steps: u32,
    ignition_held: bool,
    auto_gear: usize,
    recovery_bounds: crate::reference::Bounds,
    recovery_bodies: Vec<RigidBodyHandle>,
    last_speed_kmh: f32,
    crash_samples: u64,
    crash_parts: Vec<ItemId>,
    /// Vehicle mass without the car item and its mounted items.
    base_mass_kg: f32,
    mass_kg: f32,
}

impl GarageRuntime {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut physics = PhysicsWorld::new(reference, initial_vehicle_pose(reference)?)?;
        let mut area = StartingArea::new(reference, &mut physics, [0.0; 3])?;
        let mut stream = WorldStream::new(reference);
        stream.sync_at(
            reference.area.player_spawn.translation.map(|v| v as f64),
            &mut physics,
        );
        crate::world::sync_station_supplies(
            &mut stream,
            &mut area.items,
            &mut area.interaction,
            &mut physics,
        );
        let mut wheels = crate::interaction::wheels::WheelAssemblies::new(reference)?;
        wheels.sync(&mut area.items, &mut physics)?;
        let parts = PartSlots::new(reference, &area.items, &mut physics)?;
        physics.refresh_queries();
        area.items.sync_poses(&physics);
        area.commit_poses(&physics, true);
        let terrain_color = stream.generator.terrain_color(
            stream
                .origin
                .absolute(reference.area.player_spawn.translation),
        );
        let base_mass_kg =
            reference.vehicle.mass_kg - area.items.mounted_mass(reference.vehicle.item);
        let mut recovery_bounds = crate::reference::Bounds {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        };
        for id in &reference.vehicle.colliders {
            let record = reference
                .colliders
                .iter()
                .find(|record| record.id == *id)
                .expect("vehicle shape");
            if record.sensor || !record.enabled {
                continue;
            }
            let aabb = collider_builder(&record.geometry)?.build().compute_aabb();
            for i in 0..3 {
                recovery_bounds.min[i] = recovery_bounds.min[i].min(aabb.mins[i]);
                recovery_bounds.max[i] = recovery_bounds.max[i].max(aabb.maxs[i]);
            }
        }
        Ok(Self {
            physics,
            area,
            stream,
            engine: EngineState::new(&reference.engine),
            engine_output: EngineOutput::default(),
            trip_m: 0.0,
            handbrake: true,
            paused: false,
            started: false,
            attempts: 0,
            rebases: 0,
            notice: "",
            target: None,
            wheels,
            parts,
            day_seconds: reference.day_cycle.start_seconds,
            terrain_color,
            lamps: 0,
            headlights: reference
                .headlights
                .iter()
                .map(crate::simulation::lights::Headlight::new)
                .collect(),
            color_steps: 0,
            ignition_held: false,
            auto_gear: reference
                .vehicle
                .gear_ratios
                .iter()
                .position(|ratio| *ratio == 0.0)
                .expect("validated neutral"),
            recovery_bounds,
            recovery_bodies: Vec::with_capacity(reference.items.len()),
            last_speed_kmh: 0.0,
            crash_samples: 0,
            crash_parts: Vec::with_capacity(reference.items.len()),
            base_mass_kg,
            mass_kg: reference.vehicle.mass_kg,
        })
    }

    pub fn step(&mut self, reference: &ReferenceManifest, input: FrameInput) {
        if !self.started || self.paused {
            return;
        }
        let previous = self
            .stream
            .origin
            .absolute(self.physics.snapshot().pose.position);
        self.area.begin_step();
        let target = self.area.interaction.aim(
            &self.physics,
            self.area.player.view_pose(&self.physics),
            reference.player.aim_reach,
        );
        self.target = target;
        let seated = self.area.player.seat().is_some();
        let action = self.area.interaction.step(
            input,
            target,
            &mut self.area.player,
            &mut self.area.items,
            &mut self.physics,
            FIXED_DT,
        );
        match action {
            Some(InteractionAction::WheelRemove(item)) => {
                if let Some(slot) = self.wheels.slot_for_item(item) {
                    self.notice = if self
                        .wheels
                        .remove(slot, &mut self.area.items, &mut self.physics)
                        .is_ok()
                    {
                        ""
                    } else {
                        "Cannot remove this wheel."
                    };
                }
            }
            Some(InteractionAction::WheelInstall { item, target }) => {
                self.area
                    .interaction
                    .drop_held(&mut self.area.items, &mut self.physics);
                self.notice = if self
                    .wheels
                    .install(item, target, &mut self.area.items, &mut self.physics)
                    .is_ok()
                {
                    ""
                } else {
                    "This wheel does not fit the slot."
                };
            }
            Some(InteractionAction::WheelSplit(rim)) => {
                if let Ok((_, tire)) =
                    self.wheels
                        .split(rim, &mut self.area.items, &mut self.physics)
                {
                    self.notice = "";
                    let mut part = target.expect("aimed tire");
                    part.item = tire;
                    part.slot = None;
                    self.area.interaction.pickup(
                        part,
                        &self.area.player,
                        &mut self.area.items,
                        &mut self.physics,
                    );
                } else {
                    self.notice = "Cannot dismount this tire.";
                }
            }
            Some(InteractionAction::WheelJoin { rim, tire }) => {
                self.area
                    .interaction
                    .drop_held(&mut self.area.items, &mut self.physics);
                self.notice = if self
                    .wheels
                    .assemble(rim, tire, &mut self.area.items, &mut self.physics)
                    .is_ok()
                {
                    ""
                } else {
                    "This tire does not fit the rim."
                };
            }
            Some(InteractionAction::PartRemove(item)) => {
                self.notice = if self
                    .parts
                    .detach(item, &mut self.area.items, &mut self.physics)
                    .is_ok()
                {
                    ""
                } else {
                    "Cannot remove this part."
                };
            }
            Some(InteractionAction::PartFit(item)) => {
                self.area
                    .interaction
                    .drop_held(&mut self.area.items, &mut self.physics);
                self.notice = if self
                    .parts
                    .fit(item, &mut self.area.items, &mut self.physics)
                    .is_ok()
                {
                    self.area.interaction.close_hinge(item);
                    ""
                } else {
                    "This part does not fit the slot."
                };
            }
            _ => {}
        }
        self.area.sync_portable_mass(&mut self.physics);
        if seated
            && input.pressed & keys::USE != 0
            && matches!(action, None | Some(InteractionAction::Seat(_)))
            && self.area.interaction.dragging_hinge().is_none()
            && self.area.interaction.tuning_opening().is_none()
        {
            self.notice = if self.area.player.seat().is_some() {
                "Look toward a clear exit."
            } else {
                ""
            };
        } else if input.mouse != [0.0; 2] {
            self.notice = "";
        }
        let mut walking_input = input;
        if (input.held & keys::USE != 0 && self.area.interaction.dragging_hinge().is_some())
            || (input.held & keys::USE != 0 && self.area.interaction.tuning_opening().is_some())
            || (input.held & (keys::ROTATE | keys::MOVE) != 0
                && self.area.interaction.held().is_some())
        {
            walking_input.mouse = [0.0; 2];
        }
        self.area
            .player
            .step(walking_input, &mut self.physics, FIXED_DT);
        self.area.sync_attachments(&mut self.physics, true);
        self.area.sync_compound(&mut self.physics);
        let driving = self.area.player.driving();
        if driving && input.pressed & keys::SPACE != 0 {
            self.handbrake = !self.handbrake;
        }
        if driving && input.pressed & keys::LIGHTS != 0 {
            self.lamps = (self.lamps + 1) % crate::simulation::lights::LAMP_STATES;
        }
        let car = reference.vehicle.item;
        for light in &mut self.headlights {
            // Only a fitted headlight follows the switch.
            if self.area.items.mounted(car, light.item) {
                light.switch(self.lamps);
            }
            light.step(FIXED_DT);
        }
        self.day_seconds =
            crate::simulation::day::advance(&reference.day_cycle, self.day_seconds, FIXED_DT);
        let snapshot = self.physics.snapshot();
        let speed = snapshot.speed_mps;
        let axis = ((driving && input.held & keys::FORWARD != 0) as u8 as f32)
            - ((driving && input.held & keys::BACK != 0) as u8 as f32);
        let direction = speed
            * snapshot
                .linear_velocity
                .iter()
                .map(|v| v * v)
                .sum::<f32>()
                .sqrt();
        let throttle = axis != 0.0
            && (direction.abs() <= reference.vehicle.auto_move_min || direction * axis > 0.0);
        let ignition_held = driving
            && (input.held & keys::IGNITION != 0
                || (!reference.vehicle.dont_ignite_with_throttle_in_automatic
                    && throttle
                    && !self.engine_output.running));
        let ignition_pressed = (driving && input.pressed & keys::IGNITION != 0)
            || (ignition_held && !self.ignition_held);
        let ignition_released = !ignition_held
            && (self.ignition_held || (driving && input.released & keys::IGNITION != 0));
        self.ignition_held = ignition_held;
        if ignition_pressed {
            self.attempts = self.attempts.wrapping_add(1);
        }
        let radius = reference.wheel_slots[0].radius;
        let rolling_rpm = self.physics.driven_wheel_rpm();
        let gear = crate::vehicle::select_gear(
            rolling_rpm,
            axis < 0.0,
            throttle,
            self.auto_gear,
            &reference.engine,
            &reference.vehicle,
        );
        if throttle {
            self.auto_gear = gear;
        }
        let ratio = reference.vehicle.gear_ratios[gear].abs() * reference.vehicle.final_drive_ratio;
        let wheel_rpm = rolling_rpm.abs() * ratio;
        let clutch = if ratio == 0.0 {
            0.0
        } else {
            ((self.engine.rpm - reference.engine.auto_shift_clutch_rpm
                + reference.engine.auto_shift_clutch_rpm_range)
                / reference.engine.auto_shift_clutch_rpm_range)
                .clamp(0.0, 1.0)
        };
        let radiator_condition = self
            .area
            .items
            .mounted(car, reference.engine.coolant_tank)
            .then(|| self.area.items.get(reference.engine.coolant_tank))
            .flatten()
            .map(|item| item.condition);
        let was_running = self.engine_output.running;
        if !self.area.items.mounted(car, reference.engine.item) {
            // Without an engine the car has no power and no engine heat.
            self.engine.rpm = 0.0;
            self.engine_output = EngineOutput {
                ignition: self.engine.ignition,
                temperature: self.engine.temperature,
                ..EngineOutput::default()
            };
        } else {
            self.engine_output = self.engine.step(
                EngineInput {
                    ignition_held,
                    ignition_pressed,
                    ignition_released,
                    throttle: throttle as u8 as f32,
                    clutch,
                    wheel_rpm,
                    free_run: reference.vehicle.gear_free_run[gear],
                    speed_kmh: speed * 3.6,
                    ambient_temperature: reference.engine.initial_temperature,
                    ambient_heat_modifier: reference.engine.globals.awake_heat_modifier,
                    radiator_condition,
                },
                self.area.items.vehicle_fluids().expect("bound fluids"),
                &reference.engine,
                start_sample(START_SEED, self.attempts),
                FIXED_DT,
            );
        }
        if self.engine_output.running && !was_running {
            let engine = to_rapier(
                self.area
                    .items
                    .get(reference.engine.item)
                    .expect("engine item")
                    .pose,
            );
            let sequence = self.attempts.wrapping_mul(4);
            let factor = 0.85 + 0.3 * start_sample(START_SEED, sequence);
            let vertical = 2.0 * start_sample(START_SEED, sequence.wrapping_add(1)) - 1.0;
            let angle = std::f32::consts::TAU * start_sample(START_SEED, sequence.wrapping_add(2));
            let radius = start_sample(START_SEED, sequence.wrapping_add(3)).cbrt();
            let horizontal = (1.0 - vertical * vertical).sqrt();
            let offset = rapier3d::math::Vector::new(
                horizontal * angle.cos(),
                vertical,
                horizontal * angle.sin(),
            ) * radius;
            let impulse = engine.rotation
                * rapier3d::math::Vector::Y
                * reference.engine.starting_push_force
                * factor
                * FIXED_DT;
            let chassis = self.physics.chassis_handle();
            self.physics.rapier_mut().bodies[chassis].apply_impulse_at_point(
                impulse,
                engine.translation + offset,
                true,
            );
        }
        let drive = DriveInput {
            accelerate: driving && input.held & keys::FORWARD != 0,
            brake_reverse: driving && input.held & keys::BACK != 0,
            steer: ((driving && input.held & keys::LEFT != 0) as u8 as f32)
                - ((driving && input.held & keys::RIGHT != 0) as u8 as f32),
            handbrake: self.handbrake,
        };
        let drive_count = self.physics.driven_wheel_count();
        self.physics.set_engine_brake(
            self.engine_output.brake_torque_nm * ratio / (drive_count.max(1) as f32 * radius),
        );
        self.physics.apply_vehicle_forces(
            drive,
            crate::vehicle::drive_force(
                self.engine_output,
                &reference.vehicle,
                gear,
                radius,
                drive_count,
            ),
        );
        self.physics.step_physics();
        self.area.interaction.sync_hinges(&mut self.physics);
        self.area.sync_attachments(&mut self.physics, false);
        self.area.sync_compound(&mut self.physics);
        self.wheels
            .sync(&mut self.area.items, &mut self.physics)
            .expect("validated wheel bindings");
        self.damage(reference);
        self.parts
            .sync(
                self.area.interaction.held(),
                &self.area.items,
                &mut self.physics,
            )
            .expect("validated part bindings");
        self.sync_mass(reference);
        self.physics.refresh_queries();
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, false);
        let current = self
            .stream
            .origin
            .absolute(self.physics.snapshot().pose.position);
        self.trip_m +=
            ((current[0] - previous[0]).powi(2) + (current[2] - previous[2]).powi(2)).sqrt();
        let focus = if self.area.player.seat().is_some() {
            self.physics.snapshot().pose.position
        } else {
            self.area.player.focus_position(&self.physics)
        };
        let absolute = self.stream.origin.absolute(focus);
        self.color_steps += 1;
        if self.color_steps >= 60 {
            self.color_steps = 0;
            self.terrain_color = self.stream.generator.terrain_color(absolute);
        }
        if self.stream.rebase(focus, &mut self.physics).is_some() {
            let old_origin = self.area.origin;
            self.area.sync_origin(self.stream.origin.offset);
            if let Some(target) = &mut self.target {
                for i in 0..3 {
                    target.point[i] -= (self.area.origin[i] - old_origin[i]) as f32;
                }
            }
            self.rebases += 1;
        }
        self.stream.sync_at(absolute, &mut self.physics);
        crate::world::sync_station_supplies(
            &mut self.stream,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.physics,
        );
    }

    /// Source crash damage. A hard speed change knocks mounted parts off.
    fn damage(&mut self, reference: &ReferenceManifest) {
        let speed = self.physics.snapshot().speed_mps.abs() * 3.6;
        let change = (speed - self.last_speed_kmh).abs();
        self.last_speed_kmh = speed;
        let car = reference.vehicle.item;
        let samples = [self.crash_sample(), self.crash_sample()];
        let Some(crash) = crate::simulation::crash::crash(
            &reference.vehicle.crash,
            change,
            self.area.items.get(car).map_or(0, |item| item.condition),
            samples,
        ) else {
            return;
        };
        self.crash_parts.clear();
        for record in &reference.items {
            let items = &self.area.items;
            if let (Some(power), Some(item)) = (record.fall_off_power, items.get(record.id))
                && (matches!(item.location, ItemLocation::WheelSlot(_))
                    || self.parts.contains(record.id))
                && items.mounted(car, record.id)
                && crate::simulation::crash::falls_off(power, item.condition, crash.lerp)
            {
                self.crash_parts.push(record.id);
            }
        }
        for _ in 0..crash.count.min(self.crash_parts.len()) {
            let index = ((self.crash_sample() * self.crash_parts.len() as f32) as usize)
                .min(self.crash_parts.len() - 1);
            let id = self.crash_parts.swap_remove(index);
            match self.area.items.get(id).map(|item| item.location) {
                Some(ItemLocation::WheelSlot(slot)) => {
                    self.wheels
                        .remove(slot, &mut self.area.items, &mut self.physics)
                        .expect("mounted wheel");
                }
                // A part on a removed door stays on that door.
                _ if self.area.items.mounted(car, id) => {
                    self.parts
                        .detach(id, &mut self.area.items, &mut self.physics)
                        .expect("fitted part");
                }
                _ => {}
            }
        }
    }

    fn sync_mass(&mut self, reference: &ReferenceManifest) {
        let mass = self.base_mass_kg + self.area.items.mounted_mass(reference.vehicle.item);
        if (mass - self.mass_kg).abs() > 0.0001 {
            self.physics.set_vehicle_mass(mass);
            self.mass_kg = mass;
        }
    }

    fn crash_sample(&mut self) -> f32 {
        self.crash_samples = self.crash_samples.wrapping_add(1);
        start_sample(CRASH_SEED, self.crash_samples)
    }

    pub fn restart(&mut self, reference: &ReferenceManifest) -> Result<(), ReferenceError> {
        self.area
            .interaction
            .drop_held(&mut self.area.items, &mut self.physics);
        *self = Self::new(reference)?;
        self.started = true;
        Ok(())
    }

    pub fn recover(&mut self, reference: &ReferenceManifest) {
        self.target = None;
        let held = self
            .area
            .interaction
            .held()
            .and_then(|item| self.area.items.get(item))
            .and_then(|item| item.body);
        let old_view = to_rapier(self.area.player.view_pose(&self.physics));
        let old_car = to_rapier(self.physics.snapshot().pose);
        let inverse = old_car.inverse();
        self.recovery_bodies.clear();
        for item in self.area.items.iter() {
            let Some(handle) = item.body else {
                continue;
            };
            if Some(handle) == held
                || handle == self.physics.chassis_handle()
                || self.recovery_bodies.contains(&handle)
            {
                continue;
            }
            let body = &self.physics.rapier().bodies[handle];
            if !body.is_dynamic() || !body.is_enabled() {
                continue;
            }
            let point = inverse * body.position().translation;
            if (0..3).all(|i| {
                point[i] >= self.recovery_bounds.min[i] && point[i] <= self.recovery_bounds.max[i]
            }) {
                self.recovery_bodies.push(handle);
            }
        }
        self.area
            .interaction
            .drop_held(&mut self.area.items, &mut self.physics);
        let fuel = self
            .area
            .items
            .get(reference.engine.fuel_tank)
            .and_then(|item| item.fluid.as_ref())
            .map_or(0.0, |fluid| fluid.mixture.total());
        let pose = self.stream.reset_pose(fuel <= 0.0, &mut self.physics);
        crate::world::sync_station_supplies(
            &mut self.stream,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.physics,
        );
        self.physics.reset(pose);
        let cargo_delta = to_rapier(pose) * inverse;
        for &body in &self.recovery_bodies {
            transport_body(&mut self.physics, body, cargo_delta);
        }
        self.area.interaction.sync_hinges(&mut self.physics);
        self.area.sync_attachments(&mut self.physics, false);
        self.area.sync_compound(&mut self.physics);
        self.wheels
            .sync(&mut self.area.items, &mut self.physics)
            .expect("validated wheel bindings");
        self.parts
            .sync(
                self.area.interaction.held(),
                &self.area.items,
                &mut self.physics,
            )
            .expect("validated part bindings");
        self.last_speed_kmh = 0.0;
        self.physics.refresh_queries();
        self.area.items.sync_poses(&self.physics);
        if self.area.player.seat().is_none() {
            let old_player = self.area.player.body_handle();
            let standing = rapier3d::prelude::SharedShape::capsule_y(
                reference.player.standing_height * 0.5 - reference.player.standing_radius,
                reference.player.standing_radius,
            );
            let mut clear = None;
            for distance in [3.0, 4.0, 5.0, 6.0] {
                for side in [-1.0, 1.0] {
                    let mut candidate = to_rapier(pose);
                    candidate.translation += candidate.rotation
                        * rapier3d::math::Vector::new(
                            distance * side,
                            reference.player.standing_height * 0.5 - 1.1 + 0.05,
                            0.0,
                        );
                    if self
                        .physics
                        .queries(
                            rapier3d::prelude::QueryFilter::default()
                                .exclude_sensors()
                                .exclude_rigid_body(old_player),
                        )
                        .intersect_shape(candidate, standing.as_ref())
                        .next()
                        .is_none()
                    {
                        clear = Some(crate::vehicle::from_rapier(&candidate));
                        break;
                    }
                }
                if clear.is_some() {
                    break;
                }
            }
            if let Some(clear) = clear {
                self.physics.rapier_mut().remove_body(old_player);
                self.area.player = PlayerState::new(&reference.player, clear, &mut self.physics);
                self.area.player.configure_seats(&reference.seats);
                self.area
                    .interaction
                    .bind_player_body(self.area.player.body_handle());
            }
        }
        if let Some(body) = held {
            let player_delta =
                to_rapier(self.area.player.view_pose(&self.physics)) * old_view.inverse();
            transport_body(&mut self.physics, body, player_delta);
        }
        self.physics.refresh_queries();
        self.area.items.sync_poses(&self.physics);
        self.area.commit_poses(&self.physics, true);
    }
}

fn transport_body(
    physics: &mut PhysicsWorld,
    handle: RigidBodyHandle,
    delta: rapier3d::math::Pose,
) {
    let pose = delta * *physics.rapier().bodies[handle].position();
    let body = &mut physics.rapier_mut().bodies[handle];
    body.set_position(pose, true);
    body.set_linvel(rapier3d::math::Vector::ZERO, true);
    body.set_angvel(rapier3d::math::Vector::ZERO, true);
}

fn start_sample(seed: u64, attempt: u64) -> f32 {
    (hash(seed ^ attempt) >> 40) as f32 / 16_777_216.0
}

fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
