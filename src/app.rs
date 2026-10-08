use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::browser::{game_actions, game_keys, game_ready, game_update};
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    car::{ChassisVisual, WheelVisual},
    world::{ChunkVisual, WorldVisuals},
};
use crate::simulation::{DriveInput, DriveState, StepClock, can_refuel};
use crate::vehicle::{PhysicsWorld, VehiclePose, VehicleSnapshot};
use crate::world::{DEFAULT_SEED, WorldStream, nearest_station};

const START: VehiclePose = VehiclePose {
    position: [0.0, 1.1, 0.0],
    rotation: [0.0, 0.0, 0.0, 1.0],
};

#[derive(Resource)]
struct Drive {
    world: PhysicsWorld,
    stream: WorldStream,
    input: DriveInput,
    clock: StepClock,
    previous: VehicleSnapshot,
    current: VehicleSnapshot,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    rebases: u32,
    state: DriveState,
    refuel: bool,
    skip_elapsed: bool,
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.64, 0.72, 0.73)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Desert Drive".into(),
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                read_input,
                advance_physics,
                sync_world,
                interpolate_vehicle,
                update_camera,
                update_hud,
                announce_ready,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut world = PhysicsWorld::new(START);
    let mut stream = WorldStream::new(DEFAULT_SEED);
    stream.sync_at([0.0; 3], &mut world);
    let mut visuals = WorldVisuals::new(&mut meshes, &mut materials);
    visuals.sync(&stream, &mut commands, &mut meshes);
    for _ in 0..120 {
        world.step(DriveInput::default(), true);
    }
    let snapshot = world.snapshot();
    commands.insert_resource(Drive {
        world,
        stream,
        input: DriveInput::default(),
        clock: StepClock::default(),
        previous: snapshot,
        current: snapshot,
        rendered: snapshot.pose,
        camera: CameraMode::default(),
        frame_steps: 0,
        rebases: 0,
        state: DriveState::default(),
        refuel: false,
        skip_elapsed: false,
    });
    commands.insert_resource(visuals);
    presentation::car::spawn(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.91, 0.77),
            illuminance: 9_000.0,
            ..default()
        },
        Transform::from_xyz(6.0, 9.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DriveCamera,
        Camera3d::default(),
        presentation::camera::pose(CameraMode::Chase, snapshot.pose),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::Reinhard,
        DistanceFog {
            color: Color::srgb(0.64, 0.72, 0.73),
            falloff: FogFalloff::Linear {
                start: 110.0,
                end: 430.0,
            },
            ..default()
        },
    ));
}

fn reset_vehicle(drive: &mut Drive, restart: bool) {
    if restart {
        drive.state.restart();
        drive.stream.seed = DEFAULT_SEED;
        let shift = drive.stream.origin.offset.map(|offset| -offset as f32);
        drive.world.rebase(shift);
        drive.stream.origin.shift(shift);
        let pose = drive.stream.reset_pose(0.0, false, &mut drive.world);
        drive.world.reset(pose);
        drive.camera = CameraMode::default();
        drive.rebases = 0;
    } else {
        let distance = -drive.stream.origin.absolute(drive.current.pose.position)[2];
        let pose =
            drive
                .stream
                .reset_pose(distance, drive.state.fuel.liters == 0.0, &mut drive.world);
        drive.world.reset(pose);
    }
    drive.current = drive.world.snapshot();
    drive.previous = drive.current;
    drive.rendered = drive.current.pose;
    drive.clock = StepClock::default();
    drive.input = DriveInput::default();
    drive.refuel = false;
    drive.skip_elapsed = true;
}

fn read_input(mut drive: ResMut<Drive>) {
    let actions = game_actions();
    if actions & 1 != 0 {
        drive.state.started = true;
        drive.state.paused = false;
    }
    if actions & 4 != 0 {
        drive.state.paused = false;
    }
    if actions & 8 != 0 {
        reset_vehicle(&mut drive, true);
    }
    if actions & 2 != 0 {
        drive.state.paused = true;
    }
    if actions & 7 != 0 {
        drive.clock = StepClock::default();
        drive.previous = drive.current;
        drive.skip_elapsed = true;
    }
    if !drive.state.active() {
        drive.input = DriveInput::default();
        drive.refuel = false;
        return;
    }
    if actions & 16 != 0 {
        reset_vehicle(&mut drive, false);
    }
    if actions & 32 != 0 {
        drive.camera = match drive.camera {
            CameraMode::Cockpit => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Cockpit,
        };
    }
    let keys = game_keys();
    drive.input = DriveInput {
        accelerate: keys & 1 != 0,
        brake_reverse: keys & 2 != 0,
        steer: ((keys & 4 != 0) as u8 as f32) - ((keys & 8 != 0) as u8 as f32),
        handbrake: keys & 16 != 0,
    };
    drive.refuel = keys & 32 != 0;
}

fn station_distance(drive: &Drive) -> f64 {
    let absolute = drive.stream.origin.absolute(drive.current.pose.position);
    let station = nearest_station(drive.stream.seed, -absolute[2]);
    ((absolute[0] - station.absolute_point[0]).powi(2)
        + (absolute[1] - station.absolute_point[1]).powi(2)
        + (absolute[2] - station.absolute_point[2]).powi(2))
    .sqrt()
}
fn advance_physics(time: Res<Time<Real>>, mut drive: ResMut<Drive>) {
    drive.frame_steps = 0;
    if !drive.state.active() {
        return;
    }
    if drive.skip_elapsed {
        drive.skip_elapsed = false;
        return;
    }
    let steps = drive.clock.advance(time.delta_secs_f64());
    drive.frame_steps = steps as u32;
    for _ in 0..steps {
        drive.previous = drive.current;
        let previous_absolute = drive.stream.origin.absolute(drive.current.pose.position);
        let input = drive.input;
        let distance = station_distance(&drive) as f32;
        let refuel = drive.refuel;
        let Drive { world, state, .. } = &mut *drive;
        state.step(world, input, refuel, distance);
        drive.current = drive.world.snapshot();
        let absolute = drive.stream.origin.absolute(drive.current.pose.position);
        drive
            .state
            .record_travel(previous_absolute, absolute, false);
        let Drive { world, stream, .. } = &mut *drive;
        let shift = stream.rebase(world);
        stream.sync_at(absolute, world);
        if let Some(shift) = shift {
            shift_snapshot(&mut drive.previous, shift);
            shift_snapshot(&mut drive.current, shift);
            shift_pose(&mut drive.rendered, shift);
            drive.rebases += 1;
        }
    }
}

fn shift_pose(pose: &mut VehiclePose, shift: [f32; 3]) {
    for (position, shift) in pose.position.iter_mut().zip(shift) {
        *position -= shift;
    }
}

fn shift_snapshot(snapshot: &mut VehicleSnapshot, shift: [f32; 3]) {
    shift_pose(&mut snapshot.pose, shift);
    for wheel in &mut snapshot.wheels {
        shift_pose(wheel, shift);
    }
}

fn sync_world(
    drive: Res<Drive>,
    mut visuals: ResMut<WorldVisuals>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunks: Query<(&ChunkVisual, &mut Transform)>,
) {
    visuals.sync(&drive.stream, &mut commands, &mut meshes);
    for (chunk, mut transform) in &mut chunks {
        transform.translation = Vec3::from_array(drive.stream.origin.local(chunk.0));
    }
}

fn interpolate_vehicle(
    mut drive: ResMut<Drive>,
    mut chassis: Query<&mut Transform, (With<ChassisVisual>, Without<WheelVisual>)>,
    mut wheels: Query<(&WheelVisual, &mut Transform), Without<ChassisVisual>>,
) {
    let alpha = drive.clock.alpha();
    drive.rendered = presentation::interpolate(drive.previous.pose, drive.current.pose, alpha);
    for mut transform in &mut chassis {
        *transform = presentation::transform(drive.rendered);
    }
    for (wheel, mut transform) in &mut wheels {
        *transform = presentation::transform(presentation::interpolate(
            drive.previous.wheels[wheel.0],
            drive.current.wheels[wheel.0],
            alpha,
        ));
    }
}

fn update_camera(drive: Res<Drive>, mut cameras: Query<&mut Transform, With<DriveCamera>>) {
    for mut transform in &mut cameras {
        *transform = presentation::camera::pose(drive.camera, drive.rendered);
    }
}

fn update_hud(drive: Res<Drive>) {
    let position = drive.current.pose.position;
    let absolute = drive.stream.origin.absolute(position);
    let origin = drive.stream.origin.offset;
    let contacts = drive
        .current
        .wheels_contact
        .iter()
        .enumerate()
        .fold(0, |bits, (i, contact)| bits | ((*contact as u32) << i));
    let station = nearest_station(drive.stream.seed, -absolute[2]);
    let distance = station_distance(&drive);
    game_update(
        drive.current.speed_mps * 3.6,
        (drive.camera == CameraMode::Cockpit) as u32,
        position[0],
        position[1],
        position[2],
        contacts,
        drive.frame_steps,
        drive.stream.chunks().count() as u32,
        drive.world.collider_count() as u32,
        absolute[0],
        absolute[1],
        absolute[2],
        origin[0],
        origin[1],
        origin[2],
        drive.rebases,
        drive.state.trip_m,
        drive.stream.lateral_band() as f64,
        drive.state.fuel.liters,
        drive.state.paused,
        drive.state.started,
        station.absolute_point[0],
        station.absolute_point[1],
        station.absolute_point[2],
        distance,
        can_refuel(drive.current.absolute_speed_kmh(), distance as f32),
    );
}

fn announce_ready(mut frames: Local<u8>) {
    if *frames < 3 {
        *frames += 1;
        if *frames == 3 {
            game_ready();
        }
    }
}
