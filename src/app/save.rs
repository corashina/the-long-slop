use super::*;
use crate::presentation::save::PreparedPresentation;
use crate::save::fresh_permit::FreshSessionPermit;
use crate::save::{CameraSave, SaveError, SaveSnapshot, SourceHash, SourceStamp};
use serde::{Deserialize, Serialize};
use simulation::candidate::CandidateDriver;

#[derive(Resource)]
pub struct SaveRequests {
    source: SourceStamp,
    pending: Option<PendingPreparation>,
}
enum PendingPreparation {
    Fresh(FreshSessionPermit),
    Replacement(Candidate),
}
pub(crate) struct Candidate {
    id: u32,
    driver: CandidateDriver,
    pub(crate) presentation: Option<PreparedPresentation>,
    kind: String,
    ready: bool,
    canceled: bool,
}
#[derive(Deserialize)]
struct Request {
    id: u32,
    kind: String,
    #[serde(default)]
    payload: serde_json::Value,
}
impl SaveRequests {
    fn candidate(&self) -> Option<&Candidate> {
        match self.pending.as_ref()? {
            PendingPreparation::Replacement(c) => Some(c),
            _ => None,
        }
    }
    fn candidate_mut(&mut self) -> Option<&mut Candidate> {
        match self.pending.as_mut()? {
            PendingPreparation::Replacement(c) => Some(c),
            _ => None,
        }
    }
    fn take_candidate(&mut self) -> Option<Candidate> {
        if matches!(self.pending, Some(PendingPreparation::Replacement(_))) {
            let Some(PendingPreparation::Replacement(c)) = self.pending.take() else {
                unreachable!()
            };
            Some(c)
        } else {
            None
        }
    }

    pub(crate) fn hidden(&self) -> Option<&PreparedPresentation> {
        self.candidate().and_then(|c| c.presentation.as_ref())
    }
    pub(crate) fn freezes_world(&self) -> bool {
        self.hidden().is_some()
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn new(reference: &ReferenceManifest, hash: String) -> Self {
        Self {
            source: SourceStamp {
                app_id: reference.app_id,
                build_id: reference.build_id,
                manifest_sha256: hash,
                source_hashes: reference
                    .source_hashes
                    .iter()
                    .map(|s| SourceHash {
                        file: s.file.clone(),
                        sha256: s.sha256.clone(),
                    })
                    .collect(),
            },
            pending: None,
        }
    }
}

/// Runs after lifecycle input, between fixed-step batches and before binding updates.
pub fn dispatch(world: &mut World) {
    advance(world);
    if !game_has_save_request() {
        return;
    }
    let text = game_save_request();
    let request: Request = match serde_json::from_str(&text) {
        Ok(request) => request,
        Err(error) => {
            game_save_result(0, "Invalid", "", &error.to_string());
            return;
        }
    };
    world.resource_scope(|world, mut requests: Mut<SaveRequests>| {
        let result = process_request(world, &mut requests, &request);
        match result {
            Ok(payload) => {
                if request.kind != "Prepare" && request.kind != "PrepareNewGame" {
                    game_save_result(request.id, &request.kind, &payload, "");
                }
            }
            Err(error) => game_save_result(request.id, &request.kind, "", &error.to_string()),
        }
    });
}
#[derive(Serialize)]
struct CaptureResult<'a> {
    save: &'a SaveSnapshot,
    summary: CaptureSummary,
}
#[derive(Serialize)]
struct CaptureSummary {
    seed: i32,
    driven_km: f64,
    build_id: u32,
}
fn process_request(
    world: &mut World,
    requests: &mut SaveRequests,
    request: &Request,
) -> Result<String, SaveError> {
    let reference = world.resource::<RuntimeReference>().0.clone();
    let invalid = |message: &str| SaveError::ReconstructionFailure(message.into());
    match request.kind.as_str() {
        "Discard" => {
            if let Some(PendingPreparation::Fresh(permit)) = requests.pending.as_ref() {
                if Some(u64::from(permit.request_id)) == request.payload.as_u64() {
                    requests.pending = None;
                }
                return Ok(String::new());
            }
            if requests
                .candidate()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                let candidate = requests.candidate_mut().unwrap();
                candidate.canceled = true;
                if let Some(presentation) = &mut candidate.presentation {
                    presentation.cancel();
                }
            }
            Ok(String::new())
        }
        "Commit" => {
            let _profile = cpu_profile::Timer::new(cpu_profile::Stage::SaveCommit);
            if let Some(PendingPreparation::Fresh(permit)) = requests.pending.as_ref() {
                let permit = *permit;
                let generation = world
                    .resource::<crate::presentation::view_frame::SourceSceneEpoch>()
                    .0;
                let id = request
                    .payload
                    .as_u64()
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or_else(|| invalid("Prepared session is stale"))?;
                let mut drive = world.resource_mut::<Drive>();
                if !drive.ready {
                    return Err(invalid("The garage is not ready"));
                }
                permit.commit(id, &mut drive.sim.runtime, generation)?;
                drive.sim.clear_input();
                drive.sim.frame_steps = 0;
                drive.sim.render_delta_seconds = 0.;
                requests.pending = None;
                return Ok(String::new());
            }
            if !requests
                .candidate()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                return Err(invalid("Prepared session is missing"));
            }
            if !requests.candidate().is_some_and(|c| c.ready && !c.canceled) {
                return Err(invalid("Prepared renderer is not ready"));
            }
            let candidate = requests.candidate().unwrap();
            candidate
                .presentation
                .as_ref()
                .unwrap()
                .validate_commit(&reference, world)?;
            let mut candidate = requests.take_candidate().unwrap();
            // The prepared commands and stable bindings make this boundary infallible.
            world.resource_scope(
                |world, mut food: Mut<crate::presentation::consumables::ConsumableVisuals>| {
                    food.reset(world)
                },
            );
            world.resource_scope(
                |world, mut items: Mut<crate::presentation::spawned::SpawnedVisuals>| {
                    items.reset(world)
                },
            );
            world.resource_scope(
                |world, mut sky: Mut<crate::presentation::sky_traffic::SkyTrafficVisuals>| {
                    sky.reset(world)
                },
            );
            candidate.presentation.take().unwrap().commit(world);
            let mut drive = world.resource_mut::<Drive>();
            let session = drive
                .sim
                .runtime
                .audio
                .as_ref()
                .map_or(1, |audio| audio.session.wrapping_add(1));
            // Assignment drops the old stream and terminates its road actor first.
            let settings = drive.sim.runtime.driving_input.settings;
            drive.sim.runtime = candidate.driver.runtime;
            drive.sim.runtime.driving_input.settings = settings;
            drive
                .sim
                .runtime
                .stream
                .set_overlap(crate::world::WorldMetrics::default())
                .expect("retired world overlap only removes charges");
            let runtime = &mut drive.sim.runtime;
            if let Some(audio) = &mut runtime.audio {
                audio.reset(session);
                audio.sync_lamps(&reference.lighting, &runtime.lighting);
            }
            drive.sim.restart(candidate.driver.camera);
            drive.ready = false;
            drive.sim.runtime.advance_instruments(&reference, 0., true);
            // The new runtime starts a new frame stream. The candidate frames consumed its sent state.
            drive.sim.runtime.reset_frame_outputs();
            let mut sim = SimFrameResource::default();
            super::fill(&mut drive, &reference, &mut sim);
            let state = sim.frame.state;
            let camera = super::frame_camera(&state, drive.view);
            let (far, fov) = (state.far_plane, state.fov_degrees);
            drop(drive);
            world.insert_resource(sim);
            let mut cameras = world.query::<(
                &mut Transform,
                &mut Projection,
                Option<&DriveCamera>,
                Option<&crate::presentation::frame_graph::SourceSkyCamera>,
            )>();
            for (mut transform, mut projection, main, sky) in cameras.iter_mut(world) {
                if main.is_some() {
                    *transform = camera;
                    if let Projection::Perspective(perspective) = &mut *projection {
                        perspective.far = far;
                    }
                } else if sky.is_some() {
                    *transform = Transform::from_rotation(camera.rotation);
                }
                crate::presentation::camera_projection::sync_player_projection(
                    &mut projection,
                    main.is_some() || sky.is_some(),
                    fov,
                );
            }
            crate::browser::game_loading();
            crate::presentation::view_frame::restart_scene(world);
            Ok(String::new())
        }
        "Capture" | "Prepare" | "PrepareNewGame" => {
            let _profile = cpu_profile::Timer::new(if request.kind == "Prepare" {
                cpu_profile::Stage::LoadPrepare
            } else {
                cpu_profile::Stage::SavePrepare
            });
            let drive = world.resource::<Drive>();
            if !drive.ready {
                return Err(invalid("The garage is not ready"));
            }
            let autosave = request.kind == "Capture"
                && request
                    .payload
                    .get("autosave")
                    .and_then(serde_json::Value::as_bool)
                    .ok_or_else(|| invalid("Missing autosave flag"))?;
            if !autosave && drive.sim.runtime.started && !drive.sim.runtime.paused {
                return Err(invalid("Pause the game first"));
            }
            if request.kind == "Capture" {
                let mut drive = world.resource_mut::<Drive>();
                if !autosave {
                    drive.sim.clear_input();
                }
                let snapshot = drive.sim.runtime.capture_save(
                    &reference,
                    &requests.source,
                    if drive.sim.camera == CameraMode::Chase {
                        CameraSave::Chase
                    } else {
                        CameraSave::FirstPerson
                    },
                    request
                        .payload
                        .get("date")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| invalid("Missing capture date"))?,
                )?;
                return serde_json::to_string(&CaptureResult {
                    save: &snapshot,
                    summary: CaptureSummary {
                        seed: snapshot.world.seed,
                        driven_km: snapshot.progress.driven_km,
                        build_id: snapshot.source.build_id,
                    },
                })
                .map_err(|error| SaveError::CorruptData(error.to_string()));
            }
            if requests.pending.is_some() {
                return Err(invalid("A prepared session is pending"));
            }
            let draw_distance = request
                .payload
                .get("draw_distance")
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| invalid("Missing draw distance"))?
                as f32;
            let mode = super::game_mode(&reference);
            if request.kind == "PrepareNewGame"
                && !drive.sim.runtime.started
                && drive.sim.runtime.stream.mode() == mode
                && drive
                    .sim
                    .runtime
                    .distances
                    .matches(&reference.world.draw_distance, draw_distance)
            {
                let permit = FreshSessionPermit::prepare(
                    request.id,
                    &drive.sim.runtime,
                    world
                        .resource::<crate::presentation::view_frame::SourceSceneEpoch>()
                        .0,
                )?;
                requests.pending = Some(PendingPreparation::Fresh(permit));
                game_save_result(request.id, &request.kind, &request.id.to_string(), "");
                return Ok(request.id.to_string());
            }
            let (mut runtime, camera) = if request.kind == "Prepare" {
                let saved = request
                    .payload
                    .get("save")
                    .ok_or_else(|| invalid("Missing save"))?;
                let saved = SaveSnapshot::deserialize(saved)
                    .map_err(|error| SaveError::CorruptData(error.to_string()))?;
                let runtime = GarageRuntime::from_save(
                    &reference,
                    &requests.source,
                    &saved,
                    mode,
                    draw_distance,
                )?;
                (
                    runtime,
                    if saved.view.camera == CameraSave::Chase {
                        CameraMode::Chase
                    } else {
                        CameraMode::Cockpit
                    },
                )
            } else {
                (
                    GarageRuntime::new(&reference, mode, draw_distance)?,
                    CameraMode::default(),
                )
            };
            runtime.started = true;
            runtime.paused = true;

            requests.pending = Some(PendingPreparation::Replacement(Candidate {
                id: request.id,
                driver: CandidateDriver::new(runtime, camera),
                presentation: None,
                kind: request.kind.clone(),
                ready: false,
                canceled: false,
            }));
            Ok(request.id.to_string())
        }
        _ => Err(invalid("Unknown save request")),
    }
}

fn advance(world: &mut World) {
    world.resource_scope(|world, mut requests: Mut<SaveRequests>| {
        let Some(mut candidate) = requests.take_candidate() else {
            return;
        };
        if candidate.canceled {
            if candidate
                .presentation
                .as_mut()
                .is_none_or(|p| p.drained(world))
            {
                if let Some(p) = candidate.presentation.take() {
                    p.release(world);
                }
                game_save_result(candidate.id, &candidate.kind, "", "Save operation canceled");
                return;
            }
            requests.pending = Some(PendingPreparation::Replacement(candidate));
            return;
        }
        if candidate.ready {
            requests.pending = Some(PendingPreparation::Replacement(candidate));
            return;
        }
        let reference = world.resource::<RuntimeReference>().0.clone();
        let _profile = cpu_profile::Timer::new(cpu_profile::Stage::LoadPrepare);
        let result = (|| -> Result<bool, SaveError> {
            if candidate.presentation.is_none() {
                if !PreparedPresentation::can_start(world) {
                    return Ok(false);
                }
                candidate.presentation = Some(PreparedPresentation::new(
                    &reference,
                    &candidate.driver.runtime.stream,
                    world,
                )?);
            }
            let presentation = candidate.presentation.as_mut().unwrap();
            let driver = &mut candidate.driver;
            driver.accept(presentation.take_command());
            let active = world.resource::<Drive>().sim.runtime.stream.metrics();
            driver.advance(&reference, active, crate::browser::local_calendar());
            driver.fill(&reference, &mut presentation.frame.frame);
            presentation
                .frame
                .apply_changes(|handle| driver.release_pose(handle));
            presentation.advance(&reference, &driver.runtime.stream, world)
        })();
        match result {
            Ok(true) => {
                candidate.ready = true;
                game_save_result(candidate.id, &candidate.kind, &candidate.id.to_string(), "");
            }
            Ok(false) => {}
            Err(error) => {
                game_save_result(candidate.id, &candidate.kind, "", &error.to_string());
                candidate.canceled = true;
                if let Some(p) = &mut candidate.presentation {
                    p.cancel();
                }
            }
        }
        requests.pending = Some(PendingPreparation::Replacement(candidate));
    });
}
