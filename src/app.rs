use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    car::{ChassisVisual, WheelVisual},
    world::{ChunkVisual, WorldVisuals},
};
use crate::simulation::{DriveInput, StepClock};
use crate::vehicle::{PhysicsWorld, VehiclePose, VehicleSnapshot};
use crate::world::{DEFAULT_SEED, WorldStream};

const START: VehiclePose = VehiclePose {
    position: [0.0, 1.1, 0.0],
    rotation: [0.0, 0.0, 0.0, 1.0],
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameReady)]
    fn game_ready();
    #[wasm_bindgen(js_namespace = window, js_name = gameStarted)]
    fn game_started() -> bool;
    #[wasm_bindgen(js_namespace = window, js_name = gameUpdate)]
    fn game_update(
        speed: f32,
        camera: u32,
        x: f32,
        y: f32,
        z: f32,
        contacts: u32,
        steps: u32,
        chunks: u32,
        colliders: u32,
        absolute_x: f64,
        absolute_y: f64,
        absolute_z: f64,
        origin_x: f64,
        origin_y: f64,
        origin_z: f64,
        rebases: u32,
        travel: f64,
        lateral_band: f64,
    );
}

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
    travel_m: f64,
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.82)))
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
        travel_m: 0.0,
    });
    commands.insert_resource(visuals);
    presentation::car::spawn(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            ..default()
        },
        Transform::from_xyz(6.0, 9.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DriveCamera,
        Camera3d::default(),
        presentation::camera::pose(CameraMode::Chase, snapshot.pose),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
    ));
}

fn read_input(keys: Res<ButtonInput<KeyCode>>, windows: Query<&Window>, mut drive: ResMut<Drive>) {
    if !game_started() || !windows.iter().any(|window| window.focused) {
        drive.input = DriveInput::default();
        return;
    }
    let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    drive.input = DriveInput {
        accelerate: keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        brake_reverse: keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        steer: left as u8 as f32 - right as u8 as f32,
        handbrake: keys.pressed(KeyCode::Space),
    };
    if keys.just_pressed(KeyCode::KeyC) {
        drive.camera = match drive.camera {
            CameraMode::Cockpit => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Cockpit,
        };
    }
    if keys.just_pressed(KeyCode::KeyR) {
        let distance = -drive.stream.origin.absolute(drive.current.pose.position)[2];
        let Drive { world, stream, .. } = &mut *drive;
        let pose = stream.reset_pose(distance, false, world);
        world.reset(pose);
        drive.current = drive.world.snapshot();
        drive.previous = drive.current;
        drive.rendered = drive.current.pose;
        drive.clock = StepClock::default();
    }
}

fn advance_physics(time: Res<Time<Real>>, mut drive: ResMut<Drive>) {
    drive.frame_steps = 0;
    if !game_started() {
        return;
    }
    let steps = drive.clock.advance(time.delta_secs_f64());
    drive.frame_steps = steps as u32;
    for _ in 0..steps {
        drive.previous = drive.current;
        let previous_absolute = drive.stream.origin.absolute(drive.current.pose.position);
        let input = drive.input;
        drive.world.step(input, true);
        drive.current = drive.world.snapshot();
        let absolute = drive.stream.origin.absolute(drive.current.pose.position);
        drive.travel_m += ((absolute[0] - previous_absolute[0]).powi(2)
            + (absolute[2] - previous_absolute[2]).powi(2))
        .sqrt();
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
        drive.travel_m,
        drive.stream.lateral_band() as f64,
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
