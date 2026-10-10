use crate::browser::*;
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    reference::{BodyVisual, CapVisual, ItemVisual, ReferenceVisuals, ValveVisibility},
    world::{ChunkVisual, WorldVisuals},
};
use crate::reference::{ItemId, OpeningId, ReferenceManifest};
use crate::simulation::{FrameInput, StepClock, keys};
use crate::starting_area::GarageRuntime;
use crate::vehicle::VehiclePose;
use bevy::asset::{AssetMetaCheck, LoadState, RecursiveDependencyLoadState};
use bevy::camera::visibility::RenderLayers;
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[derive(Resource)]
struct RuntimeReference(Arc<ReferenceManifest>);
#[derive(Resource)]
struct Drive {
    runtime: GarageRuntime,
    input: FrameInput,
    clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    skip_elapsed: bool,
    ready: bool,
    last_feedback: Option<FeedbackStamp>,
}
#[derive(Clone, Copy, PartialEq)]
struct FeedbackStamp {
    actions: u32,
    context: u32,
    item: Option<ItemId>,
    opening: Option<OpeningId>,
    carrying: bool,
    notice: &'static str,
    amount: u32,
    empty: bool,
    capacity: f32,
    percents: [u32; 6],
}

#[wasm_bindgen]
pub fn run(reference_json: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let reference = Arc::new(
        ReferenceManifest::from_json(reference_json.as_bytes())
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    let runtime = GarageRuntime::new(&reference).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let current = runtime.area.player.view_pose(&runtime.physics);
    App::new()
        .insert_resource(RuntimeReference(reference))
        .insert_resource(Drive {
            runtime,
            input: FrameInput::default(),
            clock: StepClock::default(),
            previous: current,
            current,
            rendered: current,
            camera: CameraMode::default(),
            frame_steps: 0,
            skip_elapsed: false,
            ready: false,
            last_feedback: None,
        })
        .insert_resource(ClearColor(Color::srgb(0.64, 0.72, 0.73)))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "assets".into(),
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "My Summer Slop".into(),
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                check_ready,
                read_input,
                advance_physics,
                sync_world,
                sync_imported,
                update_camera,
                update_hud,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
) {
    let mut visuals = WorldVisuals::new(&mut meshes, &mut materials);
    visuals.sync(&drive.runtime.stream, &mut commands, &mut meshes);
    commands.insert_resource(visuals);
    presentation::reference::spawn(&mut commands, &server, &reference.0);
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.91, 0.77),
            illuminance: 9000.0,
            ..default()
        },
        Transform::from_xyz(6.0, 9.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        (0..32).collect::<RenderLayers>(),
    ));
    commands.spawn((
        DriveCamera,
        Camera3d::default(),
        (0..32)
            .filter(|layer| reference.0.player.camera_culling_mask & (1u32 << layer) != 0)
            .collect::<RenderLayers>(),
        Projection::Perspective(PerspectiveProjection {
            fov: reference.0.player.field_of_view_degrees.to_radians(),
            ..default()
        }),
        presentation::transform(drive.current),
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

fn check_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut visuals: ResMut<ReferenceVisuals>,
    extras: Query<(Entity, &GltfExtras)>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    mut frames: Local<u8>,
) {
    if drive.ready || visuals.failed {
        return;
    }
    for (path, handle) in &visuals.handles {
        let Some((load, _, recursive)) = server.get_load_states(handle.id()) else {
            return;
        };
        if let LoadState::Failed(error) = load {
            game_failure(&format!("Cannot load {path}: {error}"));
            visuals.failed = true;
            return;
        }
        if let RecursiveDependencyLoadState::Failed(error) = recursive {
            game_failure(&format!("Cannot load dependencies for {path}: {error}"));
            visuals.failed = true;
            return;
        }
        if !server.is_loaded_with_dependencies(handle.id()) {
            return;
        }
    }
    match presentation::reference::bind(
        &mut commands,
        &reference.0,
        &mut visuals,
        &extras,
        &primitives,
    ) {
        Ok(true) => {
            *frames += 1;
            if *frames >= 2 {
                drive.ready = true;
                game_ready();
            }
        }
        Ok(false) => {}
        Err(error) => {
            game_failure(&error);
            visuals.failed = true;
        }
    }
}

fn clear_input(drive: &mut Drive) {
    drive.input.clear();
    drive.clock = StepClock::default();
    drive.previous = drive.current;
    drive.rendered = drive.current;
    drive.runtime.area.begin_step();
    drive.skip_elapsed = true;
}
fn read_input(reference: Res<RuntimeReference>, mut drive: ResMut<Drive>) {
    let actions = game_actions();
    let frame = FrameInput {
        held: game_keys(),
        pressed: game_pressed(),
        released: game_released(),
        mouse: [
            game_mouse_x() * reference.0.player.look_sensitivity[0],
            game_mouse_y() * reference.0.player.look_sensitivity[1],
        ],
        wheel: game_wheel(),
    };
    if actions & 2 != 0 {
        drive.runtime.paused = true;
        clear_input(&mut drive);
    }
    if actions & 5 != 0 && drive.ready {
        drive.runtime.started = true;
        drive.runtime.paused = false;
        clear_input(&mut drive);
    }
    if actions & 8 != 0 {
        if let Err(error) = drive.runtime.restart(&reference.0) {
            game_failure(&error.to_string());
            return;
        }
        drive.camera = CameraMode::default();
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
        clear_input(&mut drive);
    }
    if actions & 16 != 0 {
        drive.runtime.recover(&reference.0);
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
        clear_input(&mut drive);
    }
    if !drive.runtime.started || drive.runtime.paused {
        drive.input.clear();
        return;
    }
    if frame.pressed & keys::CAMERA != 0 && drive.runtime.area.player.seat().is_some() {
        drive.camera = match drive.camera {
            CameraMode::Cockpit => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Cockpit,
        };
    }
    drive.input.merge(frame);
}
fn advance_physics(
    time: Res<Time<Real>>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
) {
    drive.frame_steps = 0;
    if !drive.runtime.started || drive.runtime.paused {
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
        let old_origin = drive.runtime.stream.origin.offset;
        let input = drive.input.take_step();
        drive.runtime.step(&reference.0, input);
        let new_origin = drive.runtime.stream.origin.offset;
        for i in 0..3 {
            drive.previous.position[i] -= (new_origin[i] - old_origin[i]) as f32;
        }
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
    }
    drive.rendered = presentation::interpolate(drive.previous, drive.current, drive.clock.alpha());
}
fn sync_world(
    drive: Res<Drive>,
    mut visuals: ResMut<WorldVisuals>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunks: Query<(&ChunkVisual, &mut Transform)>,
) {
    visuals.sync(&drive.runtime.stream, &mut commands, &mut meshes);
    for (chunk, mut transform) in &mut chunks {
        let translation = Vec3::from_array(drive.runtime.stream.origin.local(chunk.0));
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}
fn sync_imported(
    drive: Res<Drive>,
    mut query: Query<(&BodyVisual, &mut Transform)>,
    mut items: Query<(&ItemVisual, &mut Transform), Without<BodyVisual>>,
    mut caps: Query<(&CapVisual, &mut Transform), (Without<BodyVisual>, Without<ItemVisual>)>,
    mut visibility: Query<(&ValveVisibility, &mut Visibility)>,
) {
    presentation::reference::sync(
        &drive.runtime.area,
        &drive.runtime.physics,
        drive.clock.alpha(),
        &mut query,
    );
    presentation::reference::sync_items(&drive.runtime.area, drive.clock.alpha(), &mut items);
    presentation::reference::sync_caps(&drive.runtime.area, &mut caps, &mut visibility);
}
fn update_camera(drive: Res<Drive>, mut query: Query<&mut Transform, With<DriveCamera>>) {
    for mut transform in &mut query {
        *transform =
            if drive.camera == CameraMode::Chase && drive.runtime.area.player.seat().is_some() {
                presentation::camera::pose(CameraMode::Chase, drive.runtime.physics.snapshot().pose)
            } else {
                presentation::transform(drive.rendered)
            };
    }
}
fn update_hud(reference: Res<RuntimeReference>, mut drive: ResMut<Drive>) {
    let runtime = &drive.runtime;
    let snapshot = runtime.physics.snapshot();
    let position = runtime.area.player.focus_position(&runtime.physics);
    let absolute = runtime.stream.origin.absolute(position);
    let origin = runtime.stream.origin.offset;
    let contacts = snapshot
        .wheels_contact
        .iter()
        .enumerate()
        .fold(0, |bits, (i, contact)| bits | ((*contact as u32) << i));
    let fuel = runtime
        .area
        .items
        .get(reference.0.engine.fuel_tank)
        .and_then(|item| item.fluid.as_ref())
        .map_or(0.0, |fluid| fluid.mixture.total());
    game_update(
        snapshot.speed_mps * 3.6,
        (drive.camera == CameraMode::Cockpit || runtime.area.player.seat().is_none()) as u32,
        position[0],
        position[1],
        position[2],
        contacts,
        drive.frame_steps,
        runtime.stream.chunks().count() as u32,
        runtime.physics.collider_count() as u32,
        absolute[0],
        absolute[1],
        absolute[2],
        origin[0],
        origin[1],
        origin[2],
        runtime.rebases,
        runtime.trip_m,
        runtime.stream.lateral_band() as f64,
        fuel,
        runtime.paused,
        runtime.started,
    );
    let available = runtime.area.interaction.available_actions(
        runtime.target,
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
    );
    let info = runtime
        .area
        .interaction
        .feedback(&available, &runtime.area.items, &reference.0);
    let (amount, capacity, quantities, name, has_name, no_text) =
        info.fluid
            .as_ref()
            .map_or((0., 0., [0.; 6], info.name, false, true), |f| {
                (
                    f.amount,
                    f.capacity,
                    f.quantities,
                    f.presentation.name.as_str(),
                    f.presentation.has_name,
                    f.presentation.no_text,
                )
            });
    let stamp = FeedbackStamp {
        actions: info.actions.0,
        context: info.context as u32,
        item: available.item,
        opening: available.opening,
        carrying: info.carrying,
        notice: runtime.notice,
        amount: (amount * 10.).round() as u32,
        empty: amount <= 0.05,
        capacity,
        percents: std::array::from_fn(|i| {
            let p = if amount > 0. {
                quantities[i] / amount * 100.
            } else {
                0.
            };
            if p >= 1. { p.round() as u32 } else { u32::MAX }
        }),
    };
    if drive.last_feedback != Some(stamp) {
        game_feedback(
            stamp.actions,
            stamp.context,
            name,
            stamp.notice,
            stamp.carrying,
            amount,
            capacity,
            has_name,
            no_text,
            quantities[0],
            quantities[1],
            quantities[2],
            quantities[3],
            quantities[4],
            quantities[5],
        );
        drive.last_feedback = Some(stamp);
    }
}
