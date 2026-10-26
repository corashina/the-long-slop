pub(crate) use simulation::cpu_profile;
mod performance;
pub(crate) mod save;
use crate::browser::*;
use crate::presentation::{
    self,
    camera::{CameraMode, DriveCamera},
    reference::{BodyVisual, CapVisual, ItemVisual, ReferenceVisuals, ValveVisibility},
};
use crate::reference::ReferenceManifest;
use crate::simulation::{FrameInput, StepClock, keys};
use crate::starting_area::GarageRuntime;
use crate::vehicle::VehiclePose;
use bevy::asset::{AssetMetaCheck, LoadState, RecursiveDependencyLoadState};
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use presentation::sim_frame::SimFrameResource;
use simulation::boundary::{AudioCall, HudBlock, PoseRow, RenderInput, StateBlock};
use simulation::host::{game_cpu_clear_input, game_now};
use simulation::world::metrics::BUDGET_LABELS;
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
    input: FrameInput,
    frame_input: FrameInput,
    pub(crate) clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    rendered: VehiclePose,
    camera: CameraMode,
    frame_steps: u32,
    physics_ms: f64,
    world_ms: f64,
    pub render_delta_seconds: f32,
    /// Camera position of the last rendered frame. The simulation assigns radio voices by it.
    pub listener: Option<[f32; 3]>,
    skip_elapsed: bool,
    pub(crate) ready: bool,
    last_inventory: Option<([u32; 4], u8, u32)>,
}

impl Drive {
    /// Player body `pose` at the rendered camera. It moves with the camera interpolation.
    pub(crate) fn rendered_body_pose(&self, mut pose: VehiclePose) -> VehiclePose {
        for i in 0..3 {
            pose.position[i] += self.rendered.position[i] - self.current.position[i];
        }
        pose
    }
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
    #[serde(borrow)]
    world: WorldShell<'a>,
}
#[derive(serde::Deserialize, serde::Serialize)]
struct WorldShell<'a> {
    #[serde(borrow)]
    draw_distance: &'a serde_json::value::RawValue,
    #[serde(borrow, skip_serializing)]
    startup: Vec<StartupShell<'a>>,
    /// Start car fields in export order. The Game mode start car list shows them.
    #[serde(skip_deserializing)]
    start_cars: Vec<&'a str>,
}
#[derive(serde::Deserialize)]
struct StartupShell<'a> {
    group: &'a str,
    start_car: bool,
}

#[wasm_bindgen]
pub fn manifest_shell(reference_json: &[u8]) -> Result<String, JsValue> {
    let mut shell: ManifestShell =
        serde_json::from_slice(reference_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    shell.world.start_cars = shell
        .world
        .startup
        .iter()
        .filter(|s| s.start_car)
        .map(|s| s.group)
        .collect();
    serde_json::to_string(&shell).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// The Game mode from the browser. Biome values above 3 read as Mixed.
/// The start car is -1 (Random) or an index into the manifest start placements.
fn game_mode(reference: &ReferenceManifest) -> crate::world::GameMode {
    let biome = game_biome();
    crate::world::GameMode {
        biome: if biome > 3 { 0 } else { biome as u8 },
        start_car: crate::world::StartCarChoice::from_index(&reference.world, game_start_car()),
    }
}

#[wasm_bindgen]
pub fn run(
    reference_json: &[u8],
    manifest_sha256: String,
    draw_distance: f32,
) -> Result<(), JsValue> {
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::Startup);
    console_error_panic_hook::set_once();
    let reference = Arc::new(
        ReferenceManifest::from_runtime_json(reference_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    if let Err(error) = crate::reference::AudioReference::from_value(&reference.audio, &reference) {
        game_audio_error(&error.to_string());
    }
    let mut runtime = GarageRuntime::new(&reference, game_mode(&reference), draw_distance)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (fixed, graph) = crate::reference::fixed_reservation::admit_control(
        &mut runtime.stream,
        presentation::readiness::control_bytes(),
        presentation::readiness::RendererStateHandle::default,
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
            runtime,
            input: FrameInput::default(),
            frame_input: FrameInput::default(),
            clock: StepClock::default(),
            previous: current,
            current,
            rendered: current,
            camera: CameraMode::default(),
            frame_steps: 0,
            physics_ms: 0.,
            world_ms: 0.,
            render_delta_seconds: 0.,
            listener: None,
            skip_elapsed: false,
            ready: false,
            last_inventory: None,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<crate::render_scale::SceneScaleSetting>()
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
                .set(AssetPlugin {
                    file_path: reference.content_base.clone(),
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "The Long Slop".into(),
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(presentation::readiness::install)
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
        .init_resource::<presentation::computer::ComputerScreens>()
        .init_resource::<SimFrameResource>()
        .init_resource::<presentation::world::grass_jobs::GrassJobs>()
        .init_resource::<crate::presentation::world_metrics::MeshMeasurements>()
        .add_systems(
            Last,
            (
                crate::presentation::world_metrics::collect_mesh_bytes,
                crate::presentation::world_metrics::publish,
                performance::publish,
            )
                .chain(),
        )
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                initialize_storage,
                presentation::readiness::observe_main_readiness.before(check_ready),
                check_ready,
                read_input,
                save::dispatch,
                presentation::lighting::bind_lighting,
                presentation::lighting::lighting_readiness,
                advance_physics,
                (sync_world, fill_frame).chain(),
                sync_imported,
                presentation::reference::sync_home_cable,
                presentation::consumables::sync,
                (
                    presentation::spawned::sync,
                    presentation::cigarettes::sync,
                    presentation::player_body::sync,
                    presentation::ragdoll::sync,
                    presentation::sky_traffic::sync,
                )
                    .chain(),
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
    commands.insert_resource(presentation::consumables::ConsumableVisuals::default());
    commands.insert_resource(presentation::spawned::SpawnedVisuals::default());
    commands.insert_resource(presentation::player_body::PlayerBody::default());
    commands.insert_resource(presentation::ragdoll::RagdollBody::default());
    commands.insert_resource(presentation::sky_traffic::SkyTrafficVisuals::default());
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
        drive.runtime.distances.cam2_far_plane,
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
    mut scale: ResMut<crate::render_scale::SceneScaleSetting>,
    status: Res<presentation::readiness::RendererStateHandle>,
) {
    match crate::render_scale::SceneScale::from_factor(game_scene_scale()) {
        Ok(value) => {
            if scale.0 != value {
                scale.0 = value;
            }
        }
        Err(error) => {
            status.fail(error.into());
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
    extras: Query<(Entity, &GltfExtras)>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    mut renderer: presentation::readiness::RendererReadinessAccess,
    lighting: (
        Res<presentation::lighting::SourceLighting>,
        Res<presentation::fixed_assets::SourceFixedAssets>,
    ),
    materials: Res<presentation::materials::SourceMaterials>,
    plates: (
        Res<presentation::plates::PlateTextStatus>,
        Res<presentation::particles::ParticleRender>,
    ),
) {
    let (lighting, fixed) = lighting;
    let (plates, particles) = plates;
    let prepared = (|| -> bool {
        if renderer.failure().is_some() {
            return false;
        }
        if let Some(error) = plates.failure() {
            game_failure(error);
            visuals.failed = true;
        }
        if visuals.failed {
            return false;
        }
        if let presentation::lighting::RenderingState::Failed(error) = &lighting.state {
            game_failure(error);
            visuals.failed = true;
            return false;
        }
        if let presentation::materials::RenderingState::Failed(error) = &materials.state {
            game_failure(error);
            visuals.failed = true;
            return false;
        }
        if drive.ready {
            return true;
        }
        if !fixed.payloads_ready() || !materials.fixed_textures_ready() {
            return false;
        }
        if !drive
            .runtime
            .area
            .interaction
            .storage
            .initialization_ready()
        {
            return false;
        }
        for (path, handle) in &visuals.handles {
            let Some((load, _, recursive)) = server.get_load_states(handle.id()) else {
                return false;
            };
            if let LoadState::Failed(error) = load {
                game_failure(&format!("Cannot load {path}: {error}"));
                visuals.failed = true;
                return false;
            }
            if let RecursiveDependencyLoadState::Failed(error) = recursive {
                game_failure(&format!("Cannot load dependencies for {path}: {error}"));
                visuals.failed = true;
                return false;
            }
            if !server.is_loaded_with_dependencies(handle.id()) || !gltfs.contains(handle.id()) {
                return false;
            }
            if gltfs.get(handle).unwrap().scenes.is_empty() {
                game_failure(&format!("Required source scene 0 is missing from {path}"));
                visuals.failed = true;
                return false;
            }
        }
        if !lighting.materials_ready {
            return false;
        }
        if !visuals.spawned {
            presentation::reference::spawn_scenes(
                &mut commands,
                &reference.0,
                &mut visuals,
                &gltfs,
            );
            return false;
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
                    presentation::lighting::RenderingState::Loading => {
                        return false;
                    }
                    presentation::lighting::RenderingState::Failed(error) => {
                        game_failure(error);
                        visuals.failed = true;
                        return false;
                    }
                    presentation::lighting::RenderingState::Ready => {}
                }
                if materials.state != presentation::materials::RenderingState::Ready {
                    return false;
                }
                if !drive.runtime.world_supported()
                    || !drive.runtime.stream.required_ready()
                    || !lighting.road_lamps.ready()
                {
                    return false;
                }
                if !plates.ready() || !particles.prepared() {
                    return false;
                }
                true
            }
            Ok(false) => false,
            Err(error) => {
                game_failure(&error);
                visuals.failed = true;
                false
            }
        }
    })();
    let key = crate::render_readiness::ContentKey {
        session: drive.runtime.stream.frame().request.session,
        selection_revision: drive.runtime.stream.required_content().selection_revision,
    };
    let preparation = if prepared {
        crate::render_readiness::ContentPreparation::Prepared(key)
    } else {
        crate::render_readiness::ContentPreparation::Pending(key)
    };
    match renderer.check(presentation::readiness::RendererContentCheck {
        preparation,
        history_revision: drive.runtime.audio_motion_revision,
    }) {
        crate::render_readiness::RendererStatus::Loading => {}
        crate::render_readiness::RendererStatus::Ready => {
            if !drive.ready {
                drive.ready = true;
                game_ready();
            }
        }
        crate::render_readiness::RendererStatus::Failed(error) => {
            game_failure(&error);
            visuals.failed = true;
        }
    }
}

fn clear_input(drive: &mut Drive) {
    if cpu_profile::enabled() {
        game_cpu_clear_input();
    }
    drive.input.clear();
    drive.frame_input.clear();
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
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::Input);
    let runtime = &drive.runtime;
    let assists = runtime.driving_assists;
    let controls = runtime.driving_input.controls;
    game_driving_state(
        runtime
            .vehicle
            .as_ref()
            .map_or(0, |key| runtime.vehicles[key].drivetrain.gear() as i32 - 1),
        assists.automatic_clutch as u32 | ((assists.automatic_gears as u32) << 1),
        runtime.area.player.driving(),
        controls.steer,
        controls.throttle,
        controls.brake,
        controls.clutch,
        controls.handbrake,
    );
    let actions = game_actions();
    let held = game_keys();
    let player = game_player_keys();
    let mut frame = FrameInput {
        held,
        player_held: player as u8,
        player_pressed: (player >> 8) as u8,
        player_released: (player >> 16) as u8,
        pressed: game_pressed(),
        released: game_released(),
        inventory_held: game_inventory_keys() as u8 & 15,
        inventory_released: (game_inventory_keys() >> 4) as u8 & 15,
        mouse: [
            game_mouse_x() * reference.0.player.look_sensitivity[0],
            game_mouse_y() * reference.0.player.look_sensitivity[1],
        ],
        drive_mouse: [game_mouse_x(), -game_mouse_y()],
        wheel: game_wheel(),
        carry: crate::simulation::CarryDeltas::default(),
        typing: if drive.runtime.stream.computers.typing {
            let mut typing = crate::simulation::TypingInput {
                keys: game_typing_keys(),
                ..Default::default()
            };
            typing.len = game_typed_text(&mut typing.text).min(typing.text.len() as u32) as u8;
            typing
        } else {
            crate::simulation::TypingInput::default()
        },
        gamepad: crate::simulation::GamepadFrame::default(),
    };
    game_gamepad(&mut frame.gamepad.axes, &mut frame.gamepad.buttons);
    if requests.pending() {
        if cpu_profile::enabled() {
            game_cpu_clear_input();
        }
        drive.frame_input.clear();
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
    // The browser validates stored settings. A refused value keeps the current settings.
    if let Some(settings) = game_driving_settings()
        && let Ok(settings) =
            serde_json::from_str::<crate::simulation::input::DrivingSettings>(&settings)
        && settings.valid(&reference.0.driving)
    {
        drive.runtime.driving_input.settings = settings;
    }
    if !drive.ready || !drive.runtime.started || drive.runtime.paused {
        drive.input.clear();
        if cpu_profile::enabled() {
            game_cpu_clear_input();
        }
        drive.frame_input.clear();
        drive.runtime.clear_input();
        // The Controls screen shows the pad axes while the game is paused.
        drive.runtime.driving_input.preview(&frame.gamepad);
        return;
    }
    if actions & 22 != 0 {
        return;
    }
    if !drive.runtime.needs_output().dead
        && !drive.runtime.sleeping()
        && frame.pressed & keys::CAMERA != 0
        && drive.runtime.area.player.seat_vehicle().is_some()
    {
        drive.runtime.audio_motion_revision = drive.runtime.audio_motion_revision.wrapping_add(1);
        drive.camera = match drive.camera {
            CameraMode::Cockpit => CameraMode::Chase,
            CameraMode::Chase => CameraMode::Cockpit,
        };
    }
    drive.runtime.sleep_camera_allowed = drive.camera == CameraMode::Cockpit;
    drive.frame_input.merge(frame);
}
fn advance_physics(
    time: Res<Time<Real>>,
    reference: Res<RuntimeReference>,
    mut drive: ResMut<Drive>,
) {
    drive.physics_ms = 0.;
    drive.frame_steps = 0;
    drive.render_delta_seconds = 0.;
    if !drive.ready || !drive.runtime.started || drive.runtime.paused {
        return;
    }
    let blocked = drive.skip_elapsed
        && (drive.runtime.recovering
            || !drive.runtime.world_supported()
            || drive.runtime.sleeping()
            || drive.runtime.needs_output().dead);
    let Drive {
        frame_input,
        skip_elapsed,
        ..
    } = &mut *drive;
    let Some(raw) = frame_input.take_frame(skip_elapsed, blocked) else {
        if blocked {
            drive.input.clear();
            drive.runtime.clear_input();
        }
        return;
    };
    let started = game_now();
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::Physics);
    if let Some(audio) = &mut drive.runtime.audio {
        let [month, day] = local_calendar();
        audio.broadcasts.calendar(month, day);
    }
    let delta = time.delta_secs() * drive.runtime.clock_factors(&reference.0).time_scale;
    drive.runtime.area.conditions.update(delta, true);
    let view = drive.rendered;
    let frame = drive
        .runtime
        .advance_inventory_frame(&reference.0, raw, delta, view);
    frame.merge_fixed_input(&mut drive.input);
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
    let view = grass_mesh::GrassView {
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
            &mut runtime.vehicles,
            &mut runtime.powertrain,
            &runtime.flyers,
            runtime.simulation_seconds,
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
        .and_then(|_| {
            runtime
                .align_startup(&reference.0, local_calendar())
                .map(|_| ())
        })
        .and_then(|_| {
            if runtime.has_deferred_save_links() {
                runtime
                    .advance_saved_restore(&reference.0)
                    .map_err(|e| crate::reference::ReferenceError(e.to_string()))?;
            }
            runtime
                .sync_world_activity(&reference.0)
                .map_err(|e| crate::reference::ReferenceError(e.to_string()))?;
            runtime.advance_recovery()
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

pub(crate) fn sync_imported(
    frame: Res<SimFrameResource>,
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
    time: Res<Time<Real>>,
    mut caps: Query<
        (&mut CapVisual, &mut Transform),
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
    presentation::reference::sync(&frame, &mut query);
    presentation::reference::sync_items(&frame, &mut items);
    // Source DoMove advances by scaled Time.deltaTime.
    let state = frame.frame.state;
    let dt = if state.flags & StateBlock::PAUSED != 0 {
        0.
    } else {
        time.delta_secs() * state.time_scale
    };
    presentation::reference::sync_caps(&frame.frame, dt, &mut caps, &mut visibility);
}

/// Fills the frame after the simulation and world work of this Bevy frame.
fn fill_frame(
    reference: Res<RuntimeReference>,
    time: Res<Time<Real>>,
    mut drive: ResMut<Drive>,
    mut frame: ResMut<SimFrameResource>,
) {
    let drive = &mut *drive;
    let input = RenderInput {
        dt: drive.render_delta_seconds,
        real_dt: time.delta_secs(),
        listener: drive.listener,
    };
    drive.runtime.advance_render_frame(&reference.0, input);
    fill(drive, &reference.0, &mut frame);
}
/// Writes the runtime state into `frame` and applies its changes.
fn fill(drive: &mut Drive, reference: &ReferenceManifest, frame: &mut SimFrameResource) {
    let alpha = drive
        .clock
        .alpha(drive.runtime.clock_factors(reference).time_scale);
    drive.runtime.fill_frame(reference, alpha, &mut frame.frame);
    frame.apply_changes(|handle| drive.runtime.release_pose(handle));
    frame.index_particles();
}
fn active_camera(drive: &Drive, state: &StateBlock) -> Transform {
    let pose = |row: PoseRow| presentation::transform(row.pose());
    if state.sleep_camera.flags & PoseRow::ACTIVE != 0 {
        pose(state.sleep_camera)
    } else if drive.camera == CameraMode::Chase && state.seat_car.flags & PoseRow::ACTIVE != 0 {
        presentation::camera::pose(CameraMode::Chase, state.seat_car.pose())
    } else {
        presentation::transform(drive.rendered)
    }
}
fn update_camera(
    drive: Res<Drive>,
    sim: Res<SimFrameResource>,
    mut query: Query<(&mut Transform, &mut Projection), With<DriveCamera>>,
) {
    let state = &sim.frame.state;
    for (mut transform, mut projection) in &mut query {
        *transform = active_camera(&drive, state);
        presentation::camera_projection::sync_player_projection(
            &mut projection,
            true,
            state.fov_degrees,
        );
    }
}
fn publish_loading(
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    sim: Res<SimFrameResource>,
    visuals: Res<ReferenceVisuals>,
    materials: Res<presentation::materials::SourceMaterials>,
    lighting: Res<presentation::lighting::SourceLighting>,
    renderer: presentation::readiness::RendererReadinessAccess,
    plates: Res<presentation::plates::PlateTextStatus>,
    time: Res<Time<Real>>,
    mut last: Local<f64>,
    mut traced: Local<u8>,
) {
    let now = time.elapsed_secs_f64();
    let state = &sim.frame.state;
    let rendering = visuals.bound
        && materials.state == presentation::materials::RenderingState::Ready
        && lighting.state == presentation::lighting::RenderingState::Ready
        && renderer.graph_ready(presentation::readiness::RendererContentCheck {
            preparation: crate::render_readiness::ContentPreparation::Prepared(
                crate::render_readiness::ContentKey {
                    session: state.session,
                    selection_revision: state.selection_revision,
                },
            ),
            history_revision: state.motion_revision,
        })
        && plates.ready();
    if state.flags & StateBlock::GENERATION_READY != 0 && *traced & 1 == 0 {
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
        state.prepared_roads,
        reference.0.world.road.gen_at_start,
        rendering,
        state.flags & StateBlock::WORLD_READY != 0,
    );
    for (budget, (name, unit)) in sim.frame.budgets.iter().zip(BUDGET_LABELS) {
        game_loading_budget(name, budget.used, budget.limit, unit);
    }
}

fn update_hud(mut drive: ResMut<Drive>, mut sim: ResMut<SimFrameResource>) {
    let hud = sim.frame.hud;
    let state = sim.frame.state;
    let stamp = (hud.slots, hud.selected as u8, hud.swap_progress.to_bits());
    if drive.last_inventory != Some(stamp) {
        game_inventory(
            hud.selected,
            hud.slots[0],
            hud.slots[1],
            hud.slots[2],
            hud.slots[3],
            hud.swap_progress,
        );
        drive.last_inventory = Some(stamp);
    }
    game_survival(
        hud.hp,
        hud.food,
        hud.water,
        hud.bowel,
        hud.bladder,
        hud.opacity,
        hud.flags & HudBlock::DEAD != 0,
        hud.look_scale,
        hud.flags & HudBlock::MENU_DUE != 0,
    );
    game_sleep(
        hud.sleep_phase,
        hud.flags & HudBlock::SLEEP_ELIGIBLE != 0,
        hud.flags & HudBlock::SLEEP_HINT != 0,
        hud.sleep_charge,
        hud.cooldown_remaining,
        hud.sleep_progress,
    );
    if hud.flags & HudBlock::CAR == 0 {
        return;
    }
    let seated = state.flags & StateBlock::SEATED != 0;
    game_update(
        hud.speed_kmh,
        (drive.camera == CameraMode::Cockpit || !seated) as u32,
        hud.focus[0],
        hud.focus[1],
        hud.focus[2],
        hud.contacts,
        drive.frame_steps,
        hud.surfaces,
        hud.colliders,
        hud.absolute[0],
        hud.absolute[1],
        hud.absolute[2],
        hud.origin[0],
        hud.origin[1],
        hud.origin[2],
        hud.rebases,
        hud.trip_m,
        f64::from(state.prepared_roads),
        hud.fuel,
        state.flags & StateBlock::PAUSED != 0,
        state.flags & StateBlock::STARTED != 0,
        hud.flags & HudBlock::AUTOSAVE_SAFE != 0 && drive.input.held & keys::GRAB == 0,
        seated,
    );
    if hud.flags & HudBlock::RADIO != 0 {
        game_radio_feedback(
            hud.radio_volume,
            hud.radio_frequency,
            hud.flags & HudBlock::RADIO_AM != 0,
        );
    }
    let Some(f) = sim.feedback.take() else {
        return;
    };
    game_feedback(
        f.actions,
        f.context,
        &f.name,
        &f.notice,
        f.carrying,
        f.amount,
        f.capacity,
        f.has_name,
        f.no_text,
        &f.fluid_name,
        f.condition,
        &f.held_name,
        f.throw_charge,
        f.throw_active,
        f.rpm,
        f.ignition,
        f.running,
        f.starter,
        f.seated,
        f.handbrake,
        f.quantities[0],
        f.quantities[1],
        f.quantities[2],
        f.quantities[3],
        f.quantities[4],
        f.quantities[5],
        f.order,
        f.light_control,
        f.lmb_use,
        f.prompts.into(),
        f.computer.into(),
    );
}

fn publish_audio(
    mut drive: ResMut<Drive>,
    sim: Res<SimFrameResource>,
    visuals: Res<ReferenceVisuals>,
    globals: Query<(&GlobalTransform, &InheritedVisibility)>,
    camera: Query<&GlobalTransform, With<DriveCamera>>,
) {
    let camera = camera
        .single()
        .ok()
        .map(|c| c.to_scale_rotation_translation());
    drive.listener = camera.map(|(_, _, p)| p.to_array());
    for call in &sim.frame.audio {
        match *call {
            AudioCall::Begin {
                session,
                started,
                paused,
                seconds,
            } => game_audio_begin(session, started, paused, seconds),
            AudioCall::SurvivalMaster {
                gain,
                revision,
                seconds,
            } => game_audio_survival_master(gain, revision, seconds),
            AudioCall::OriginShift([x, y, z]) => game_audio_origin_shift(x, y, z),
            AudioCall::Listener => {
                if let Some((_, q, p)) = camera {
                    game_audio_listener(p.x, p.y, p.z, q.x, q.y, q.z, q.w);
                }
            }
            AudioCall::SurvivalEmitter {
                kind,
                position: [x, y, z],
            } => game_audio_survival_emitter(kind, x, y, z),
            AudioCall::Survival {
                kind,
                emitter,
                action,
                clip,
                gain,
                pitch,
                position: [x, y, z],
            } => game_audio_survival(kind, emitter, action, clip, gain, pitch, x, y, z),
            AudioCall::NodeEmitter { id, node, alive } => {
                if let Some((global, visibility)) = visuals
                    .nodes
                    .get(&node)
                    .and_then(|&entity| globals.get(entity).ok())
                {
                    let (_, q, p) = global.to_scale_rotation_translation();
                    game_audio_emitter(
                        id,
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
            AudioCall::Emitter {
                id,
                position: [x, y, z],
                rotation: [qx, qy, qz, qw],
                alive,
            } => game_audio_emitter(id, x, y, z, qx, qy, qz, qw, alive),
            AudioCall::Mixer { id, db } => game_audio_mixer(id, db),
            AudioCall::Route { id, mixer } => game_audio_route(id, mixer),
            AudioCall::Voice {
                id,
                playing,
                gain,
                pitch,
                looping,
                clip,
            } => game_audio_voice(id, playing, gain, pitch, looping, clip),
            AudioCall::Event {
                session,
                sequence,
                binding,
                emitter,
                position: [x, y, z],
                gain,
                pitch,
                seconds,
            } => game_audio_event(
                session, sequence, binding, emitter, x, y, z, gain, pitch, seconds,
            ),
            AudioCall::Radio { emitter, frame } => game_audio_radio(
                emitter,
                frame.power,
                frame.am,
                frame.frequency,
                frame.volume,
                frame.static_gain,
                frame.primary.clip.unwrap_or(0),
                frame.primary.offset_seconds,
                frame.primary.gain,
                frame.primary.next_clip.unwrap_or(0),
                frame.secondary.clip.unwrap_or(0),
                frame.secondary.offset_seconds,
                frame.secondary.gain,
                frame.secondary.next_clip.unwrap_or(0),
            ),
            AudioCall::End => game_audio_end(),
        }
    }
}
