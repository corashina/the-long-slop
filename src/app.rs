mod save;
#[cfg(feature = "save-fixture")]
mod save_fixture;
use crate::browser::*;
use crate::interaction::labels::display_name;
use crate::presentation::instruments::InstrumentVisuals;
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    reference::{BodyVisual, CapVisual, ItemVisual, ReferenceVisuals, ValveVisibility},
    world::{ChunkVisual, WorldVisuals},
};
use crate::reference::{ItemId, OpeningId, ReferenceManifest};
use crate::simulation::instruments::{InstrumentSample, InstrumentState};
use crate::simulation::{FrameInput, StepClock, keys};
use crate::starting_area::GarageRuntime;
use crate::vehicle::VehiclePose;
use bevy::asset::{AssetMetaCheck, LoadState, RecursiveDependencyLoadState};
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
    /// Gauge state per vehicle.
    instruments: Vec<InstrumentState>,
    input: FrameInput,
    pub(crate) clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    pub render_delta_seconds: f32,
    skip_elapsed: bool,
    pub(crate) ready: bool,
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
pub fn run(reference_json: &str, manifest_sha256: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let reference = Arc::new(
        ReferenceManifest::from_runtime_json(reference_json.as_bytes())
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    if let Err(error) = crate::reference::AudioReference::from_value(&reference.audio, &reference) {
        game_audio_error(&error.to_string());
    }
    let runtime = GarageRuntime::new(&reference).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let current = runtime.area.player.view_pose(&runtime.physics);
    App::new()
        .insert_resource(RuntimeReference(reference.clone()))
        .insert_resource(save::SaveRequests::new(&reference, manifest_sha256))
        .insert_resource(Drive {
            instruments: reference
                .vehicles
                .iter()
                .map(|vehicle| InstrumentState::new(&vehicle.instruments))
                .collect(),
            runtime,
            input: FrameInput::default(),
            clock: StepClock::default(),
            previous: current,
            current,
            rendered: current,
            camera: CameraMode::default(),
            frame_steps: 0,
            render_delta_seconds: 0.,
            skip_elapsed: false,
            ready: false,
            last_feedback: None,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(presentation::source_io::SourceIoPlugin)
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
            presentation::plates::PlateTextPlugin,
            presentation::mirrors::SourceMirrorPlugin,
            presentation::source_lighting::SourceLightingPlugin,
            presentation::frame_graph::SourceFramePlugin,
            presentation::weather::SourceWeatherPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                initialize_storage,
                check_ready,
                read_input,
                save::dispatch,
                presentation::lighting::bind_lighting,
                presentation::lighting::lighting_readiness,
                advance_physics,
                sync_world,
                sync_imported,
                presentation::consumables::sync,
                sync_instruments,
                presentation::lighting::sync_control_visuals,
                update_camera,
                presentation::frame_graph::sync_stack_cameras,
                update_hud,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            publish_audio
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
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
    mut materials: ResMut<Assets<presentation::materials::SourceMaterial>>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    gltf_extensions: Res<bevy::gltf::extensions::GltfExtensionHandlers>,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
) {
    let mut visuals = WorldVisuals::new(&mut meshes, &mut materials);
    visuals.sync(&drive.runtime.stream, &mut commands, &mut meshes);
    commands.insert_resource(visuals);
    commands.insert_resource(InstrumentVisuals::default());
    commands.insert_resource(presentation::consumables::ConsumableVisuals::default());
    presentation::materials::load_materials(
        &mut commands,
        &server,
        &reference.0.rendering,
        &reference.0.plates.material,
    );
    presentation::lighting::load_lighting(&mut commands, &server, &reference.0);
    presentation::lighting_assets::load_lighting(
        &mut commands,
        &server,
        &reference.0.render_lighting,
    );
    presentation::lighting::load_lamp_materials(&mut commands, &gltf_extensions, &reference.0);
    presentation::reference::load(&mut commands, &server, &reference.0);
    presentation::frame_graph::spawn_cameras(
        &mut commands,
        &mut images,
        &reference.0,
        presentation::transform(drive.current),
    );
}

fn initialize_storage(
    time: Res<Time<Real>>,
    mut drive: ResMut<Drive>,
    mut visuals: ResMut<ReferenceVisuals>,
) {
    if drive.ready || visuals.failed {
        return;
    }
    if let Err(error) = drive.runtime.initialize_storage_frame(time.delta_secs()) {
        game_failure(&error.to_string());
        visuals.failed = true;
    }
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
    mut readiness: Local<u8>,
    lighting: (
        Res<presentation::lighting::SourceLighting>,
        Res<presentation::source_lighting::SourceLighting>,
    ),
    mut materials: ResMut<presentation::materials::SourceMaterials>,
    mirrors: Res<presentation::mirrors::MirrorGpuStatus>,
    plates: Res<presentation::plates::PlateTextStatus>,
) {
    let (lighting, source_lighting) = lighting;
    if let Some(error) = mirrors.failure() {
        game_failure(&error);
        visuals.failed = true;
        return;
    }
    if let presentation::materials::RenderingState::Failed(error) = &source_lighting.state {
        game_failure(error);
        visuals.failed = true;
        return;
    }
    if let Some(error) = plates.failure() {
        game_failure(error);
        visuals.failed = true;
    }
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
        *readiness = 0;
        return;
    }
    if !drive
        .runtime
        .area
        .interaction
        .storage
        .initialization_ready()
    {
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
            if !mirrors.ready() || !plates.ready() {
                return;
            }
            if source_lighting.state != presentation::materials::RenderingState::Ready {
                return;
            }
            *readiness += 1;
            if *readiness >= 2 {
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
    drive.clock.reset();
    drive.previous = drive.current;
    drive.rendered = drive.current;
    drive.runtime.area.begin_step();
    drive.skip_elapsed = true;
}
fn read_input(
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    requests: Res<save::SaveRequests>,
) {
    let runtime = &drive.runtime;
    let assists = runtime.driving_assists;
    game_driving_state(
        runtime.vehicles[runtime.vehicle].drivetrain.gear() as i32 - 1,
        assists.automatic_clutch as u32 | ((assists.automatic_gears as u32) << 1),
        runtime.area.player.driving(),
    );
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
    if requests.pending() {
        return;
    }
    if actions & 2 != 0 {
        drive.runtime.paused = true;
        clear_input(&mut drive);
    }
    if actions & 4 != 0 && drive.ready && !drive.runtime.needs_output().menu_due {
        drive.runtime.started = true;
        drive.runtime.paused = false;
        clear_input(&mut drive);
    }

    if actions & 16 != 0 {
        drive.runtime.recover(&reference.0);
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
        clear_input(&mut drive);
    }
    let requested = game_driving_assists();
    if requested & 4 != 0 && !requests.pending() {
        let _ = drive
            .runtime
            .set_driving_assists(crate::reference::DrivingAssists {
                automatic_clutch: requested & 1 != 0,
                automatic_gears: requested & 2 != 0,
            });
    }
    if !drive.ready || !drive.runtime.started || drive.runtime.paused {
        drive.input.clear();
        return;
    }
    if actions & 22 != 0 {
        return;
    }
    if !drive.runtime.needs_output().dead
        && !drive.runtime.sleeping()
        && frame.pressed & keys::CAMERA != 0
        && drive.runtime.area.player.seat().is_some()
    {
        drive.runtime.audio_motion_revision = drive.runtime.audio_motion_revision.wrapping_add(1);
        drive.camera = match drive.camera {
            CameraMode::Cockpit => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Cockpit,
        };
    }
    drive.runtime.sleep_camera_allowed = drive.camera == CameraMode::Cockpit;
    drive.input.merge(frame);
}
fn advance_physics(
    time: Res<Time<Real>>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
) {
    drive.frame_steps = 0;
    drive.render_delta_seconds = 0.;
    if !drive.ready || !drive.runtime.started || drive.runtime.paused {
        return;
    }
    if drive.skip_elapsed {
        drive.skip_elapsed = false;
        return;
    }
    if let Some(audio) = &mut drive.runtime.audio {
        let calendar = game_audio_calendar();
        audio
            .radio
            .calendar((calendar / 100) as u8, (calendar % 100) as u8);
    }
    drive.clock.accept_frame(time.delta_secs_f64());
    loop {
        let scale = drive.runtime.clock_factors(&reference.0).time_scale;
        if !drive.clock.take_step(scale) {
            break;
        }
        drive.frame_steps += 1;
        drive.render_delta_seconds += crate::simulation::FIXED_DT;
        drive.previous = drive.current;
        let old_origin = drive.runtime.stream.origin.offset;
        let input = drive.input.take_step();
        drive.runtime.step(&reference.0, input);
        let new_origin = drive.runtime.stream.origin.offset;
        for i in 0..3 {
            drive.previous.position[i] -= (new_origin[i] - old_origin[i]) as f32;
        }
        drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
        if drive.runtime.paused {
            drive.clock.reset();
            break;
        }
    }
    let scale = drive.runtime.clock_factors(&reference.0).time_scale;
    drive.rendered =
        presentation::interpolate(drive.previous, drive.current, drive.clock.alpha(scale));
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
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    mut query: Query<(
        &BodyVisual,
        &mut Transform,
        &mut Visibility,
        Option<&ValveVisibility>,
    )>,
    mut items: Query<
        (
            &ItemVisual,
            &mut Transform,
            &mut Visibility,
            Option<&ValveVisibility>,
        ),
        Without<BodyVisual>,
    >,
    mut caps: Query<(&CapVisual, &mut Transform), (Without<BodyVisual>, Without<ItemVisual>)>,
    mut visibility: Query<
        (&ValveVisibility, &mut Visibility),
        (Without<BodyVisual>, Without<ItemVisual>),
    >,
) {
    presentation::reference::sync(
        &drive.runtime.area,
        &drive.runtime.physics,
        drive
            .clock
            .alpha(drive.runtime.clock_factors(&reference.0).time_scale),
        &mut query,
    );
    presentation::reference::sync_items(
        &drive.runtime.area,
        drive
            .clock
            .alpha(drive.runtime.clock_factors(&reference.0).time_scale),
        &mut items,
    );
    presentation::reference::sync_caps(&drive.runtime.area, &mut caps, &mut visibility);
}
fn instrument_sample(
    reference: &ReferenceManifest,
    run: &GarageRuntime,
    index: usize,
) -> InstrumentSample {
    let amount = |id: ItemId| {
        run.area
            .items
            .get(id)
            .and_then(|i| i.fluid.as_ref())
            .map_or(0., |f| f.mixture.total())
    };
    let vehicle = &reference.vehicles[index];
    let state = &run.vehicles[index];
    let r = &vehicle.instruments;
    InstrumentSample {
        speed_kmh: run.physics.snapshot(index).speed_mps * 3.6,
        fuel_liters: amount(vehicle.engine.fuel_tank),
        oil_liters: amount(vehicle.engine.oil_tank),
        temperature: state.engine_output.temperature,
        rpm: state.engine_output.rpm,
        headlights: run.lighting.headlights(index),
        local_ignition: state.engine_output.ignition
            && r.electronics_output > r.electronics_off_threshold,
        distance_m: state.trip_m,
    }
}
fn sync_instruments(
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut visuals: ResMut<InstrumentVisuals>,
    mut transforms: Query<&mut Transform, Without<DriveCamera>>,
    mut materials: Query<&mut MeshMaterial3d<presentation::materials::SourceMaterial>>,
    mut casters: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let drive = &mut *drive;
    let run = &drive.runtime;
    for (index, vehicle) in reference.0.vehicles.iter().enumerate() {
        let r = &vehicle.instruments;
        let sample = instrument_sample(&reference.0, run, index);
        if run.started && !run.paused {
            drive.instruments[index].update(r, sample, drive.render_delta_seconds);
        }
    }
    visuals.update(
        &reference.0,
        &drive.instruments,
        &mut transforms,
        &mut materials,
        &mut casters,
    );
}
fn active_camera(drive: &Drive) -> Transform {
    if let Some(camera) = drive.runtime.sleep_camera() {
        presentation::camera::sleep_pose(camera, &drive.runtime.stream.origin)
    } else if drive.camera == CameraMode::Chase && drive.runtime.area.player.seat().is_some() {
        presentation::camera::pose(
            CameraMode::Chase,
            drive.runtime.physics.snapshot(drive.runtime.vehicle).pose,
        )
    } else {
        presentation::transform(drive.rendered)
    }
}
fn update_camera(drive: Res<Drive>, mut query: Query<&mut Transform, With<DriveCamera>>) {
    for mut transform in &mut query {
        *transform = active_camera(&drive);
    }
}
fn update_hud(reference: Res<RuntimeReference>, mut drive: ResMut<Drive>) {
    let needs = drive.runtime.needs_output();
    game_survival(
        needs.needs.hp,
        needs.needs.food,
        needs.needs.water,
        needs.needs.bowel,
        needs.bladder_liters,
        needs.opacity,
        needs.dead,
        needs.look_scale,
        needs.menu_due,
    );
    let sleep = drive.runtime.sleep.output();
    game_sleep(
        sleep.phase as u32,
        sleep.eligible,
        sleep.charge,
        sleep.cooldown_remaining,
        sleep.progress,
    );
    let runtime = &drive.runtime;
    let current = &runtime.vehicles[runtime.vehicle];
    let snapshot = runtime.physics.snapshot(runtime.vehicle);
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
        .get(reference.0.vehicles[runtime.vehicle].engine.fuel_tank)
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
        current.trip_m,
        runtime.stream.lateral_band() as f64,
        fuel,
        runtime.paused,
        runtime.started,
    );
    let available = runtime.area.interaction.available_actions(
        &runtime.aim,
        &runtime.area.player,
        &runtime.area.items,
        &runtime.physics,
        runtime.survival.output(),
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
        rpm: current.engine_output.rpm.to_bits(),
        engine_flags: [
            current.engine_output.ignition,
            current.engine_output.running,
            current.engine_output.starter,
            runtime.area.player.seat().is_some(),
            current.handbrake,
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
            current.engine_output.rpm,
            current.engine_output.ignition,
            current.engine_output.running,
            current.engine_output.starter,
            runtime.area.player.seat().is_some(),
            current.handbrake,
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

fn publish_audio(
    reference: Res<RuntimeReference>,
    time: Res<Time<Real>>,
    mut drive: ResMut<Drive>,
    visuals: Res<ReferenceVisuals>,
    globals: Query<(&GlobalTransform, &InheritedVisibility)>,
    camera: Query<&GlobalTransform, With<DriveCamera>>,
) {
    let alpha = drive
        .clock
        .alpha(drive.runtime.clock_factors(&reference.0).time_scale);
    let runtime = &mut drive.runtime;
    let Some(audio) = &mut runtime.audio else {
        return;
    };
    game_audio_begin(
        audio.session,
        runtime.started,
        runtime.paused,
        audio.game_seconds,
    );
    let needs = runtime.survival.output();
    let level = if needs.dead {
        (1. - needs.death_seconds as f32 / reference.0.survival.death_seconds).clamp(0.001, 1.)
    } else {
        1.
    };
    game_audio_survival_master(
        10_f32.powf(level.ln()),
        runtime.audio_motion_revision,
        audio.game_seconds + f64::from(alpha) / 60.,
    );
    let shift = audio.take_origin_shift();
    if shift != [0.; 3] {
        game_audio_origin_shift(shift[0], shift[1], shift[2]);
    }
    if let Ok(camera) = camera.single() {
        let (_, q, p) = camera.to_scale_rotation_translation();
        game_audio_listener(p.x, p.y, p.z, q.x, q.y, q.z, q.w);
    }
    let body = crate::vehicle::to_rapier(runtime.area.player.body_pose(&runtime.physics));
    for b in &audio.reference.survival {
        let p = body * rapier3d::math::Vector::from_array(b.offset);
        game_audio_survival_emitter(b.kind, p.x, p.y, p.z);
    }
    let origin = runtime.stream.origin.offset;
    for e in &audio.survival_loops {
        let p = std::array::from_fn::<_, 3, _>(|i| (e.position[i] - origin[i]) as f32);
        game_audio_survival(
            e.kind as u32,
            e.emitter,
            e.action as u32,
            e.clip,
            e.gain,
            e.pitch,
            p[0],
            p[1],
            p[2],
        );
    }
    for e in &audio.survival_events {
        let p = std::array::from_fn::<_, 3, _>(|i| (e.position[i] - origin[i]) as f32);
        game_audio_survival(
            e.kind as u32,
            e.emitter,
            e.action as u32,
            e.clip,
            e.gain,
            e.pitch,
            p[0],
            p[1],
            p[2],
        );
    }
    audio.survival_events.clear();
    for emitter in &audio.reference.emitters {
        let alive = audio.owner_alive(emitter.id)
            && (emitter.id != audio.reference.engine.emitter
                || runtime
                    .area
                    .items
                    .mounted(reference.0.vehicles[audio.vehicle_index].engine.item))
            && emitter.item.is_none_or(|id| {
                runtime
                    .area
                    .items
                    .get(id)
                    .is_some_and(|item| item.inactive_pose.is_none())
            });
        if let Some((global, visibility)) = visuals
            .nodes
            .get(&emitter.node)
            .and_then(|&entity| globals.get(entity).ok())
        {
            let (_, q, p) = global.to_scale_rotation_translation();
            game_audio_emitter(
                emitter.id,
                p.x,
                p.y,
                p.z,
                q.x,
                q.y,
                q.z,
                q.w,
                alive && visibility.get(),
            );
        }
    }
    if runtime.started && !runtime.paused {
        audio.update_mixers(&runtime.area, time.delta_secs());
    }
    for (id, db) in audio.mixer_outputs() {
        game_audio_mixer(id, db);
    }
    for (index, route) in audio.reference.routes.iter().enumerate() {
        for &id in &route.voices {
            game_audio_route(id, audio.route_mixers[index]);
        }
    }
    for signal in &audio.signals {
        if audio.reference.radio.voices.contains(&signal.id) {
            continue;
        }
        game_audio_voice(
            signal.id,
            signal.playing,
            signal.gain,
            signal.pitch,
            signal.looping,
        );
    }
    game_audio_starter(
        audio.reference.engine.emitter,
        audio.starter_enabled,
        audio.game_seconds,
    );
    while let Some(event) = audio.pop_event() {
        game_audio_event(
            event.session,
            event.sequence,
            event.binding,
            event.emitter,
            event.position[0],
            event.position[1],
            event.position[2],
            event.gain,
            event.pitch,
            event.time_seconds,
        );
    }
    let radio = audio.radio_frame;
    game_audio_radio(
        radio.power,
        radio.am,
        radio.frequency,
        radio.volume,
        radio.static_gain,
        radio.primary.clip.unwrap_or(0),
        radio.primary.offset_seconds,
        radio.primary.gain,
        radio.primary.next_clip.unwrap_or(0),
        radio.secondary.clip.unwrap_or(0),
        radio.secondary.offset_seconds,
        radio.secondary.gain,
        radio.secondary.next_clip.unwrap_or(0),
    );
    game_audio_end();
}
