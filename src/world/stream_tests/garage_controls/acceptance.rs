//! P11 starting-area acceptance. The loop uses only keys and mouse input.
use super::*;
use crate::interaction::{AimTarget, ResolvedAim};
use crate::reference::{ActionBinding, TankRole};
use crate::save::{CameraSave, SaveSnapshot};
use crate::simulation::FIXED_DT;
use crate::world::renderer_pending::PipelineContinuation;

struct Play {
    reference: ReferenceManifest,
    run: GarageRuntime,
    claim: Option<PipelineContinuation>,
}

fn wrap_degrees(angle: f32) -> f32 {
    (angle + 180.).rem_euclid(360.) - 180.
}

fn heading_degrees(direction: Vector) -> f32 {
    direction.x.atan2(-direction.z).to_degrees()
}

impl Play {
    fn frame(&mut self, input: FrameInput) {
        self.run.step(&self.reference, input);
        super::super::prepare_step(&mut self.run, &self.reference, &mut self.claim);
    }

    fn hold(&mut self, held: u32, frames: u32) {
        for _ in 0..frames {
            self.frame(FrameInput {
                held,
                ..FrameInput::default()
            });
        }
    }

    fn idle(&mut self, frames: u32) {
        self.hold(0, frames);
    }

    fn tap(&mut self, key: u32) {
        self.frame(FrameInput {
            held: key,
            pressed: key,
            ..FrameInput::default()
        });
        self.frame(FrameInput {
            released: key,
            ..FrameInput::default()
        });
    }

    fn camera(&self) -> VehiclePose {
        self.run.area.player.view(&self.run.physics).camera_pose
    }

    fn view_forward(&self) -> Vector {
        Rotation::from_array(self.camera().rotation) * -Vector::Z
    }

    /// Turns the view with mouse deltas until the camera looks at the point.
    fn look_at(&mut self, point: Vector) -> bool {
        for _ in 0..400 {
            let to = (point - Vector::from_array(self.camera().position)).normalize();
            let forward = self.view_forward();
            let mut yaw = wrap_degrees(heading_degrees(to) - heading_degrees(forward));
            let pitch = (forward.y.asin() - to.y.asin()).to_degrees();
            if yaw.abs() < 0.3 && pitch.abs() < 0.3 {
                return true;
            }
            // Seat yaw stops short of 180 degrees, so turn the other way round.
            if self.run.area.player.seat().is_some() {
                let current = self.run.area.player.yaw_degrees();
                if current + yaw > 178. {
                    yaw -= 360.;
                } else if current + yaw < -178. {
                    yaw += 360.;
                }
            }
            self.frame(FrameInput {
                mouse: [yaw.clamp(-4., 4.), pitch.clamp(-4., 4.)],
                ..FrameInput::default()
            });
        }
        false
    }

    /// Walks with the movement keys and steers with the mouse.
    /// Returns false when an obstacle stops the player before the target.
    fn walk_to(&mut self, target: Vector, reach: f32) -> bool {
        let mut best = f32::INFINITY;
        let mut stalled = 0;
        for _ in 0..3000 {
            let position = Vector::from_array(self.camera().position);
            let flat = Vector::new(target.x - position.x, 0., target.z - position.z);
            if flat.length() < reach {
                self.idle(10);
                return true;
            }
            if flat.length() < best - 0.01 {
                best = flat.length();
                stalled = 0;
            } else {
                stalled += 1;
                if stalled > 60 {
                    self.idle(10);
                    return false;
                }
            }
            let error = wrap_degrees(heading_degrees(flat) - heading_degrees(self.view_forward()));
            self.frame(FrameInput {
                held: if error.abs() < 20. { keys::FORWARD } else { 0 },
                // About 200 degrees per second, a brisk human turn.
                mouse: [error.clamp(-4., 4.), 0.],
                ..FrameInput::default()
            });
        }
        panic!(
            "player did not reach {target:?}, stopped at {:?}",
            self.camera().position
        );
    }

    fn collider_centers(&self, colliders: &[u32]) -> Vec<Vector> {
        self.run
            .physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, c)| {
                c.is_enabled()
                    && colliders.contains(&((c.user_data >> 64) as u32))
                    && c.parent()
                        .is_some_and(|body| self.run.physics.rapier().bodies[body].is_enabled())
            })
            .map(|(_, c)| c.compute_aabb().center())
            .collect()
    }

    /// Flat car frame: center, hood direction, and the side direction.
    fn car_frame(&self) -> (Vector, Vector, Vector) {
        let center = Vector::from_array(self.run.physics.snapshot(0).pose.position).with_y(0.);
        let hood = self
            .reference
            .items
            .iter()
            .find(|i| i.name == "Hood")
            .unwrap()
            .id;
        let front = (Vector::from_array(self.run.area.items.get(hood).unwrap().pose.position)
            .with_y(0.)
            - center)
            .normalize();
        (center, front, Vector::Y.cross(front))
    }

    /// Walks past the car front when the point is on the other car side.
    /// The garage leaves no room behind the car.
    fn walk_around_car_to(&mut self, point: Vector, reach: f32) {
        let (center, front, side) = self.car_frame();
        let mine = (Vector::from_array(self.camera().position).with_y(0.) - center).dot(side);
        let offset = point.with_y(0.) - center;
        let theirs = offset.dot(side);
        if mine.signum() != theirs.signum() {
            for sign in [mine.signum(), theirs.signum()] {
                self.walk_to(center + front * 2.55 + side * sign * 1.15, 0.25);
            }
        }
        let along = offset.dot(front);
        if along.abs() < 2.2 {
            // Stand beside the car, clear of the body.
            self.walk_to(center + front * along + side * theirs.signum() * 1.2, 0.2);
        } else {
            self.walk_to(point, reach);
        }
    }

    /// Walks to the colliders and looks at each until the aim resolves the item.
    fn approach(
        &mut self,
        item: ItemId,
        colliders: &[u32],
        resolved: fn(&ResolvedAim) -> Option<AimTarget>,
    ) {
        let points = self.collider_centers(colliders);
        assert!(!points.is_empty(), "item {item:?} has no enabled collider");
        self.walk_around_car_to(points[0], 1.3);
        assert!(
            self.aim_at(item, colliders, resolved),
            "aim did not resolve item {item:?} from {:?}; aim {:?}",
            self.camera().position,
            self.run.aim
        );
    }

    fn aim_at(
        &mut self,
        item: ItemId,
        colliders: &[u32],
        resolved: fn(&ResolvedAim) -> Option<AimTarget>,
    ) -> bool {
        for point in self.collider_centers(colliders) {
            if !self.look_at(point) {
                continue;
            }
            self.idle(2);
            if resolved(&self.run.aim).is_some_and(|target| target.item == item) {
                return true;
            }
        }
        false
    }

    /// Holds use and drags the mouse until the mechanism opens past the angle.
    fn drag_open(&mut self, item: ItemId, mouse: [f32; 2], angle: f32) -> f32 {
        let degrees = |play: &Self| {
            let d = play
                .run
                .area
                .interaction
                .mechanisms
                .snapshot(item)
                .unwrap()
                .degrees;
            d[0].abs().max(d[1].abs())
        };
        self.frame(FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            ..FrameInput::default()
        });
        for _ in 0..80 {
            self.frame(FrameInput {
                held: keys::USE,
                mouse,
                ..FrameInput::default()
            });
            if degrees(self) > angle {
                break;
            }
        }
        self.frame(FrameInput {
            released: keys::USE,
            ..FrameInput::default()
        });
        self.idle(20);
        degrees(self)
    }

    fn bound(&self, item: ItemId, owner: fn(&ActionBinding) -> Option<ItemId>) -> Vec<u32> {
        self.reference
            .action_bindings
            .iter()
            .filter(|binding| owner(binding) == Some(item))
            .map(|binding| binding.collider.0)
            .collect()
    }

    fn car_absolute(&self) -> [f64; 3] {
        self.run
            .stream
            .origin
            .absolute(self.run.physics.snapshot(0).pose.position)
    }

    fn car_position_y(&self) -> f32 {
        self.run.physics.snapshot(0).pose.position[1]
    }

    fn car_speed(&self) -> f32 {
        Vector::from_array(self.run.physics.snapshot(0).linear_velocity).length()
    }

    fn tank(&self, item: ItemId) -> f32 {
        self.run
            .area
            .items
            .get(item)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .total()
    }

    /// Steers with held keys toward an absolute XZ point until the car is within reach.
    fn drive_to(&mut self, target: [f64; 3], reach: f64, max_speed: f32) {
        self.release_handbrake();
        for _ in 0..(90. / FIXED_DT) as u32 {
            let position = self.car_absolute();
            let delta = Vector::new(
                (target[0] - position[0]) as f32,
                0.,
                (target[2] - position[2]) as f32,
            );
            if f64::from(delta.length()) < reach {
                return;
            }
            let forward = self.car_frame().1;
            let error = wrap_degrees(heading_degrees(delta) - heading_degrees(forward));
            let steer = if error > 3. {
                keys::RIGHT
            } else if error < -3. {
                keys::LEFT
            } else {
                0
            };
            let pedal = self.pedal(max_speed);
            self.frame(FrameInput {
                held: pedal | steer,
                ..FrameInput::default()
            });
        }
        panic!(
            "car did not reach {target:?}, stopped at {:?}",
            self.car_absolute()
        );
    }

    /// Follows the straight line from start to end, steering at a point ahead on the line.
    fn follow_line(&mut self, start: [f64; 3], end: [f64; 3], reach: f64, max_speed: f32) {
        let line = Vector::new((end[0] - start[0]) as f32, 0., (end[2] - start[2]) as f32);
        let length = line.length();
        let direction = line / length;
        for _ in 0..(60. / FIXED_DT) as u32 {
            let position = self.car_absolute();
            let along = (position[0] - start[0]) as f32 * direction.x
                + (position[2] - start[2]) as f32 * direction.z;
            if distance(position, end) < reach || along > length {
                return;
            }
            let goal = (along + 4.).min(length);
            let target = [
                start[0] + f64::from(direction.x * goal),
                0.,
                start[2] + f64::from(direction.z * goal),
            ];
            self.steer_toward(target, max_speed);
        }
        panic!(
            "car did not follow the line to {end:?}, stopped at {:?}",
            self.car_absolute()
        );
    }

    /// Throttle below the speed, coast just above it, and brake well above it.
    fn pedal(&self, max_speed: f32) -> u32 {
        let speed = self.car_speed();
        if speed > max_speed + 1. {
            keys::BACK
        } else if speed > max_speed {
            0
        } else {
            keys::FORWARD
        }
    }

    fn steer_toward(&mut self, target: [f64; 3], max_speed: f32) {
        let position = self.car_absolute();
        let delta = Vector::new(
            (target[0] - position[0]) as f32,
            0.,
            (target[2] - position[2]) as f32,
        );
        let error = wrap_degrees(heading_degrees(delta) - heading_degrees(self.car_frame().1));
        let steer = if error > 1.5 {
            keys::RIGHT
        } else if error < -1.5 {
            keys::LEFT
        } else {
            0
        };
        let pedal = self.pedal(max_speed);
        self.frame(FrameInput {
            held: pedal | steer,
            ..FrameInput::default()
        });
    }

    /// Reverses straight back with held keys until the car passes the distance.
    fn reverse_out(&mut self, from: [f64; 3], direction: Vector, metres: f32, max_speed: f32) {
        self.release_handbrake();
        for _ in 0..(60. / FIXED_DT) as u32 {
            let position = self.car_absolute();
            let travelled = -((position[0] - from[0]) as f32 * direction.x
                + (position[2] - from[2]) as f32 * direction.z);
            if travelled > metres {
                self.brake();
                return;
            }
            let pedal = if self.car_speed() > max_speed {
                0
            } else {
                keys::BACK
            };
            self.hold(pedal, 1);
        }
        panic!(
            "car did not reverse {metres} m, stopped at {:?}",
            self.car_absolute()
        );
    }

    /// Brakes with the pedal against the motion until the car is slow.
    fn brake(&mut self) {
        for _ in 0..500 {
            if self.car_speed() < 0.2 {
                return;
            }
            let forward = self.car_frame().1;
            let moving = Vector::from_array(self.run.physics.snapshot(0).linear_velocity);
            self.hold(
                if moving.dot(forward) > 0. {
                    keys::BACK
                } else {
                    keys::FORWARD
                },
                1,
            );
        }
        panic!("car did not stop");
    }

    fn release_handbrake(&mut self) {
        if self.run.vehicles[0].handbrake {
            self.tap(keys::SPACE);
        }
    }

    fn park(&mut self) {
        self.brake();
        if !self.run.vehicles[0].handbrake {
            self.tap(keys::SPACE);
        }
        self.idle(50);
    }

    fn start_engine(&mut self) {
        for _ in 0..300 {
            if self.run.vehicles[0].engine_output.running {
                break;
            }
            self.hold(keys::IGNITION, 1);
        }
        self.frame(FrameInput {
            released: keys::IGNITION,
            ..FrameInput::default()
        });
        self.idle(50);
        assert!(
            self.run.vehicles[0].engine_output.running,
            "engine did not start"
        );
    }
}

#[test]
#[ignore = "requires the installed source export"]
fn p11_starting_area_loop() {
    let (reference, run) = load_garage();
    let mut play = Play {
        reference,
        run,
        claim: None,
    };
    play.idle(50);
    let mut failures: Vec<String> = Vec::new();
    let spawn = play.car_absolute();
    println!(
        "car {spawn:?}, player {:?}",
        play.run.stream.origin.absolute(play.camera().position)
    );

    // Repair: take the hood off and fit it back.
    let hood = play
        .reference
        .items
        .iter()
        .find(|i| i.name == "Hood")
        .unwrap()
        .id;
    let colliders = play.bound(hood, |b| b.removal_owner);
    play.approach(hood, &colliders, |aim| aim.grab_target);
    play.tap(keys::GRAB);
    assert!(
        !play.run.area.interaction.parts.installed(hood),
        "hood stayed on"
    );
    assert_eq!(play.run.area.interaction.held(), Some(hood));
    let slot = play
        .reference
        .part_slots
        .iter()
        .find(|slot| slot.part == Some(hood))
        .unwrap()
        .clone();
    let slot_colliders: Vec<u32> = slot.colliders.iter().map(|c| c.0).collect();
    let target = play.collider_centers(&slot_colliders);
    assert!(!target.is_empty(), "hood slot has no enabled collider");
    assert!(play.look_at(target[0]), "view did not reach the hood slot");
    play.idle(5);
    play.tap(keys::GRAB);
    assert!(
        play.run.area.interaction.parts.installed(hood),
        "hood not refitted"
    );
    assert_eq!(play.run.area.interaction.held(), None);

    // Fill: carry each petrol can to the car and pour it into the tank.
    let fuel_tank = play.reference.vehicles[0].engine.fuel_tank;
    let before = play.tank(fuel_tank);
    let opening = play
        .reference
        .openings
        .iter()
        .find(|opening| opening.tank == fuel_tank)
        .unwrap()
        .clone();
    let cans: Vec<ItemId> = play
        .reference
        .tanks
        .iter()
        .filter(|tank| tank.role == TankRole::Portable)
        .filter(|tank| tank.fluids.iter().any(|f| f.fluid_type == 0))
        .map(|tank| tank.item)
        .collect();
    for can in cans {
        let height = play.run.area.items.get(can).unwrap().pose.position[1];
        if height < play.car_position_y() - 1. {
            failures.push(format!(
                "petrol can {can:?} fell under the ground (y {height})"
            ));
            continue;
        }
        let colliders = play.bound(can, |b| b.pickup_owner);
        play.approach(can, &colliders, |aim| aim.grab_target);
        play.frame(FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..FrameInput::default()
        });
        play.frame(FrameInput {
            released: keys::GRAB,
            ..FrameInput::default()
        });
        if play.run.area.interaction.held() != Some(can) {
            failures.push(format!("petrol can {can:?} could not be picked up"));
            continue;
        }
        // Scroll the can toward the view, then raise it to chest height.
        for _ in 0..40 {
            play.frame(FrameInput {
                wheel: -0.05,
                ..FrameInput::default()
            });
        }
        let ahead = Vector::from_array(play.camera().position)
            + play.view_forward().with_y(0.).normalize() * 2.;
        assert!(play.look_at(ahead), "view did not level");
        play.idle(25);
        let colliders = play.bound(opening.cap, |b| b.use_owner);
        play.approach(opening.cap, &colliders, |aim| aim.use_target);
        if play.run.area.interaction.held() != Some(can) {
            failures.push(format!(
                "petrol can {can:?} dropped while carried; it stayed at {:?}",
                play.run.area.items.get(can).unwrap().pose.position
            ));
            continue;
        }
        if !play.run.area.interaction.opening_is_open(opening.id) {
            play.tap(keys::USE);
        }
        assert!(
            play.run.area.interaction.opening_is_open(opening.id),
            "cap closed"
        );
        let start = play.tank(can);
        for _ in 0..3000 {
            play.hold(keys::PRIMARY, 1);
            if play.tank(can) < 0.05 || play.run.area.interaction.held() != Some(can) {
                break;
            }
        }
        if play.tank(can) > start - 1. {
            failures.push(format!(
                "pouring petrol can {can:?} moved {} L",
                start - play.tank(can)
            ));
        }
        play.tap(keys::USE);
        assert!(
            !play.run.area.interaction.opening_is_open(opening.id),
            "cap open"
        );
        play.tap(keys::GRAB);
        assert_eq!(play.run.area.interaction.held(), None, "can still held");
    }
    let fuel = play.tank(fuel_tank);
    println!("fuel {before} -> {fuel} L");
    if fuel < before + 15. {
        failures.push(format!("fuel tank only went from {before} to {fuel} L"));
    }

    let drift = distance(play.car_absolute(), spawn);
    if drift > 0.1 {
        failures.push(format!(
            "parked car slid {drift} m while the player worked around it"
        ));
    }

    // Open both garage doors.
    for door in [5, 7].map(ItemId) {
        let colliders = play.bound(door, |b| b.use_owner);
        play.approach(door, &colliders, |aim| aim.use_target);
        let mut open = play.drag_open(door, [0., 4.], 85.);
        if open < 60. {
            open = play.drag_open(door, [0., -4.], 85.);
        }
        println!("garage door {door:?} open {open} degrees");
        if open < 60. {
            failures.push(format!("garage door {door:?} opened only {open} degrees"));
        }
    }

    // Start: sit in the driver seat and turn the key.
    let seat = play
        .reference
        .seats
        .iter()
        .find(|seat| seat.driver && seat.vehicle == 0)
        .unwrap()
        .clone();
    let colliders = play.bound(seat.item, |b| b.use_owner);
    let points = play.collider_centers(&colliders);
    play.walk_around_car_to(points[0], 1.3);
    let mut driver_door = None;
    if !play.aim_at(seat.item, &colliders, |aim| aim.use_target) {
        // The closed driver door covers the seat. Open it first.
        let door = play
            .run
            .aim
            .use_target
            .expect("door in front of the seat")
            .item;
        driver_door = Some(door);
        // Stand ahead of the door hinge so the door can swing, then drag it open.
        let (center, front, side) = play.car_frame();
        let offset = points[0].with_y(0.) - center;
        let standpoint =
            center + front * (offset.dot(front) + 0.9) + side * offset.dot(side).signum() * 1.1;
        play.walk_to(standpoint, 0.2);
        let handle = play.collider_centers(&play.bound(door, |b| b.use_owner));
        assert!(play.look_at(handle[0]), "view did not reach the door");
        play.idle(2);
        let open = play.drag_open(door, [4., 0.], 55.);
        println!("door {door:?} open {open} degrees");
        let seat_point = play.collider_centers(&colliders)[0];
        assert!(play.look_at(seat_point), "view did not reach the seat");
        assert!(
            play.aim_at(seat.item, &colliders, |aim| aim.use_target),
            "seat hidden after opening {door:?}; aim {:?}",
            play.run.aim
        );
    }
    play.tap(keys::USE);
    assert_eq!(play.run.area.player.seat(), Some(seat.id), "not seated");
    assert!(play.run.area.player.driving());
    if let Some(door) = driver_door {
        // Pull the door closed from the seat.
        let colliders = play.bound(door, |b| b.use_owner);
        for _ in 0..4 {
            if play
                .run
                .area
                .interaction
                .mechanisms
                .snapshot(door)
                .unwrap()
                .degrees[1]
                < 1.
            {
                break;
            }
            if play.aim_at(door, &colliders, |aim| aim.use_target) {
                play.frame(FrameInput {
                    held: keys::USE,
                    pressed: keys::USE,
                    ..FrameInput::default()
                });
                for _ in 0..160 {
                    if play
                        .run
                        .area
                        .interaction
                        .mechanisms
                        .snapshot(door)
                        .unwrap()
                        .degrees[1]
                        < 1.
                    {
                        break;
                    }
                    play.frame(FrameInput {
                        held: keys::USE,
                        mouse: [-4., 0.],
                        ..FrameInput::default()
                    });
                }
                play.frame(FrameInput {
                    released: keys::USE,
                    ..FrameInput::default()
                });
                play.idle(20);
            }
        }
        let open = play
            .run
            .area
            .interaction
            .mechanisms
            .snapshot(door)
            .unwrap()
            .degrees[1];
        println!("driver door {door:?} at {open} degrees after closing");
        if open > 5. {
            failures.push(format!(
                "driver door stayed open at {open} degrees from the seat"
            ));
        }
    }
    if play.run.vehicles[0].handbrake {
        play.tap(keys::SPACE);
    }
    play.start_engine();

    // Reverse out through the garage doors, drive past the stream edge, and come back.
    let (_, front, _) = play.car_frame();
    let ahead = |metres: f32| {
        [
            spawn[0] + f64::from(front.x * metres),
            spawn[1],
            spawn[2] + f64::from(front.z * metres),
        ]
    };
    play.reverse_out(spawn, front, 12., 3.);
    play.drive_to(ahead(-400.), 8., 15.);
    let furthest = play.car_absolute();
    println!(
        "furthest {furthest:?}, {} m from the garage",
        distance(furthest, spawn)
    );
    play.drive_to(ahead(-40.), 4., 10.);
    play.follow_line(ahead(-40.), spawn, 0.5, 2.);
    play.park();
    let back = play.car_absolute();
    println!("back {back:?}");
    assert!(distance(back, spawn) < 3., "car is not back in the garage");

    // Save, write, read, and load into a fresh runtime.
    let stamp = super::super::roadside_stamp(&play.reference);
    let saved = play
        .run
        .capture_save(
            &play.reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-24T12:00:00Z",
        )
        .unwrap();
    let loaded = SaveSnapshot::from_json(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut restored = GarageRuntime::from_save(&play.reference, &stamp, &loaded).unwrap();
    super::super::prepare_ready(&mut restored, &play.reference);
    let mut claim = None;
    for _ in 0..600 {
        if restored.advance_saved_restore(&play.reference).unwrap() {
            break;
        }
        super::super::prepare_step(&mut restored, &play.reference, &mut claim);
    }
    assert!(!restored.restore_pending(), "restore did not finish");
    let recaptured = restored
        .capture_save(
            &play.reference,
            &stamp,
            CameraSave::FirstPerson,
            &saved.saved_at,
        )
        .unwrap();
    assert_eq!(recaptured.vehicles, saved.vehicles);
    assert_eq!(recaptured.player, saved.player);
    assert_eq!(recaptured.items.len(), saved.items.len());
    for (before, after) in saved.items.iter().zip(&recaptured.items) {
        let moved = distance(
            before.pose.position.map(f64::from),
            after.pose.position.map(f64::from),
        );
        // q and -q are the same rotation.
        let turned = Rotation::from_array(before.pose.rotation)
            .angle_between(Rotation::from_array(after.pose.rotation));
        let same = crate::save::ItemSnapshot {
            pose: before.pose,
            local_pose: before.local_pose,
            ..after.clone()
        } == *before;
        if !same || moved > 0.01 || turned > 0.01 {
            failures.push(format!(
                "item {:?} changed through save and load: moved {moved:.3} m, turned {turned:.3}",
                before.instance
            ));
        }
    }
    restored.started = true;
    restored.paused = false;
    let mut play = Play {
        reference: play.reference,
        run: restored,
        claim,
    };
    assert_eq!(play.run.area.player.seat(), Some(seat.id), "seat lost");
    assert!(play.run.area.interaction.parts.installed(hood), "hood lost");
    let reloaded = play.tank(fuel_tank);
    assert!((reloaded - fuel).abs() < 1., "fuel {fuel} -> {reloaded}");

    // Continue: start if needed and reverse back out with normal input.
    if !play.run.vehicles[0].engine_output.running {
        play.start_engine();
    }
    let start = play.car_absolute();
    let (_, front, _) = play.car_frame();
    play.reverse_out(start, front, 8., 3.);
    let moved = distance(play.car_absolute(), start);
    println!("moved {moved} m after reload");
    assert!(
        failures.is_empty(),
        "acceptance failures:\n{}",
        failures.join("\n")
    );
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
