use crate::browser::*;
use crate::interaction::labels::display_name;
use crate::presentation::instruments::InstrumentVisuals;
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    reference::{BodyVisual, CapVisual, ItemVisual, ReferenceVisuals, ValveVisibility},
};
use crate::reference::{ItemId, OpeningId, ReferenceManifest};
use crate::simulation::instruments::{InstrumentSample, InstrumentState};
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
    instruments: InstrumentState,
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
    light_control: u32,
    lmb_use: bool,
    item: Option<ItemId>,
    opening: Option<OpeningId>,
    carrying: bool,
    notice: &'static str,
    amount: u32,
    empty: bool,
    capacity: f32,
    percents: [u32; 6],
    condition: u32,
    held: Option<ItemId>,
    throw_charge: u32,
    throw_active: bool,
    rpm: u32,
    engine_flags: [bool; 5],
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
        .insert_resource(RuntimeReference(reference.clone()))
        .insert_resource(Drive {
            instruments: InstrumentState::new(&reference.instruments),
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
        .add_plugins((
            presentation::lighting::SourceLightingPlugin,
            presentation::materials::SourceMaterialPlugin,
            presentation::world::prototypes::SourcePrototypePlugin,
            presentation::world::pipelines::WorldPipelinePlugin,
        ))
        .init_resource::<presentation::world::SourceWorldVisuals>()
        .init_resource::<presentation::world::instances::SourceInstances>()
        .init_resource::<presentation::world::grass_jobs::GrassJobs>()
        .init_resource::<presentation::world::runtime::SourcePresentationAllocation>()
        .init_resource::<crate::world::metrics::browser::MeshMeasurements>()
        .add_systems(
            Last,
            (
                crate::world::metrics::browser::collect_mesh_bytes,
                crate::world::metrics::browser::publish,
            )
                .chain(),
        )
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
                sync_instruments,
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
    server: Res<AssetServer>,
    gltf_extensions: Res<bevy::gltf::extensions::GltfExtensionHandlers>,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
) {
    commands.insert_resource(InstrumentVisuals::default());
    presentation::materials::load_materials(&mut commands, &server, &reference.0);
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
    mut instruments: ResMut<InstrumentVisuals>,
    gltf_materials: Res<Assets<bevy::gltf::GltfMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    extras: Query<(Entity, &GltfExtras)>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    mut frames: Local<u8>,
    lighting: Res<presentation::lighting::SourceLighting>,
    mut materials: ResMut<presentation::materials::SourceMaterials>,
) {
    if visuals.failed {
        return;
    }
    if let presentation::lighting::RenderingState::Failed(error) = &lighting.state {
        game_failure(error);
        visuals.failed = true;
        return;
    }
    if let presentation::materials::RenderingState::Failed(error) = &materials.state {
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
            match instruments.bind(
                &reference.0,
                &visuals.nodes,
                &primitives,
                &visuals.gltfs,
                &gltfs,
                &gltf_materials,
                &mut standard_materials,
                &mut materials,
            ) {
                Ok(true) => {}
                Ok(false) => return,
                Err(e) => {
                    game_failure(&e.to_string());
                    visuals.failed = true;
                    return;
                }
            }
            match &lighting.state {
                presentation::lighting::RenderingState::Loading => return,
                presentation::lighting::RenderingState::Failed(error) => {
                    game_failure(error);
                    visuals.failed = true;
                    return;
                }
                presentation::lighting::RenderingState::Ready => {}
            }
            if materials.state != presentation::materials::RenderingState::Ready {
                return;
            }
            if !drive.runtime.world_supported() || !drive.runtime.stream.window_ready() {
                return;
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
    drive.runtime.clear_input();
    drive.clock = StepClock::default();
    drive.previous = drive.current;
    drive.rendered = drive.current;
    drive.runtime.area.begin_step();
    drive.skip_elapsed = true;
}
fn read_input(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut source: ResMut<presentation::lighting::SourceLighting>,
) {
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
        drive.ready = false;
        drive.runtime.started = false;
        source.reset_instances(&mut commands);
        drive.instruments.reset(&reference.0.instruments);
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
    if actions & 31 != 0 {
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
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut presentation: presentation::world::runtime::WorldPresentation,
    camera: Query<(&Projection, &GlobalTransform), With<DriveCamera>>,
    windows: Query<&Window>,
) {
    let Ok((projection, camera_transform)) = camera.single() else {
        return;
    };
    let origin = drive.runtime.stream.origin;
    let position = Vec3::from_array(
        origin
            .absolute(camera_transform.translation().to_array())
            .map(|v| v as f32),
    );
    let source_from_runtime = Mat4::from_scale(Vec3::new(1., 1., -1.));
    let mut camera_world = camera_transform.to_matrix();
    camera_world.w_axis = position.extend(1.);
    let clip = projection.get_clip_from_view() * camera_world.inverse() * source_from_runtime;
    let rows = clip.transpose();
    let planes = [
        rows.w_axis + rows.x_axis,
        rows.w_axis - rows.x_axis,
        rows.w_axis + rows.y_axis,
        rows.w_axis - rows.y_axis,
    ]
    .map(|p| p / p.truncate().length());
    let view = presentation::materials::grass::GrassView {
        position: position * Vec3::new(1., 1., -1.),
        screen_height: windows.single().map_or(1., |w| w.physical_height() as f32),
        clip_planes: planes,
    };
    let lod_view = presentation::world::lod::LodView {
        position,
        fov_degrees: reference.0.player.field_of_view_degrees,
        ortho_size: match projection {
            Projection::Orthographic(p) => Some(p.area.height() * 0.5),
            _ => None,
        },
    };
    let inverse_clip = (projection.get_clip_from_view() * camera_world.inverse()).inverse();
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [0.00001, 1.] {
                let point = inverse_clip.project_point3(Vec3::new(x, y, z));
                min = min.min(point);
                max = max.max(point);
            }
        }
    }
    let mut receivers = [crate::world::WorldBounds {
        min: min.to_array().map(|v| v as f64),
        max: max.to_array().map(|v| v as f64),
    }; 2];
    let receiver_count = if let Some(bounds) = drive.runtime.stream.recovery_window() {
        receivers[1] = bounds;
        2
    } else {
        1
    };
    let runtime = &mut drive.runtime;
    let enabled = runtime
        .lighting
        .road_lamp_output(&reference.0.lighting)
        .enabled;
    let result = presentation
        .prepare(
            &mut runtime.stream,
            &reference.0,
            &mut runtime.physics,
            &mut runtime.area,
            &mut runtime.wheels,
            view,
            runtime.lighting.cycle.output().shader_time_seconds as f32,
            lod_view,
            &receivers[..receiver_count],
            enabled,
        )
        .and_then(|_| runtime.align_startup(&reference.0).map(|_| ()))
        .and_then(|_| runtime.advance_recovery(&reference.0));
    match result {
        Ok(moved) => {
            if moved {
                drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
                clear_input(&mut drive);
            }
        }
        Err(error) => {
            game_failure(&error.to_string());
            drive.runtime.stream.fail(error);
            drive.runtime.clear_input();
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
fn sync_instruments(
    time: Res<Time<Real>>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut visuals: ResMut<InstrumentVisuals>,
    mut transforms: Query<&mut Transform, Without<DriveCamera>>,
    mut materials: Query<&mut MeshMaterial3d<presentation::materials::SourceMaterial>>,
    mut casters: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let run = &drive.runtime;
    let amount = |id: ItemId| {
        run.area
            .items
            .get(id)
            .and_then(|i| i.fluid.as_ref())
            .map_or(0., |f| f.mixture.total())
    };
    let r = &reference.0.instruments;
    let sample = InstrumentSample {
        speed_kmh: run.physics.snapshot().speed_mps * 3.6,
        fuel_liters: amount(reference.0.engine.fuel_tank),
        oil_liters: amount(reference.0.engine.oil_tank),
        temperature: run.engine_output.temperature,
        rpm: run.engine_output.rpm,
        headlights: run.lighting.headlights(),
        local_ignition: run.engine_output.ignition
            && r.electronics_output > r.electronics_off_threshold,
        distance_m: run.trip_m,
    };
    if run.started && !run.paused {
        drive.instruments.update(r, sample, time.delta_secs());
    }
    visuals.update(
        &reference.0,
        &drive.instruments,
        &mut transforms,
        &mut materials,
        &mut casters,
    );
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
        runtime.stream.frame().surfaces.len() as u32,
        runtime.physics.collider_count() as u32,
        absolute[0],
        absolute[1],
        absolute[2],
        origin[0],
        origin[1],
        origin[2],
        runtime.rebases,
        runtime.trip_m,
        runtime.stream.metrics().prepared_roads as f64,
        fuel,
        runtime.paused,
        runtime.started,
    );
    let available = runtime.area.interaction.available_actions(
        &runtime.aim,
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
    );
    let info = runtime
        .area
        .interaction
        .feedback(&available, &runtime.area.items, &reference.0);
    let (amount, capacity, quantities, fluid_name, has_name, no_text) =
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
    let condition = available
        .item
        .and_then(|id| runtime.area.items.get(id))
        .map_or(-1.0, |item| item.condition);
    let held = runtime.area.interaction.held();
    let notice = if !runtime.notice.is_empty() {
        runtime.notice
    } else if let Some(block) = runtime.aim.installation_block.or(runtime.aim.grab_block) {
        block.notice()
    } else {
        ""
    };
    let stamp = FeedbackStamp {
        condition: condition.to_bits(),
        held,
        throw_charge: runtime.area.interaction.throw_charge().to_bits(),
        throw_active: runtime.area.interaction.throw_active(),
        rpm: runtime.engine_output.rpm.to_bits(),
        engine_flags: [
            runtime.engine_output.ignition,
            runtime.engine_output.running,
            runtime.engine_output.starter,
            runtime.area.player.seat().is_some(),
            runtime.handbrake,
        ],
        actions: info.actions.0,
        context: info.context as u32,
        light_control: available.light_control.map_or(0, |id| id.0),
        lmb_use: available.lmb_use,
        item: available.item,
        opening: available.opening,
        carrying: info.carrying,
        notice,
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
            info.name,
            stamp.notice,
            stamp.carrying,
            amount,
            capacity,
            has_name,
            no_text,
            fluid_name,
            condition,
            held.map(|id| display_name(&reference.0, &runtime.area.items, id))
                .unwrap_or(""),
            runtime.area.interaction.throw_charge(),
            runtime.area.interaction.throw_active(),
            runtime.engine_output.rpm,
            runtime.engine_output.ignition,
            runtime.engine_output.running,
            runtime.engine_output.starter,
            runtime.area.player.seat().is_some(),
            runtime.handbrake,
            quantities[0],
            quantities[1],
            quantities[2],
            quantities[3],
            quantities[4],
            quantities[5],
            stamp.light_control,
            stamp.lmb_use,
        );
        drive.last_feedback = Some(stamp);
    }
}
