mod mass;
mod mutations;
mod presentation_bodies;
mod query_bounds;
mod wheel;

use presentation_bodies::{BodyPresentationState, PresentationBodies};
use query_bounds::QueryBounds;
use rapier3d::math::{Pose, Rotation, Vector};
pub use rapier3d::prelude::ColliderHandle;
use rapier3d::prelude::{
    ActiveEvents, ColliderBuilder, ColliderSet, CollisionEvent, ContactPair, EventHandler, Group,
    InteractionGroups, InteractionTestMode, QueryFilter, QueryPipeline, Ray, RigidBodyBuilder,
    RigidBodyHandle, RigidBodySet,
};
pub use wheel::WheelAssembly;
use wheel::{WheelContact, WheelForce, WheelFrame, WheelSlip, WheelState, WheelTorques};

/// Tire impulse solver passes per physics step.
const TIRE_ITERATIONS: usize = 4;

use crate::reference::{
    ColliderGeometry, ReferenceError, ReferenceManifest, VehicleReference, WheelSlotId,
    WheelSlotReference,
};
use crate::simulation::{DriveInput, FIXED_DT};

/// Axle torque after gear ratios and engine engagement. Motor torque has a sign.
#[derive(Clone, Copy, Debug, Default)]
pub struct DriveTorques {
    pub motor_nm: f32,
    pub engine_brake_nm: f32,
}

pub const VEHICLE_COLLIDER_FLAG: u128 = 1 << 127;

/// Rapier 0.36 CCD sweeps disabled colliders too. A disabled collider below the
/// ground then clamps its body's motion each step. Empty solver groups stop this.
pub fn solver_groups(enabled: bool) -> InteractionGroups {
    if enabled {
        InteractionGroups::all()
    } else {
        InteractionGroups::none()
    }
}

/// Enables or disables a collider. See [`solver_groups`].
pub fn set_collider_enabled(collider: &mut rapier3d::prelude::Collider, enabled: bool) {
    collider.set_enabled(enabled);
    collider.set_solver_groups(solver_groups(enabled));
}
pub const SLOT_SENSOR_FLAG: u128 = 1 << 126;
/// Collider user data bits 32 to 34 hold the `StepSurface` code. 0 is none.
pub const STEP_SURFACE_SHIFT: u32 = 32;
pub const STEP_SURFACE_MASK: u128 = 0b111 << STEP_SURFACE_SHIFT;
pub fn step_surface_bits(surface: Option<crate::reference::StepSurface>) -> u128 {
    surface.map_or(0, |s| (s as u128) << STEP_SURFACE_SHIFT)
}
/// Collider user data bits 35 to 38 hold the source tag code. 0 is none.
pub const TAG_SHIFT: u32 = 35;
pub const TAG_MASK: u128 = 0b1111 << TAG_SHIFT;
pub fn tag_bits(tag: Option<crate::reference::ColliderTag>) -> u128 {
    tag.map_or(0, |t| (t as u128 + 1) << TAG_SHIFT)
}
pub fn collider_tag(user_data: u128) -> Option<crate::reference::ColliderTag> {
    let code = ((user_data & TAG_MASK) >> TAG_SHIFT) as usize;
    code.checked_sub(1)
        .and_then(|i| crate::reference::COLLIDER_TAGS.get(i).copied())
}

/// One started contact. `speed` is the relative speed at `point` before the solver.
#[derive(Clone, Copy, Debug)]
pub struct HitContact {
    pub colliders: [ColliderHandle; 2],
    pub point: [f32; 3],
    pub speed: f32,
}
const HIT_CAPACITY: usize = 64;

/// Records started contacts of colliders with `ActiveEvents::COLLISION_EVENTS`.
/// A full buffer drops new hits.
pub struct HitCollector(std::sync::Mutex<Vec<HitContact>>);
impl Default for HitCollector {
    fn default() -> Self {
        Self(std::sync::Mutex::new(Vec::with_capacity(HIT_CAPACITY)))
    }
}
impl HitCollector {
    /// Moves the recorded hits into `out` without allocation.
    pub fn take(&self, out: &mut Vec<HitContact>) {
        let mut hits = self.0.lock().unwrap();
        out.clear();
        std::mem::swap(out, &mut *hits);
    }
}
impl EventHandler for HitCollector {
    fn handle_collision_event(
        &self,
        bodies: &RigidBodySet,
        colliders: &ColliderSet,
        event: CollisionEvent,
        pair: Option<&ContactPair>,
    ) {
        let (CollisionEvent::Started(a, b, _), Some(pair)) = (event, pair) else {
            return;
        };
        let (Some(first), Some(second)) = (colliders.get(a), colliders.get(b)) else {
            return;
        };
        let Some(point) = pair.manifolds().iter().find_map(|m| {
            let contact = m.points.first()?;
            let pose = colliders.get(pair.collider1)?.position();
            let pose = match m.subshape_poses.as_ref().and_then(|s| s.pos1) {
                Some(sub) => *pose * sub,
                None => *pose,
            };
            Some(pose * contact.local_p1)
        }) else {
            return;
        };
        let velocity = |collider: &rapier3d::prelude::Collider| {
            collider
                .parent()
                .and_then(|h| bodies.get(h))
                .map_or(Vector::ZERO, |body| body.velocity_at_point(point))
        };
        let mut hits = self.0.lock().unwrap();
        if hits.len() < HIT_CAPACITY {
            hits.push(HitContact {
                colliders: [a, b],
                point: point.to_array(),
                speed: (velocity(first) - velocity(second)).length(),
            });
        }
    }
    fn handle_contact_force_event(
        &self,
        _: f32,
        _: &RigidBodySet,
        _: &ColliderSet,
        _: &ContactPair,
        _: f32,
    ) {
    }
    fn handle_soft_body_tear_event(
        &self,
        _: &rapier3d::prelude::SoftBodySet,
        _: &rapier3d::prelude::SoftBodyTearEvent,
    ) {
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehiclePose {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct VehicleSnapshot {
    pub pose: VehiclePose,
    pub wheels: [VehiclePose; 4],
    pub speed_mps: f32,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub wheels_contact: [bool; 4],
}

impl VehicleSnapshot {
    pub fn absolute_speed_kmh(&self) -> f32 {
        self.linear_velocity
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt()
            * 3.6
    }
}

/// One rigid chassis with its four source wheel slots.
struct VehicleBody {
    chassis: RigidBodyHandle,
    slip: WheelSlip,
    wheels: [WheelState; 4],
    wheel_references: [WheelSlotReference; 4],
    max_steer: f32,
    brake_torque: f32,
    handbrake_torque: f32,
    assemblies: [WheelAssembly; 4],
    last_handbrake: f32,
    /// Mass of items locked to the chassis. `set_vehicle_mass` adds it.
    cargo_mass: f32,
}

pub struct PhysicsWorld {
    body_mass_override: Option<mass::BodyMassOverride>,
    /// Locked item mass on non-vehicle bodies. `set_body_mass` adds it.
    body_cargo: Vec<(RigidBodyHandle, f32)>,
    pub(crate) inactive_bodies: std::collections::HashSet<RigidBodyHandle>,
    physics: rapier3d::prelude::PhysicsWorld,
    query_bounds: QueryBounds,
    presentation_bodies: PresentationBodies,
    chassis: Vec<RigidBodyHandle>,
    vehicles: Vec<VehicleBody>,
    hit_collector: HitCollector,
    hits: Vec<HitContact>,
    ignored: mutations::IgnoredCollisions,
}

impl PhysicsWorld {
    /// Poses follow the order of `reference.vehicles`.
    pub fn new(
        reference: &ReferenceManifest,
        poses: &[VehiclePose],
    ) -> Result<Self, ReferenceError> {
        if poses.len() != reference.vehicles.len() {
            return Err(ReferenceError("Expected one pose per vehicle".into()));
        }
        let mut physics = Self::empty(
            poses.len(),
            reference.bodies.len() + reference.parts.len() + reference.wheel_parts.len(),
        );
        for (vehicle, pose) in reference.vehicles.iter().zip(poses) {
            let vehicle = VehicleBody::new(reference, vehicle, *pose, &mut physics)?;
            physics.chassis.push(vehicle.chassis);
            physics.vehicles.push(vehicle);
        }
        physics.refresh_queries();
        Ok(physics)
    }
    /// A world without vehicles.
    pub(crate) fn empty(vehicles: usize, bodies: usize) -> Self {
        let mut physics = Self {
            physics: rapier3d::prelude::PhysicsWorld::new(),
            body_mass_override: None,
            body_cargo: Vec::with_capacity(mass::CARGO_BODIES),
            query_bounds: QueryBounds::default(),
            presentation_bodies: PresentationBodies::default(),
            chassis: Vec::with_capacity(vehicles),
            vehicles: Vec::with_capacity(vehicles),
            hit_collector: HitCollector::default(),
            hits: Vec::with_capacity(HIT_CAPACITY),
            ignored: Default::default(),
            inactive_bodies: std::collections::HashSet::with_capacity(bodies),
        };
        physics.physics.integration_parameters.dt = FIXED_DT;
        physics
    }
}

impl VehicleBody {
    fn new(
        reference: &ReferenceManifest,
        vehicle: &VehicleReference,
        pose: VehiclePose,
        physics: &mut PhysicsWorld,
    ) -> Result<Self, ReferenceError> {
        let source_body = reference
            .bodies
            .iter()
            .find(|body| body.id == vehicle.body)
            .ok_or_else(|| ReferenceError("Missing chassis body".into()))?;
        let chassis = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .pose(to_rapier(pose))
                .linear_damping(source_body.linear_damping)
                .angular_damping(source_body.angular_damping)
                .ccd_enabled(true),
        );
        for id in &vehicle.colliders {
            let collider = reference
                .colliders
                .iter()
                .find(|record| record.id == *id)
                .ok_or_else(|| ReferenceError(format!("Missing vehicle collider {}", id.0)))?;
            let builder = collider_builder(&collider.geometry)?
                .friction(collider.friction)
                .restitution(collider.restitution)
                .sensor(collider.sensor)
                .enabled(collider.enabled)
                .solver_groups(solver_groups(collider.enabled))
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(collider.membership),
                    Group::from_bits_truncate(collider.filter),
                    InteractionTestMode::And,
                ))
                .density(if collider.sensor { 0.0 } else { 1.0 })
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .user_data(
                    VEHICLE_COLLIDER_FLAG
                        | ((id.0 as u128) << 64)
                        | step_surface_bits(collider.step_surface)
                        | tag_bits(collider.tag),
                );
            physics.insert_collider_with_parent(builder, chassis);
        }
        physics.recompute_body_mass(chassis);
        let geometry_mass = physics.rapier().bodies[chassis].mass();
        if geometry_mass <= 0.0 || geometry_mass >= vehicle.mass_kg {
            return Err(ReferenceError(
                "Invalid car collision volume or mass".into(),
            ));
        }
        physics.edit_body(chassis, |body| {
            body.set_additional_mass(vehicle.mass_kg - geometry_mass, true)
        });
        physics.recompute_body_mass(chassis);
        let slip = WheelSlip {
            tire: vehicle.wheel_slip,
            rim: vehicle.rim_slip,
        };
        let mut wheel_records = Vec::with_capacity(4);
        let mut wheels = Vec::with_capacity(4);
        let mut assemblies = [WheelAssembly::Hub; 4];
        for (index, id) in vehicle.wheel_slots.iter().enumerate() {
            let wheel = reference
                .wheel_slots
                .iter()
                .find(|record| record.id == *id)
                .ok_or_else(|| ReferenceError(format!("Missing wheel slot {}", id.0)))?;
            let anchor = reference
                .anchors
                .iter()
                .find(|record| record.id == wheel.anchor)
                .ok_or_else(|| {
                    ReferenceError(format!("Missing wheel anchor {}", wheel.anchor.0))
                })?;
            let rotation = Rotation::from_xyzw(
                anchor.transform.rotation[0],
                anchor.transform.rotation[1],
                anchor.transform.rotation[2],
                anchor.transform.rotation[3],
            );
            let connection = Vector::from_array(anchor.transform.translation)
                + rotation * Vector::from_array(wheel.center);
            // Wheel assembly sync replaces this before the first drive step.
            assemblies[index] = WheelAssembly::Tire {
                radius_m: wheel.radius,
                mass_kg: wheel.wheel_mass_kg,
                quality: 1.0,
            };
            wheels.push(WheelState::new(
                connection,
                wheel::resolve_profile(wheel, &slip, assemblies[index]),
                wheel.suspension_distance * wheel.target_position,
            ));
            wheel_records.push(wheel.clone());
        }
        Ok(Self {
            chassis,
            slip,
            wheels: wheels
                .try_into()
                .map_err(|_| ReferenceError("Expected four wheels".into()))?,
            wheel_references: wheel_records
                .try_into()
                .map_err(|_| ReferenceError("Expected four wheels".into()))?,
            max_steer: vehicle.max_steer_degrees.to_radians(),
            brake_torque: vehicle.brake_torque_nm,
            handbrake_torque: vehicle.handbrake_torque_nm,
            assemblies,
            last_handbrake: vehicle.handbrake as u8 as f32,
            cargo_mass: 0.0,
        })
    }
}

impl PhysicsWorld {
    /// A world with no vehicles and no gravity.
    #[cfg(test)]
    pub(crate) fn empty_for_tests() -> Self {
        let mut physics = rapier3d::prelude::PhysicsWorld::new();
        physics.integration_parameters.dt = FIXED_DT;
        physics.gravity = Vector::ZERO;
        Self {
            body_mass_override: None,
            body_cargo: Vec::new(),
            inactive_bodies: Default::default(),
            physics,
            query_bounds: QueryBounds::default(),
            presentation_bodies: Default::default(),
            chassis: Vec::new(),
            vehicles: Vec::new(),
            hit_collector: Default::default(),
            hits: Vec::new(),
            ignored: Default::default(),
        }
    }
    pub fn vehicle_count(&self) -> usize {
        self.vehicles.len()
    }

    /// Returns the vehicle whose chassis is `body`.
    pub fn vehicle_of(&self, body: RigidBodyHandle) -> Option<usize> {
        self.chassis.iter().position(|chassis| *chassis == body)
    }

    /// Returns the vehicle that owns the wheel slot.
    pub fn slot_vehicle(&self, slot: WheelSlotId) -> Option<usize> {
        self.vehicles.iter().position(|vehicle| {
            vehicle
                .wheel_references
                .iter()
                .any(|reference| reference.id == slot)
        })
    }

    pub fn apply_vehicle_forces(&mut self, index: usize, input: DriveInput, torques: DriveTorques) {
        let drive_count = self.driven_wheel_count(index).max(1) as f32;
        let vehicle = &mut self.vehicles[index];
        let pedal = |value: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        let brake = pedal(input.brake);
        let handbrake = pedal(input.handbrake);
        let motor_nm = if torques.motor_nm.is_finite() {
            torques.motor_nm
        } else {
            0.0
        };
        let engine_brake_nm = if torques.engine_brake_nm.is_finite() {
            torques.engine_brake_nm.max(0.0)
        } else {
            0.0
        };
        if pedal(input.throttle) > 0.0
            || brake > 0.0
            || motor_nm != 0.0
            || input.steer != 0.0
            || handbrake != vehicle.last_handbrake
        {
            self.query_bounds.mark_body(vehicle.chassis);
            let body = &mut self.physics.bodies[vehicle.chassis];
            let before = BodyPresentationState::capture(body);
            body.wake_up(true);
            self.presentation_bodies
                .mark_if_changed(vehicle.chassis, before, body);
        }
        vehicle.last_handbrake = handbrake;
        if self.physics.bodies[vehicle.chassis].is_sleeping() {
            return;
        }
        let steering = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0) * vehicle.max_steer
        } else {
            0.0
        };
        // Source carscript.SetWheel: driven wheels take motor torque and engine brake.
        // Service brake and handbrake add to the wheels in their source lists.
        let wheel_torques: [WheelTorques; 4] = std::array::from_fn(|slot| {
            let reference = &vehicle.wheel_references[slot];
            let mut torques = WheelTorques::default();
            if reference.drive {
                torques.motor_nm = motor_nm / drive_count;
                torques.brake_nm = engine_brake_nm / drive_count;
            }
            if reference.brake {
                torques.brake_nm += vehicle.brake_torque * brake;
            }
            if reference.handbrake {
                torques.brake_nm += vehicle.handbrake_torque * handbrake;
            }
            torques
        });
        let own = vehicle.chassis;
        let chassis = &self.chassis;
        // Wheels hit other vehicles' bodies but never their own vehicle.
        let not_own = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            let parent = collider.parent();
            parent != Some(own)
                && (collider.user_data & VEHICLE_COLLIDER_FLAG == 0
                    || parent.is_some_and(|parent| chassis.contains(&parent)))
        };
        let pose = *self.physics.bodies[own].position();
        let up = pose.rotation * Vector::Y;
        {
            let queries = QueryPipeline {
                dispatcher: self.physics.narrow_phase.query_dispatcher(),
                bvh: &self.query_bounds.bvh,
                bodies: &self.physics.bodies,
                colliders: &self.physics.colliders,
                filter: QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(own)
                    .predicate(&not_own),
            };
            for (wheel, reference) in vehicle.wheels.iter_mut().zip(&vehicle.wheel_references) {
                let travel = reference.suspension_distance;
                let radius = wheel.profile.radius;
                let origin = pose * wheel.connection;
                let hit =
                    queries.cast_ray_and_get_normal(&Ray::new(origin, -up), travel + radius, true);
                wheel.steering = if reference.steer { steering } else { 0.0 };
                wheel.previous_angle = wheel.angle;
                wheel.contact = hit.map(|(collider, hit)| WheelContact {
                    collider,
                    point: origin - up * hit.time_of_impact,
                    normal: if hit.time_of_impact > 0.0 && hit.normal != Vector::ZERO {
                        hit.normal
                    } else {
                        up
                    },
                });
                wheel.suspension_length = hit.map_or(travel, |(_, hit)| {
                    (hit.time_of_impact - radius).clamp(0.0, travel)
                });
            }
        }
        let gravity = self.physics.gravity.length();
        let sprung_mass = self.physics.bodies[own].mass() / 4.0;
        let mut contacts: [Option<(WheelForce, Vector, Option<RigidBodyHandle>)>; 4] = [None; 4];
        for (slot, wheel) in vehicle.wheels.iter_mut().enumerate() {
            let reference = &vehicle.wheel_references[slot];
            let ground = wheel.contact.and_then(|contact| {
                self.physics.colliders[contact.collider]
                    .parent()
                    .filter(|parent| self.physics.bodies[*parent].is_dynamic())
            });
            let point = wheel.contact.map_or(Vector::ZERO, |contact| {
                contact.point + up * reference.force_application_distance_m
            });
            let frame = WheelFrame {
                up,
                axle: pose.rotation * Rotation::from_rotation_y(wheel.steering) * Vector::X,
                velocity: relative_velocity(&self.physics.bodies, own, ground, point),
                sprung_mass,
                gravity,
            };
            contacts[slot] = wheel::solve(wheel, reference, frame, wheel_torques[slot], FIXED_DT)
                .map(|force| (force, point, ground));
        }
        // All wheels see the same pre-impulse state.
        for (force, point, ground) in contacts.iter().flatten() {
            apply_wheel_impulse(
                &mut self.physics.bodies,
                &mut self.query_bounds,
                &mut self.presentation_bodies,
                own,
                *ground,
                *point,
                force.up * (force.load * FIXED_DT),
            );
        }
        // Tire impulses: a few projected Gauss-Seidel passes over the contacts.
        let mut accumulated = [[0.0f32; 2]; 4];
        for _ in 0..TIRE_ITERATIONS {
            for (slot, contact) in contacts.iter().enumerate() {
                let Some((force, point, ground)) = contact else {
                    continue;
                };
                let velocity = relative_velocity(&self.physics.bodies, own, *ground, *point);
                let mut inverse = [
                    inverse_mass_at(&self.physics.bodies[own], *point, force.forward),
                    inverse_mass_at(&self.physics.bodies[own], *point, force.side),
                ];
                if let Some(ground) = ground {
                    let body = &self.physics.bodies[*ground];
                    inverse[0] += inverse_mass_at(body, *point, force.forward);
                    inverse[1] += inverse_mass_at(body, *point, force.side);
                }
                let needed = [
                    (force.surface_speed - velocity.dot(force.forward)) / inverse[0],
                    -velocity.dot(force.side) / inverse[1],
                ];
                let limits = force.limits(FIXED_DT);
                let previous = accumulated[slot];
                let next = force.ellipse(
                    std::array::from_fn(|axis| {
                        (previous[axis] + needed[axis]).clamp(limits[axis].0, limits[axis].1)
                    }),
                    FIXED_DT,
                );
                accumulated[slot] = next;
                let delta =
                    force.forward * (next[0] - previous[0]) + force.side * (next[1] - previous[1]);
                apply_wheel_impulse(
                    &mut self.physics.bodies,
                    &mut self.query_bounds,
                    &mut self.presentation_bodies,
                    own,
                    *ground,
                    *point,
                    delta,
                );
            }
        }
    }

    /// Started contacts of the last step.
    pub fn hits(&self) -> &[HitContact] {
        &self.hits
    }

    pub fn body_active(&self, handle: RigidBodyHandle) -> bool {
        !self.inactive_bodies.contains(&handle)
    }

    pub fn step_physics(&mut self) {
        for body in self.physics.islands.active_bodies() {
            self.query_bounds.mark_simulated_body(body);
            self.presentation_bodies.mark(body);
        }
        self.physics
            .step_with_events(&self.ignored, &self.hit_collector);
        self.hit_collector.take(&mut self.hits);
        for body in self.physics.islands.active_bodies() {
            self.query_bounds.mark_simulated_body(body);
            self.presentation_bodies.mark(body);
        }
        self.query_bounds.requeue_step_mutations();
        self.query_bounds.refresh(&mut self.physics);
        self.query_bounds.finish_step();
    }

    pub fn capture_save(
        &self,
        vehicle: usize,
        identity: crate::save::BodyIdentity,
        origin: &crate::world::WorldOrigin,
    ) -> crate::save::BodySnapshot {
        capture_body_save(
            &self.physics.bodies[self.chassis_handle(vehicle)],
            identity,
            origin,
        )
    }
    pub(crate) fn restore_wheel_save(
        &mut self,
        vehicle: usize,
        index: usize,
        saved: &crate::save::WheelAttachment,
    ) {
        let wheel = &mut self.vehicles[vehicle].wheels[index];
        wheel.angle = saved.rotation;
        wheel.omega = saved.rpm * std::f32::consts::TAU / 60.0;
        wheel.previous_angle = saved.previous_rotation;
    }
    pub(crate) fn wheel_save_state(&self, vehicle: usize, index: usize) -> (f32, f32, f32) {
        let wheel = &self.vehicles[vehicle].wheels[index];
        (wheel.angle, wheel.rpm(), wheel.previous_angle)
    }

    /// Source WheelCollider rpm, radius and isGrounded of one wheel slot.
    pub fn wheel_motion(&self, vehicle: usize, wheel: usize) -> (f32, f32, bool) {
        let w = &self.vehicles[vehicle].wheels[wheel];
        (w.rpm(), w.profile.radius, w.contact.is_some())
    }

    /// Source carscript avgRpm: all driven slots over the driven rim count.
    pub fn driven_wheel_rpm(&self, index: usize) -> f32 {
        let vehicle = &self.vehicles[index];
        let sum: f32 = vehicle
            .wheels
            .iter()
            .zip(&vehicle.wheel_references)
            .filter(|(_, reference)| reference.drive)
            .map(|(wheel, _)| wheel.rpm())
            .sum();
        sum / self.driven_wheel_count(index).max(1) as f32
    }

    /// Source carscript.DrivenWheelsNum2: driven slots with a mounted rim.
    pub fn driven_wheel_count(&self, index: usize) -> usize {
        let vehicle = &self.vehicles[index];
        vehicle
            .assemblies
            .iter()
            .zip(&vehicle.wheel_references)
            .filter(|(assembly, reference)| reference.drive && assembly.has_rim())
            .count()
    }

    /// Sets the vehicle mass without locked cargo. The cargo mass is added.
    pub fn set_vehicle_mass(&mut self, index: usize, mass_kg: f32) {
        let vehicle = &mut self.vehicles[index];
        let mass_kg = mass_kg + vehicle.cargo_mass;
        let geometry_mass: f32 = self.physics.bodies[vehicle.chassis]
            .colliders()
            .iter()
            .map(|handle| self.physics.colliders[*handle].mass())
            .sum();
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);
        body.set_additional_mass((mass_kg - geometry_mass).max(0.0), true);
        body.recompute_mass_properties_from_colliders(&self.physics.colliders);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
    }

    pub fn set_wheel(
        &mut self,
        slot: WheelSlotId,
        assembly: WheelAssembly,
    ) -> Result<(), ReferenceError> {
        let vehicle = self
            .slot_vehicle(slot)
            .map(|index| &mut self.vehicles[index])
            .ok_or_else(|| ReferenceError("Unknown wheel slot".into()))?;
        let index = vehicle
            .wheel_references
            .iter()
            .position(|reference| reference.id == slot)
            .unwrap();
        vehicle.assemblies[index] = assembly;
        vehicle.wheels[index].profile =
            wheel::resolve_profile(&vehicle.wheel_references[index], &vehicle.slip, assembly);
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);
        body.wake_up(true);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
        Ok(())
    }

    pub fn rapier(&self) -> &rapier3d::prelude::PhysicsWorld {
        &self.physics
    }

    pub fn chassis_handle(&self, index: usize) -> RigidBodyHandle {
        self.chassis[index]
    }

    pub fn refresh_queries(&mut self) {
        self.query_bounds.refresh(&mut self.physics);
    }

    pub(crate) fn query_storage_bytes(&self) -> usize {
        self.query_bounds.retained_bytes()
    }

    pub fn queries<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        QueryPipeline {
            dispatcher: self.physics.narrow_phase.query_dispatcher(),
            bvh: &self.query_bounds.bvh,
            bodies: &self.physics.bodies,
            colliders: &self.physics.colliders,
            filter,
        }
    }

    /// Player movement queries. Shape-cast normals come from contact manifolds.
    pub fn character_queries<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        QueryPipeline {
            dispatcher: &crate::player::normals::CharacterQueryDispatcher,
            ..self.queries(filter)
        }
    }

    pub fn snapshot(&self, index: usize) -> VehicleSnapshot {
        let vehicle = &self.vehicles[index];
        let body = &self.physics.bodies[vehicle.chassis];
        let pose = body.position();
        let wheels = std::array::from_fn(|i| {
            let wheel = &vehicle.wheels[i];
            let local = wheel.connection - Vector::Y * wheel.suspension_length;
            VehiclePose {
                position: (pose * local).to_array(),
                rotation: (pose.rotation
                    * Rotation::from_rotation_y(wheel.steering)
                    * Rotation::from_rotation_x(-wheel.angle))
                .to_array(),
            }
        });
        VehicleSnapshot {
            pose: from_rapier(pose),
            wheels,
            speed_mps: body.linvel().dot(pose.rotation * -Vector::Z),
            linear_velocity: body.linvel().to_array(),
            angular_velocity: body.angvel().to_array(),
            wheels_contact: std::array::from_fn(|i| vehicle.wheels[i].contact.is_some()),
        }
    }

    /// Moves the car and stops it. Wheel angles and spin stay unchanged.
    pub fn reset_preserving_wheels(&mut self, vehicle: usize, pose: VehiclePose) {
        let wheels = self.vehicles[vehicle].wheels;
        self.reset(vehicle, pose);
        self.vehicles[vehicle].wheels = wheels;
        for wheel in &mut self.vehicles[vehicle].wheels {
            wheel.contact = None;
        }
    }

    pub fn reset(&mut self, index: usize, pose: VehiclePose) {
        let vehicle = &mut self.vehicles[index];
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);

        body.set_position(to_rapier(pose), true);
        body.set_linvel(Vector::ZERO, true);
        body.set_angvel(Vector::ZERO, true);
        body.reset_forces(true);
        body.reset_torques(true);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
        for (wheel, reference) in vehicle.wheels.iter_mut().zip(&vehicle.wheel_references) {
            wheel.omega = 0.0;
            wheel.previous_angle = wheel.angle;
            wheel.contact = None;
            wheel.suspension_length = reference.suspension_distance * reference.target_position;
        }
        self.refresh_queries();
    }

    pub fn insert_mesh(&mut self, vertices: &[[f32; 3]], indices: &[[u32; 3]]) -> ColliderHandle {
        self.insert_mesh_at(vertices, indices, [0.0; 3])
    }

    pub fn insert_mesh_at(
        &mut self,
        vertices: &[[f32; 3]],
        indices: &[[u32; 3]],
        position: [f32; 3],
    ) -> ColliderHandle {
        let mesh = ColliderBuilder::trimesh(
            vertices.iter().copied().map(Vector::from_array).collect(),
            indices.to_vec(),
        )
        .expect("Static mesh must contain valid triangles")
        .friction(0.8)
        .translation(Vector::from_array(position));
        let handle = self.insert_collider(mesh);
        self.refresh_queries();
        handle
    }

    /// Fallible source mesh publication leaves existing collision untouched on failure.
    pub fn insert_source_mesh(
        &mut self,
        surface: &crate::world::WorldSurface,
        position: [f32; 3],
        layer: u8,
        filter: u32,
        friction: f32,
        restitution: f32,
        available_cpu: usize,
    ) -> Result<(ColliderHandle, usize), ReferenceError> {
        // Reserve vertices, indices, BVH construction and retained collider storage before allocation.
        let reservation = surface
            .vertices
            .len()
            .saturating_mul(48)
            .saturating_add(surface.triangles.len().saturating_mul(256))
            .saturating_add(std::mem::size_of::<rapier3d::prelude::Collider>());
        if reservation > available_cpu {
            return Err(ReferenceError(format!(
                "Source collision reservation exceeds budget: {reservation}"
            )));
        }
        let mesh = ColliderBuilder::trimesh(
            surface
                .vertices
                .iter()
                .copied()
                .map(Vector::from_array)
                .collect(),
            surface.triangles.clone(),
        )
        .map_err(|error| ReferenceError(format!("Source collision mesh: {error}")))?
        .friction(friction)
        .restitution(restitution)
        .translation(Vector::from_array(position))
        .collision_groups(InteractionGroups::new(
            Group::from_bits_truncate(1u32 << layer),
            Group::from_bits_truncate(filter),
            InteractionTestMode::And,
        ))
        .build();
        let bytes = std::mem::size_of::<rapier3d::prelude::Collider>()
            + mesh
                .shape()
                .as_trimesh()
                .expect("source triangle mesh")
                .total_memory_size();
        if bytes > available_cpu {
            return Err(ReferenceError(format!(
                "Source collision allocation exceeds budget: {bytes}"
            )));
        }
        let handle = self.insert_collider(mesh);
        self.refresh_queries();
        Ok((handle, bytes))
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        for vehicle in &self.vehicles {
            if vehicle.wheels.iter().any(|wheel| {
                wheel
                    .contact
                    .is_some_and(|contact| contact.collider == handle)
            }) {
                self.query_bounds.mark_body(vehicle.chassis);
                let body = &mut self.physics.bodies[vehicle.chassis];
                let before = BodyPresentationState::capture(body);
                body.wake_up(true);
                self.presentation_bodies
                    .mark_if_changed(vehicle.chassis, before, body);
            }
        }
        if self.physics.colliders.contains(handle) {
            self.query_bounds.retire_collider(handle);
        }
        if let Some(parent) = self
            .physics
            .colliders
            .get(handle)
            .and_then(|collider| collider.parent())
        {
            self.query_bounds.mark_body(parent);
        }
        self.physics.remove_collider(handle);
        self.refresh_queries();
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
        let shift = Vector::from_array(shift);
        for (handle, body) in self.physics.bodies.iter_mut() {
            if self.inactive_bodies.contains(&handle) {
                continue;
            }
            self.query_bounds.mark_body(handle);
            let before = BodyPresentationState::capture(body);
            body.set_translation(body.translation() - shift, false);
            self.presentation_bodies
                .mark_if_changed(handle, before, body);
        }
        for (handle, collider) in self.physics.colliders.iter_mut() {
            if collider.parent().is_none() {
                self.query_bounds.mark_collider(handle, false);
                collider.set_translation(collider.translation() - shift);
            }
        }
        // Rapier refreshes private wheel ray points before applying forces next step.
        // Keep suspension lengths, wheel rotations, and velocities unchanged here.
        self.refresh_queries();
    }

    pub fn insert_box(&mut self, center: [f32; 3], half_size: [f32; 3]) -> ColliderHandle {
        let handle = self.insert_collider(
            ColliderBuilder::cuboid(half_size[0], half_size[1], half_size[2])
                .translation(Vector::from_array(center))
                .friction(0.8),
        );
        self.refresh_queries();
        handle
    }

    pub fn collider_count(&self) -> usize {
        self.physics.colliders.len()
    }
}

/// Applies a wheel impulse to the chassis and its reaction to a dynamic ground body.
fn apply_wheel_impulse(
    bodies: &mut rapier3d::prelude::RigidBodySet,
    query_bounds: &mut QueryBounds,
    presentation_bodies: &mut PresentationBodies,
    chassis: RigidBodyHandle,
    ground: Option<RigidBodyHandle>,
    point: Vector,
    impulse: Vector,
) {
    bodies[chassis].apply_impulse_at_point(impulse, point, false);
    if let Some(ground) = ground {
        query_bounds.mark_body(ground);
        let body = &mut bodies[ground];
        let before = BodyPresentationState::capture(body);
        body.apply_impulse_at_point(-impulse, point, true);
        presentation_bodies.mark_if_changed(ground, before, body);
    }
}

/// Chassis velocity minus ground body velocity at `point`.
fn relative_velocity(
    bodies: &rapier3d::prelude::RigidBodySet,
    chassis: RigidBodyHandle,
    ground: Option<RigidBodyHandle>,
    point: Vector,
) -> Vector {
    let velocity = bodies[chassis].velocity_at_point(point);
    ground.map_or(velocity, |ground| {
        velocity - bodies[ground].velocity_at_point(point)
    })
}

/// Inverse effective mass of `body` for an impulse at `point` along `direction`.
fn inverse_mass_at(body: &rapier3d::prelude::RigidBody, point: Vector, direction: Vector) -> f32 {
    let props = body.mass_properties();
    let arm = (point - props.world_com).cross(direction);
    (props.effective_inv_mass * direction * direction).element_sum()
        + arm.dot(props.effective_world_inv_inertia * arm)
}

pub(crate) fn capture_body_save(
    body: &rapier3d::prelude::RigidBody,
    identity: crate::save::BodyIdentity,
    origin: &crate::world::WorldOrigin,
) -> crate::save::BodySnapshot {
    let pose = from_rapier(body.position());
    crate::save::BodySnapshot {
        identity,
        pose: crate::save::AbsolutePose {
            position: origin.absolute(pose.position),
            rotation: pose.rotation,
        },
        linear_velocity: body.linvel().to_array(),
        angular_velocity: body.angvel().to_array(),
        sleeping: body.is_sleeping(),
    }
}

pub fn collider_builder(geometry: &ColliderGeometry) -> Result<ColliderBuilder, ReferenceError> {
    match geometry {
        ColliderGeometry::Box {
            center,
            half_extents,
            rotation,
        } => Ok(
            ColliderBuilder::cuboid(half_extents[0], half_extents[1], half_extents[2]).position(
                to_rapier(VehiclePose {
                    position: *center,
                    rotation: *rotation,
                }),
            ),
        ),
        ColliderGeometry::Sphere { center, radius } => {
            Ok(ColliderBuilder::ball(*radius).translation(Vector::from_array(*center)))
        }
        ColliderGeometry::Capsule { a, b, radius } => Ok(ColliderBuilder::capsule_from_endpoints(
            Vector::from_array(*a),
            Vector::from_array(*b),
            *radius,
        )),
        ColliderGeometry::Convex { points } => {
            let points: Vec<_> = points.iter().copied().map(Vector::from_array).collect();
            ColliderBuilder::convex_hull(&points)
                .ok_or_else(|| ReferenceError("Invalid convex collision geometry".into()))
        }
        ColliderGeometry::Trimesh { points, triangles } => ColliderBuilder::trimesh(
            points.iter().copied().map(Vector::from_array).collect(),
            triangles.clone(),
        )
        .map_err(|error| ReferenceError(format!("Invalid collision mesh: {error}"))),
    }
}

pub fn to_rapier(pose: VehiclePose) -> Pose {
    Pose::from_parts(
        Vector::from_array(pose.position),
        Rotation::from_array(pose.rotation),
    )
}

pub fn from_rapier(pose: &Pose) -> VehiclePose {
    VehiclePose {
        position: pose.translation.to_array(),
        rotation: pose.rotation.to_array(),
    }
}

#[cfg(test)]
mod hit_tests {
    use super::*;

    fn drop_box(events: ActiveEvents) -> Vec<HitContact> {
        let mut world = rapier3d::prelude::PhysicsWorld::new();
        world.integration_parameters.dt = FIXED_DT;
        world.insert_collider(ColliderBuilder::cuboid(5., 0.1, 5.), None);
        world.insert(
            RigidBodyBuilder::dynamic().translation(Vector::new(0., 1.1, 0.)),
            ColliderBuilder::cuboid(0.1, 0.1, 0.1).active_events(events),
        );
        let collector = HitCollector::default();
        let mut hits = Vec::new();
        let mut step = Vec::with_capacity(HIT_CAPACITY);
        for _ in 0..120 {
            world.step_with_events(&(), &collector);
            collector.take(&mut step);
            hits.extend_from_slice(&step);
        }
        hits
    }

    #[test]
    fn dropped_box_reports_one_hit() {
        let hits = drop_box(ActiveEvents::COLLISION_EVENTS);
        assert_eq!(hits.len(), 1, "{hits:?}");
        // A 0.8 m fall gives sqrt(2 g h) = 3.96 m/s. The 50 Hz steps detect it up to 0.04 s late.
        assert!((3.9..4.8).contains(&hits[0].speed), "{hits:?}");
        assert!(hits[0].point[1].abs() < 0.2, "{hits:?}");
        assert!(drop_box(ActiveEvents::empty()).is_empty());
    }
}
