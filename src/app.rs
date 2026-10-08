use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    car::{ChassisVisual, WheelVisual},
};
use crate::simulation::{DriveInput, StepClock};
use crate::vehicle::{PhysicsWorld, VehiclePose, VehicleSnapshot};

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
    fn game_update(speed: f32, camera: u32, x: f32, y: f32, z: f32, contacts: u32, steps: u32);
}

#[derive(Resource)]
struct Drive {
    world: PhysicsWorld,
    input: DriveInput,
    clock: StepClock,
    previous: VehicleSnapshot,
    current: VehicleSnapshot,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
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
    world.insert_mesh(
        &[
            [-300.0, 0.0, -300.0],
            [300.0, 0.0, -300.0],
            [300.0, 0.0, 300.0],
            [-300.0, 0.0, 300.0],
        ],
        &[[0, 2, 1], [0, 3, 2]],
    );
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(600.0, 600.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.63, 0.47, 0.29))),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 600.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.24, 0.23, 0.21))),
        Transform::from_xyz(0.0, 0.01, 0.0),
    ));
    let stripe_mesh = meshes.add(Cuboid::new(0.12, 0.015, 3.0));
    let stripe_material = materials.add(Color::srgb(0.86, 0.75, 0.48));
    for i in -35..35 {
        commands.spawn((
            Mesh3d(stripe_mesh.clone()),
            MeshMaterial3d(stripe_material.clone()),
            Transform::from_xyz(0.0, 0.025, i as f32 * 8.0),
        ));
    }
    let obstacle_mesh = meshes.add(Cuboid::new(2.0, 1.2, 2.0));
    let obstacle_material = materials.add(Color::srgb(0.44, 0.32, 0.22));
    for position in [[5.5, 0.6, -25.0], [-5.5, 0.6, -45.0]] {
        commands.spawn((
            Mesh3d(obstacle_mesh.clone()),
            MeshMaterial3d(obstacle_material.clone()),
            Transform::from_translation(Vec3::from_array(position)),
        ));
        insert_box(&mut world, position);
    }
    for _ in 0..120 {
        world.step(DriveInput::default(), true);
    }
    let snapshot = world.snapshot();
    commands.insert_resource(Drive {
        world,
        input: DriveInput::default(),
        clock: StepClock::default(),
        previous: snapshot,
        current: snapshot,
        rendered: snapshot.pose,
        camera: CameraMode::default(),
        frame_steps: 0,
    });
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

fn insert_box(world: &mut PhysicsWorld, center: [f32; 3]) {
    let vertices = [
        [-1.0, -0.6, -1.0],
        [1.0, -0.6, -1.0],
        [1.0, -0.6, 1.0],
        [-1.0, -0.6, 1.0],
        [-1.0, 0.6, -1.0],
        [1.0, 0.6, -1.0],
        [1.0, 0.6, 1.0],
        [-1.0, 0.6, 1.0],
    ]
    .map(|v| [v[0] + center[0], v[1] + center[1], v[2] + center[2]]);
    world.insert_mesh(
        &vertices,
        &[
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    );
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
        drive.world.reset(START);
        drive.current = drive.world.snapshot();
        drive.previous = drive.current;
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
        let input = drive.input;
        drive.world.step(input, true);
        drive.current = drive.world.snapshot();
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
