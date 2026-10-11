use crate::browser::*;
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    reference::{BodyVisual, CapVisual, ItemVisual, ReferenceVisuals, ValveVisibility},
    world::{ChunkVisual, WorldVisuals},
};
use crate::reference::{ItemId, ItemKind, ReferenceManifest};
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
pub(crate) struct RuntimeReference(pub Arc<ReferenceManifest>);
#[derive(Resource)]
pub(crate) struct Drive {
    pub runtime: GarageRuntime,
    input: FrameInput,
    clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    skip_elapsed: bool,
    ready: bool,
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
        })
        .insert_resource(ClearColor(Color::BLACK))
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
        .add_plugins(presentation::lighting::SourceLightingPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                check_ready,
                presentation::lighting::bind_lighting,
                presentation::lighting::lighting_readiness,
                read_input,
                advance_physics,
                sync_world,
                sync_imported,
                presentation::lighting::sync_control_visuals,
                update_camera,
                update_hud,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            presentation::lighting::sync_lighting
                .after(bevy::transform::TransformSystems::Propagate)
                .before(bevy::light::SimulationLightSystems::AssignLightsToClusters)
                .before(bevy::light::SimulationLightSystems::UpdateDirectionalLightCascades)
                .before(bevy::light::SimulationLightSystems::UpdateLightFrusta),
        )
        .run();
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    gltf_extensions: Res<bevy::gltf::extensions::GltfExtensionHandlers>,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
) {
    let mut visuals = WorldVisuals::new(&mut meshes, &mut materials);
    visuals.sync(&drive.runtime.stream, &mut commands, &mut meshes);
    commands.insert_resource(visuals);
    presentation::lighting::load_lighting(&mut commands, &server, &reference.0.lighting);
    presentation::lighting::load_lamp_materials(&mut commands, &gltf_extensions, &reference.0);
    presentation::reference::load(&mut commands, &server, &reference.0);
    commands.spawn((
        DriveCamera,
        Camera3d::default(),
        bevy::camera::Hdr,
        (0..32)
            .filter(|layer| reference.0.player.camera_culling_mask & (1u32 << layer) != 0)
            .collect::<RenderLayers>(),
        Projection::Perspective(PerspectiveProjection {
            fov: reference.0.player.field_of_view_degrees.to_radians(),
            near: reference
                .0
                .lighting
                .cameras
                .iter()
                .map(|c| c.startup_near_plane)
                .fold(f32::INFINITY, f32::min),
            far: reference
                .0
                .lighting
                .cameras
                .iter()
                .map(|c| c.startup_far_plane)
                .fold(0., f32::max),
            ..default()
        }),
        presentation::transform(drive.current),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
    ));
}

fn check_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut visuals: ResMut<ReferenceVisuals>,
    extras: Query<(Entity, &GltfExtras)>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    mut frames: Local<u8>,
    lighting: Res<presentation::lighting::SourceLighting>,
) {
    if visuals.failed {
        return;
    }
    if let presentation::lighting::RenderingState::Failed(error) = &lighting.state {
        game_failure(error);
        visuals.failed = true;
        return;
    }
    if drive.ready {
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
        if !server.is_loaded_with_dependencies(handle.id()) || !gltfs.contains(handle.id()) {
            return;
        }
        if gltfs.get(handle).unwrap().scenes.is_empty() {
            game_failure(&format!("Required source scene 0 is missing from {path}"));
            visuals.failed = true;
            return;
        }
    }
    if !lighting.materials_ready {
        return;
    }
    if !visuals.spawned {
        presentation::reference::spawn_scenes(&mut commands, &reference.0, &mut visuals, &gltfs);
        return;
    }
    match presentation::reference::bind(
        &mut commands,
        &reference.0,
        &mut visuals,
        &extras,
        &primitives,
    ) {
        Ok(true) => {
            match &lighting.state {
                presentation::lighting::RenderingState::Loading => return,
                presentation::lighting::RenderingState::Failed(error) => {
                    game_failure(error);
                    visuals.failed = true;
                    return;
                }
                presentation::lighting::RenderingState::Ready => {}
            }
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
    if !drive.runtime.started || drive.runtime.paused {
        drive.input.clear();
        return;
    }
    if actions & 16 != 0 {
        drive.runtime.recover(&reference.0);
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
        clear_input(&mut drive);
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
    drive
        .runtime
        .advance_lighting(&reference.0, time.delta_secs_f64());
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
    drive
        .runtime
        .advance_light_fades(&reference.0, time.delta_secs_f64());
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
fn update_hud(reference: Res<RuntimeReference>, drive: Res<Drive>) {
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
    let target = runtime.target;
    let availability = runtime.area.interaction.available_actions(
        target,
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
    );
    game_feedback(
        availability.context as u32,
        availability.actions.0,
        availability.light_control.map_or(0, |id| id.0),
        availability.lmb_use,
    );
    let record = availability
        .item
        .and_then(|id| reference.0.items.iter().find(|item| item.id == id));
    let held = runtime
        .area
        .interaction
        .held()
        .map(|id| display_name(&reference.0, &runtime.area.items, id))
        .unwrap_or("");
    let state = availability.item.and_then(|id| {
        let id = reference
            .0
            .openings
            .iter()
            .find(|opening| opening.cap == id)
            .map_or(id, |opening| opening.tank);
        runtime.area.items.get(id)
    });
    let amount = state
        .and_then(|item| item.fluid.as_ref())
        .map_or(-1.0, |fluid| fluid.mixture.total());
    let volumes = state
        .and_then(|item| item.fluid.as_ref())
        .map_or([-1.0; 6], |fluid| fluid.mixture.quantities_liters);
    use crate::interaction::feedback::ActionMask;
    let usable_target =
        availability.actions.0 & (ActionMask::USE | ActionMask::DOOR | ActionMask::TUNE) != 0;
    let action = if availability.light_control.is_some() {
        if availability.lmb_use {
            "E / LMB: switch lights"
        } else {
            "E: switch lights"
        }
    } else if !runtime.notice.is_empty() {
        runtime.notice
    } else if runtime.area.player.seat().is_some() && !usable_target {
        if runtime.area.player.driving() {
            "E: exit · I: starter · Space: handbrake · C: camera"
        } else {
            "E: exit · C: camera"
        }
    } else {
        record.map_or(
            if target.is_some() && state.and_then(|item| item.fluid.as_ref()).is_some() {
                "Open the container before pickup. Hold LMB: fill."
            } else {
                if held.is_empty() {
                    ""
                } else {
                    "F: drop · Mouse wheel: distance · RMB + mouse: rotate"
                }
            },
            |item| match item.kind {
                ItemKind::Door | ItemKind::Hood | ItemKind::Trunk => "Hold E + mouse: move",
                ItemKind::Cap => {
                    if reference
                        .0
                        .openings
                        .iter()
                        .any(|opening| opening.cap == item.id && opening.can_tune)
                    {
                        "Hold E + mouse / Scroll: tune valve · Hold LMB: fill"
                    } else {
                        "E: cap · Hold LMB: fill"
                    }
                }
                ItemKind::Seat => {
                    if reference.0.seats.iter().any(|seat| seat.item == item.id) {
                        "E: enter"
                    } else {
                        ""
                    }
                }
                ItemKind::Wheel | ItemKind::Rim | ItemKind::Tire => {
                    let mounted = state.is_some_and(|state| match state.location {
                        crate::interaction::items::ItemLocation::WheelSlot(_) => true,
                        crate::interaction::items::ItemLocation::Attached(parent) => {
                            runtime.area.items.get(parent).is_some_and(|parent| {
                                matches!(
                                    parent.location,
                                    crate::interaction::items::ItemLocation::WheelSlot(_)
                                )
                            })
                        }
                        _ => false,
                    });
                    if mounted {
                        "F: remove wheel"
                    } else if target.is_some_and(|target| target.slot.is_some()) {
                        if held.is_empty() {
                            "Carry a wheel. Aim here and press F."
                        } else {
                            "F: install wheel"
                        }
                    } else if held.is_empty() {
                        "F: carry · X: split wheel"
                    } else {
                        "F: drop · X: split or assemble"
                    }
                }
                ItemKind::Container => {
                    if item.body == reference.0.vehicle.body {
                        "Open the fuel cap to fill."
                    } else {
                        "F: carry"
                    }
                }
                _ => "",
            },
        )
    };
    game_detail(
        availability
            .item
            .map(|id| display_name(&reference.0, &runtime.area.items, id))
            .unwrap_or(""),
        action,
        amount,
        state.map_or(-1.0, |item| item.condition),
        held,
        runtime.engine_output.rpm,
        runtime.engine_output.ignition,
        runtime.engine_output.running,
        runtime.engine_output.starter,
        runtime.area.player.seat().is_some(),
        runtime.handbrake,
        volumes[0],
        volumes[1],
        volumes[2],
        volumes[3],
        volumes[4],
        volumes[5],
    );
}

fn display_name<'a>(
    reference: &'a ReferenceManifest,
    items: &'a crate::interaction::items::ItemStore,
    id: ItemId,
) -> &'a str {
    let Some(item) = reference.items.iter().find(|item| item.id == id) else {
        return items.name(id).unwrap_or("");
    };
    match item.kind {
        ItemKind::Decoration => "",
        ItemKind::Door => {
            if item.body == reference.vehicle.body
                || crate::starting_area::belongs_to_vehicle(reference, item.body)
            {
                "Car door"
            } else {
                item.name.as_str()
            }
        }
        ItemKind::Hood => "Hood",
        ItemKind::Trunk => "Trunk",
        ItemKind::Engine => "Engine",
        ItemKind::Radiator => "Radiator",
        ItemKind::Tire => "Tire",
        ItemKind::Rim => "Rim",
        ItemKind::Wheel => "Wheel",
        ItemKind::Seat => {
            if reference
                .seats
                .iter()
                .any(|seat| seat.item == id && seat.driver)
            {
                "Driver seat"
            } else {
                "Seat"
            }
        }
        ItemKind::Cap => {
            if reference
                .openings
                .iter()
                .any(|opening| opening.cap == id && opening.can_tune)
            {
                return "Water tap";
            }
            let tank = reference
                .openings
                .iter()
                .find(|opening| opening.cap == id)
                .map(|opening| opening.tank);
            if tank == Some(reference.engine.fuel_tank) {
                "Fuel cap"
            } else if tank == Some(reference.engine.oil_tank) {
                "Oil cap"
            } else if tank == Some(reference.engine.coolant_tank) {
                "Coolant cap"
            } else {
                "Container cap"
            }
        }
        ItemKind::Container => {
            if item.body == reference.vehicle.body {
                return "Fuel tank";
            }
            if let Some(tank) = reference.tanks.iter().find(|tank| tank.item == id) {
                if tank.quantities_liters[0] > 0.0 {
                    "Fuel can"
                } else if tank.quantities_liters[1] > 0.0 {
                    "Oil can"
                } else if tank.quantities_liters[3] > 0.0 {
                    "Water container"
                } else {
                    "Container"
                }
            } else {
                "Container"
            }
        }
    }
}
