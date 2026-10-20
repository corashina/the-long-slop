mod performance;
pub(crate) mod save;
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
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// World generation time per frame while the loading panel shows.
pub(crate) const LOADING_BUDGET_MS: f64 = 12.;

#[derive(Resource)]
pub(crate) struct RuntimeReference(pub Arc<ReferenceManifest>);
#[derive(Resource)]
pub(crate) struct Drive {
    pub runtime: GarageRuntime,
    /// Gauge state per vehicle.
    instruments: Vec<InstrumentState>,
    input: FrameInput,
    tool_lmb: bool,
    tool_delta_seconds: f32,
    pub(crate) clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    physics_ms: f64,
    world_ms: f64,
    pub render_delta_seconds: f32,
    skip_elapsed: bool,
    pub(crate) ready: bool,
    last_feedback: Option<FeedbackStamp>,
    last_inventory: Option<([u32; 4], u8, u32)>,
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

/// Manifest fields the browser shell reads. Copies them without parsing the world records.
#[derive(serde::Deserialize, serde::Serialize)]
struct ManifestShell<'a> {
    #[serde(borrow)]
    app_id: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    build_id: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    source_hashes: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    driving: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    ui: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    sleep: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    audio: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    lighting: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    content_base: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    file_catalog: &'a serde_json::value::RawValue,
    #[serde(borrow)]
    items: &'a serde_json::value::RawValue,
}

#[wasm_bindgen]
pub fn manifest_shell(reference_json: &[u8]) -> Result<String, JsValue> {
    let shell: ManifestShell =
        serde_json::from_slice(reference_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&shell).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn run(reference_json: &[u8], manifest_sha256: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let reference = Arc::new(
        ReferenceManifest::from_runtime_json(reference_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    if let Err(error) = crate::reference::AudioReference::from_value(&reference.audio, &reference) {
        game_audio_error(&error.to_string());
    }
    let mut runtime =
        GarageRuntime::new(&reference).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (fixed, graph) = crate::reference::fixed_reservation::admit_control(
        &mut runtime.stream,
        presentation::frame_graph::control_bytes(),
        presentation::frame_graph::SourceGraphProgress::default,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let allocation = presentation::world::runtime::SourcePresentationAllocation::bootstrap(
        runtime.stream.frame().request.session,
        fixed,
    );
    let current = runtime.area.player.view(&runtime.physics).camera_pose;
    App::new()
        .insert_resource(graph)
        .insert_resource(allocation)
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
            tool_lmb: false,
            tool_delta_seconds: 0.,
            clock: StepClock::default(),
            previous: current,
            current,
            rendered: current,
            camera: CameraMode::default(),
            frame_steps: 0,
            physics_ms: 0.,
            world_ms: 0.,
            render_delta_seconds: 0.,
            skip_elapsed: false,
            ready: false,
            last_feedback: None,
            last_inventory: None,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<crate::render_scale::SceneScale>()
        .add_systems(PreUpdate, update_scene_scale)
        .add_plugins(presentation::source_io::SourceIoPlugin::new(
            reference.content_base.clone(),
            Arc::clone(&reference.file_catalog),
        ))
        .add_plugins(
            DefaultPlugins
                .set(bevy::render::RenderPlugin {
                    render_creation: {
                        let settings = bevy::render::settings::WgpuSettings::default();
                        bevy::render::settings::WgpuSettings {
                            features: settings.features | bevy::render::render_resource::WgpuFeatures::TEXTURE_COMPRESSION_BC,
                            ..settings
                        }.into()
                    },
                    ..default()
                })
                // The source frame graph draws Bevy phases in its own passes, without the GPU preprocessing nodes.
                .set(bevy::pbr::PbrPlugin {
                    use_gpu_instance_buffer_builder: false,
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: reference.content_base.clone(),
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
            presentation::conditions::ConditionPlugin,
            presentation::source_lighting::SourceLightingPlugin,
            presentation::frame_graph::SourceFramePlugin,
            presentation::particles::SourceParticlePlugin,
            presentation::world::prototypes::SourcePrototypePlugin,
            presentation::world::pipelines::WorldPipelinePlugin,
        ))
        .init_resource::<presentation::save::CandidateImportedPoses>()
        .init_resource::<presentation::world::SourceWorldVisuals>()
        .init_resource::<presentation::world::instances::SourceInstances>()
        .init_resource::<presentation::world::grass_jobs::GrassJobs>()
        .init_resource::<crate::world::metrics::browser::MeshMeasurements>()
        .add_systems(
            Last,
            (
                crate::world::metrics::browser::collect_mesh_bytes,
                crate::world::metrics::browser::publish,
                performance::publish,
            )
                .chain(),
        )
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
                advance_tools,
                presentation::consumables::sync,
                sync_instruments,
                presentation::lighting::sync_control_visuals,
                presentation::reference::sync_inventory,
                update_camera,
                presentation::frame_graph::sync_sky_camera,
                publish_loading,
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

    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
    gltf_extensions: Res<bevy::gltf::extensions::GltfExtensionHandlers>,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
) {
    commands.insert_resource(InstrumentVisuals::default());
    commands.insert_resource(presentation::consumables::ConsumableVisuals::default());
    if let Err(error) =
        presentation::materials::load_materials(&mut commands, &server, &reference.0)
    {
        game_failure(&error.to_string());
        return;
    }
    presentation::lighting::load_lighting(&mut commands, &server, &reference.0.lighting);
    presentation::lighting_assets::load_lighting(&mut commands, &server, Arc::clone(&reference.0));
    presentation::lighting::load_lamp_materials(&mut commands, &gltf_extensions, &reference.0);
    presentation::reference::load(&mut commands, &server, &reference.0);
    match presentation::fixed_assets::SourceFixedAssets::new(&reference.0) {
        Ok(fixed) => commands.insert_resource(fixed),
        Err(error) => {
            game_failure(&error.to_string());
            return;
        }
    };
    presentation::frame_graph::spawn_cameras(
        &mut commands,
        &mut images,
        &reference.0,
        presentation::transform(drive.current),
        drive
            .runtime
            .area
            .player
            .view(&drive.runtime.physics)
            .fov_degrees,
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

fn update_scene_scale(
    mut scale: ResMut<crate::render_scale::SceneScale>,
    status: Res<presentation::source_lighting::SourceLightingStatus>,
) {
    match crate::render_scale::SceneScale::from_factor(game_scene_scale()) {
        Ok(value) => {
            if *scale != value {
                *scale = value;
            }
        }
        Err(error) => {
            *status.0.lock().unwrap() =
                presentation::materials::RenderingState::Failed(error.into());
        }
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
    mut readiness: Local<crate::save::fresh_permit::PresentedFrames>,
    lighting: (
        Res<presentation::lighting::SourceLighting>,
        Res<presentation::source_lighting::SourceLighting>,
        Res<presentation::fixed_assets::SourceFixedAssets>,
        Res<presentation::source_lighting::SourceLightingStatus>,
    ),
    mut materials: ResMut<presentation::materials::SourceMaterials>,
    plates: (
        Res<presentation::plates::PlateTextStatus>,
        Res<presentation::particles::ParticleRender>,
        Res<presentation::view_frame::SourceSubmissionAcks>,
        Res<presentation::view_frame::SourceViewClock>,
        Res<presentation::view_frame::SourceSceneEpoch>,
        Res<presentation::frame_graph::SourceGraphProgress>,
        Res<presentation::lighting_scene::ProbeActivation>,
        Query<(&presentation::surface_views::SourceSurfaceView, &Camera)>,
        Query<&Window>,
        Res<crate::render_scale::SceneScale>,
    ),
) {
    let (lighting, source_lighting, fixed, status) = lighting;
    let (
        plates,
        particles,
        submissions,
        view_clock,
        epoch,
        graph,
        activation,
        cameras,
        windows,
        scale,
    ) = plates;
    graph.content_checked(None);
    let shared_status = status.0.lock().unwrap();
    if let presentation::materials::RenderingState::Failed(error) = &*shared_status {
        game_failure(error);
        drop(shared_status);
        visuals.failed = true;
        graph.invalidate_loading();
        readiness.reset();
        return;
    }
    drop(shared_status);
    if let presentation::materials::RenderingState::Failed(error) = &source_lighting.state {
        game_failure(error);
        visuals.failed = true;
        graph.invalidate_loading();
        readiness.reset();
        return;
    }
    if let Some(error) = plates.failure() {
        game_failure(error);
        visuals.failed = true;
        graph.invalidate_loading();
    }
    if visuals.failed {
        readiness.reset();
        return;
    }
    if let presentation::lighting::RenderingState::Failed(error) = &lighting.state {
        game_failure(error);
        visuals.failed = true;
        graph.invalidate_loading();
        readiness.reset();
        return;
    }
    if let presentation::materials::RenderingState::Failed(error) = &materials.state {
        game_failure(error);
        visuals.failed = true;
        graph.invalidate_loading();
        readiness.reset();
        return;
    }
    if drive.ready {
        readiness.reset();
        return;
    }
    if !fixed.payloads_ready() || !materials.fixed_textures_ready() {
        readiness.reset();
        return;
    }
    if !drive
        .runtime
        .area
        .interaction
        .storage
        .initialization_ready()
    {
        readiness.reset();
        return;
    }
    for (path, handle) in &visuals.handles {
        let Some((load, _, recursive)) = server.get_load_states(handle.id()) else {
            readiness.reset();
            return;
        };
        if let LoadState::Failed(error) = load {
            game_failure(&format!("Cannot load {path}: {error}"));
            visuals.failed = true;
            graph.invalidate_loading();
            readiness.reset();
            return;
        }
        if let RecursiveDependencyLoadState::Failed(error) = recursive {
            game_failure(&format!("Cannot load dependencies for {path}: {error}"));
            visuals.failed = true;
            graph.invalidate_loading();
            readiness.reset();
            return;
        }
        if !server.is_loaded_with_dependencies(handle.id()) || !gltfs.contains(handle.id()) {
            readiness.reset();
            return;
        }
        if gltfs.get(handle).unwrap().scenes.is_empty() {
            game_failure(&format!("Required source scene 0 is missing from {path}"));
            visuals.failed = true;
            graph.invalidate_loading();
            readiness.reset();
            return;
        }
    }
    if !lighting.materials_ready {
        readiness.reset();
        return;
    }
    if !visuals.spawned {
        presentation::reference::spawn_scenes(&mut commands, &reference.0, &mut visuals, &gltfs);
        readiness.reset();
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
                Ok(false) => {
                    readiness.reset();
                    return;
                }
                Err(e) => {
                    game_failure(&e.to_string());
                    visuals.failed = true;
                    graph.invalidate_loading();
                    readiness.reset();
                    return;
                }
            }
            match &lighting.state {
                presentation::lighting::RenderingState::Loading => {
                    readiness.reset();
                    return;
                }
                presentation::lighting::RenderingState::Failed(error) => {
                    game_failure(error);
                    visuals.failed = true;
                    graph.invalidate_loading();
                    readiness.reset();
                    return;
                }
                presentation::lighting::RenderingState::Ready => {}
            }
            if materials.state != presentation::materials::RenderingState::Ready {
                readiness.reset();
                return;
            }
            if !drive.runtime.world_supported()
                || !drive.runtime.stream.required_ready()
                || !lighting.road_lamps.ready()
            {
                readiness.reset();
                return;
            }
            if !plates.ready() || !particles.prepared() {
                readiness.reset();
                return;
            }
            let session = drive.runtime.stream.frame().request.session;
            let revision = drive.runtime.stream.required_content().selection_revision;
            graph.content_checked(Some((session, revision)));
            if source_lighting.state != presentation::materials::RenderingState::Ready {
                readiness.reset();
                return;
            }
            let extent =
                cameras
                    .iter()
                    .find(|(view, _)| view.index == 0)
                    .and_then(|(_, camera)| {
                        let extent = camera.physical_viewport_size()?;
                        let target = camera.physical_target_size()?;
                        (extent == target
                            && windows.single().is_ok_and(|window| {
                                window.physical_width() == target.x
                                    && window.physical_height() == target.y
                            }))
                        .then_some(scale.extent(extent.to_array()))
                    });
            let enabled = activation
                .enabled
                .iter()
                .enumerate()
                .fold(0, |mask, (i, enabled)| mask | (u32::from(*enabled) << i));
            if extent.is_none_or(|extent| {
                !graph.complete_scene(
                    crate::reference::loading_dispatch::LoadingKey {
                        session,
                        scene: epoch.0,
                        extent,
                    },
                    revision,
                    enabled,
                )
            }) {
                readiness.reset();
                return;
            }
            if readiness.observe(
                drive.runtime.stream.frame().request.session,
                epoch.0,
                view_clock.main_token(),
                submissions.main_token(),
            ) {
                drive.ready = true;
                game_ready();
            }
        }
        Ok(false) => readiness.reset(),
        Err(error) => {
            game_failure(&error);
            visuals.failed = true;
            graph.invalidate_loading();
            readiness.reset();
        }
    }
}

fn clear_input(drive: &mut Drive) {
    drive.input.clear();
    drive.tool_lmb = false;
    drive.tool_delta_seconds = 0.;
    drive.runtime.clear_input();
    drive.current = drive
        .runtime
        .area
        .player
        .view(&drive.runtime.physics)
        .camera_pose;
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
    let held = game_keys();
    let player = game_player_keys();
    let frame = FrameInput {
        held,
        player_held: player as u8 & 3,
        player_pressed: (player >> 2) as u8 & 3,
        player_released: (player >> 4) as u8 & 3,
        pressed: game_pressed(),
        released: game_released(),
        inventory_held: game_inventory_keys() as u8 & 15,
        inventory_released: (game_inventory_keys() >> 4) as u8 & 15,
        mouse: [
            game_mouse_x() * reference.0.player.look_sensitivity[0],
            game_mouse_y() * reference.0.player.look_sensitivity[1],
        ],
        wheel: game_wheel(),
    };
    drive.tool_lmb = frame.held & keys::PRIMARY != 0;
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
        drive.current = drive
            .runtime
            .area
            .player
            .view(&drive.runtime.physics)
            .camera_pose;
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
    drive.physics_ms = 0.;
    drive.frame_steps = 0;
    drive.render_delta_seconds = 0.;
    drive.tool_delta_seconds = 0.;
    if !drive.ready || !drive.runtime.started || drive.runtime.paused {
        return;
    }
    if drive.skip_elapsed {
        drive.skip_elapsed = false;
        return;
    }
    let started = game_now();
    if let Some(audio) = &mut drive.runtime.audio {
        let calendar = game_audio_calendar();
        audio
            .radio
            .calendar((calendar / 100) as u8, (calendar % 100) as u8);
    }
    let delta = time.delta_secs() * drive.runtime.clock_factors(&reference.0).time_scale;
    drive.tool_delta_seconds = delta;
    drive.runtime.area.conditions.update(delta, true);
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
        drive.current = drive
            .runtime
            .area
            .player
            .view(&drive.runtime.physics)
            .camera_pose;
        if drive.runtime.paused {
            drive.clock.reset();
            break;
        }
    }
    let scale = drive.runtime.clock_factors(&reference.0).time_scale;
    drive.rendered =
        presentation::interpolate(drive.previous, drive.current, drive.clock.alpha(scale));
    drive.physics_ms = game_now() - started;
}
fn sync_world(
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
    mut presentation: presentation::world::runtime::WorldPresentation,
    requests: Res<save::SaveRequests>,
    camera: Query<(&Projection, &Transform), With<DriveCamera>>,
    views: Res<presentation::view_frame::SourceViewFrame>,
    mut required_views: Local<Option<crate::world::RequiredViews>>,
    epoch: Res<presentation::view_frame::SourceSceneEpoch>,
    windows: Query<&Window>,
) {
    drive.world_ms = 0.;
    if requests.freezes_world() {
        return;
    }
    let Ok((projection, camera_transform)) = camera.single() else {
        return;
    };
    // Preparation consumes one completed pose/FOV pair. Startup and load use the root camera.
    let coherent = views.session == drive.runtime.stream.frame().request.session
        && views.scene == epoch.0
        && views.views[..presentation::surface_views::WORLD_VIEWS]
            .iter()
            .all(Option::is_some);
    let snapshot = coherent.then(|| {
        views.views[0]
            .as_ref()
            .expect("complete world camera snapshot")
    });
    let camera_world = snapshot.map_or_else(
        || camera_transform.to_matrix(),
        |view| view.view_from_world.inverse(),
    );
    let clip_from_view = snapshot.map_or_else(
        || projection.get_clip_from_view(),
        |view| view.raster_projection,
    );
    let fov_degrees = snapshot.map_or_else(
        || match projection {
            Projection::Perspective(p) => p.fov.to_degrees(),
            _ => 0.,
        },
        |view| view.fov_degrees,
    );
    let started = game_now();
    let origin = drive.runtime.stream.origin;
    let position = Vec3::from_array(
        origin
            .absolute(camera_world.w_axis.truncate().to_array())
            .map(|v| v as f32),
    );
    let source_from_runtime = Mat4::from_scale(Vec3::new(1., 1., -1.));
    let mut camera_world = camera_world;
    camera_world.w_axis = position.extend(1.);
    let clip = clip_from_view * camera_world.inverse() * source_from_runtime;
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
        position: snapshot.map_or_else(
            || camera_transform.translation,
            |view| view.view_from_world.inverse().w_axis.truncate(),
        ),
        fov_degrees,
        ortho_size: match projection {
            Projection::Orthographic(p) => Some(p.area.height() * 0.5),
            _ => None,
        },
    };
    // The loading panel covers the screen, so generation can use most of each frame.
    let budget_ms = if drive.ready { 0. } else { LOADING_BUDGET_MS };
    let runtime = &mut drive.runtime;
    runtime.stream.advance_generation();
    let was_aligned = runtime.startup_aligned;
    let enabled = runtime
        .lighting
        .road_lamp_output(&reference.0.lighting)
        .enabled;
    let [x, y, z] = runtime
        .lighting
        .cycle
        .output()
        .directional_euler_degrees
        .map(f32::to_radians);
    let forward = Quat::from_euler(EulerRot::YXZ, -y, -x, z) * Vec3::NEG_Z;
    let forward = [forward.x, forward.y, -forward.z];
    if coherent {
        let first = views.views[0]
            .as_ref()
            .expect("complete world camera snapshot");
        let result = (|| -> Result<(), crate::reference::ReferenceError> {
            let first = presentation::view_frame::required_view(
                first,
                runtime.stream.origin.offset,
                forward,
                reference.0.render_lighting.lights[0].mask,
            )?;
            let required =
                required_views.get_or_insert_with(|| crate::world::RequiredViews::new(first));
            required.reset(first);
            presentation::view_frame::append_probe_required_views(
                required,
                &reference.0.render_lighting.probes,
                &views.probe_position,
                &views.probe_enabled,
                runtime.stream.origin.offset,
                forward,
                reference.0.render_lighting.lights[0].mask,
            )
        })();
        if let Err(error) = result {
            runtime.stream.fail(error);
            return;
        }
    } else {
        *required_views = None;
    }
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
            enabled,
            runtime.lighting.light_outputs(),
            required_views
                .as_ref()
                .map_or(&[], crate::world::RequiredViews::as_slice),
            budget_ms,
        )
        .and_then(|_| runtime.align_startup(&reference.0).map(|_| ()))
        .and_then(|_| {
            if runtime.has_deferred_save_links() {
                runtime
                    .advance_saved_restore(&reference.0)
                    .map_err(|e| crate::reference::ReferenceError(e.to_string()))?;
            }
            runtime
                .sync_world_activity(&reference.0)
                .map_err(|e| crate::reference::ReferenceError(e.to_string()))?;
            runtime.advance_recovery(&reference.0)
        });
    match result {
        Ok(moved) => {
            if moved || (!was_aligned && drive.runtime.startup_aligned) {
                drive.current = drive
                    .runtime
                    .area
                    .player
                    .view(&drive.runtime.physics)
                    .camera_pose;
                clear_input(&mut drive);
            }
        }
        Err(error) => {
            game_failure(&error.to_string());
            drive.runtime.stream.fail(error);
            drive.runtime.clear_input();
        }
    }
    drive.world_ms = game_now() - started;
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
    mut caps: Query<
        (&CapVisual, &mut Transform),
        (
            Without<BodyVisual>,
            Without<ItemVisual>,
            Without<presentation::save::HiddenWorld>,
        ),
    >,
    mut visibility: Query<
        (&ValveVisibility, &mut Visibility),
        (
            Without<BodyVisual>,
            Without<ItemVisual>,
            Without<presentation::save::HiddenWorld>,
        ),
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
pub(crate) fn advance_tools(reference: Res<RuntimeReference>, mut drive: ResMut<Drive>) {
    let frame = crate::interaction::cleaning::ToolFrame {
        held: drive.runtime.area.interaction.held(),
        lmb: drive.tool_lmb,
        active: drive.ready
            && !drive.runtime.recovering
            && drive.runtime.world_supported()
            && drive.tool_delta_seconds > 0.,
        dt: drive.tool_delta_seconds,
        view: drive.rendered,
        origin: drive.runtime.stream.origin.offset,
    };
    drive.runtime.advance_tools(&reference.0, frame);
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
fn update_camera(
    drive: Res<Drive>,
    mut query: Query<(&mut Transform, &mut Projection), With<DriveCamera>>,
) {
    let fov = drive
        .runtime
        .area
        .player
        .view(&drive.runtime.physics)
        .fov_degrees;
    for (mut transform, mut projection) in &mut query {
        *transform = active_camera(&drive);
        presentation::camera_projection::sync_player_projection(&mut projection, true, fov);
    }
}
fn publish_loading(
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    visuals: Res<ReferenceVisuals>,
    materials: Res<presentation::materials::SourceMaterials>,
    lighting: Res<presentation::lighting::SourceLighting>,
    source_lighting: Res<presentation::source_lighting::SourceLighting>,
    plates: Res<presentation::plates::PlateTextStatus>,
    time: Res<Time<Real>>,
    mut last: Local<f64>,
    mut traced: Local<u8>,
) {
    let now = time.elapsed_secs_f64();
    let stream = &drive.runtime.stream;
    let rendering = visuals.bound
        && materials.state == presentation::materials::RenderingState::Ready
        && lighting.state == presentation::lighting::RenderingState::Ready
        && source_lighting.state == presentation::materials::RenderingState::Ready
        && plates.ready();
    if stream.initial_generation_ready() && *traced & 1 == 0 {
        game_startup_stage("generation", false);
        *traced |= 1;
    }
    if visuals.bound && *traced & 2 == 0 {
        game_startup_stage("decodeUpload", false);
        *traced |= 2;
    }
    if rendering && *traced & 4 == 0 {
        game_startup_stage("pipeline", false);
        *traced |= 4;
    }
    if drive.ready || visuals.failed || now - *last < 0.5 {
        return;
    }
    *last = now;
    game_loading_progress(
        stream.metrics().prepared_roads,
        reference.0.world.road.gen_at_start,
        rendering,
        drive.runtime.world_supported() && stream.required_ready(),
    );
    for budget in stream.presentation_budgets(stream.presentation_allocation()) {
        game_loading_budget(
            budget.name,
            budget.used as f64,
            budget.limit as f64,
            budget.unit,
        );
    }
}

fn update_hud(reference: Res<RuntimeReference>, mut drive: ResMut<Drive>) {
    let state = &drive.runtime.area.inventory;
    let stamp = (
        state.slots.map(|id| id.map_or(0, |id| id.0)),
        state.selected,
        state.swap_progress.to_bits(),
    );
    if drive.last_inventory != Some(stamp) {
        game_inventory(
            stamp.1 as u32,
            stamp.0[0],
            stamp.0[1],
            stamp.0[2],
            stamp.0[3],
            f32::from_bits(stamp.2),
        );
        drive.last_inventory = Some(stamp);
    }
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
        runtime.stream.frame().surfaces.len() as u32,
        runtime.physics.collider_count() as u32,
        absolute[0],
        absolute[1],
        absolute[2],
        origin[0],
        origin[1],
        origin[2],
        runtime.rebases,
        current.trip_m,
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
        runtime.survival.output(),
    );
    let info = runtime
        .area
        .interaction
        .feedback(&available, &runtime.area.items, &reference.0);
    let (mut amount, mut capacity, quantities, fluid_name, has_name, mut no_text) = info
        .fluid
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
    let paint = runtime
        .area
        .interaction
        .paint_feedback(&runtime.aim, &reference.0);
    if let Some((quantity, maximum)) = paint {
        amount = quantity;
        capacity = maximum;
        no_text = false;
    }
    let condition = available
        .item
        .and_then(|id| {
            runtime.area.conditions.item_quality(id).or_else(|| {
                runtime
                    .area
                    .items
                    .get(id)
                    .and_then(|i| i.generated_condition)
            })
        })
        .unwrap_or(-1.0);
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
        context: if paint.is_some() {
            crate::interaction::feedback::FeedbackContext::Paint as u32
        } else {
            info.context as u32
        },
        light_control: available.light_control.map_or(0, |id| id.0),
        lmb_use: available.lmb_use,
        item: available.item,
        opening: available.opening,
        carrying: info.carrying,
        notice,
        amount: if paint.is_some() {
            amount.round() as u32
        } else {
            (amount * 10.).round() as u32
        },
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
