use super::*;
use crate::presentation::save::PreparedPresentation;
use crate::save::{CameraSave, SaveError, SaveSnapshot, SourceHash, SourceStamp};
use serde::Deserialize;

#[derive(Resource)]
pub struct SaveRequests {
    source: SourceStamp,
    candidate: Option<Candidate>,
    /// A new game that keeps the loaded world, which nobody has started yet.
    fresh: Option<u32>,
}
pub(crate) struct Candidate {
    id: u32,
    runtime: GarageRuntime,
    camera: CameraMode,
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
    pub(crate) fn hidden(&self) -> Option<&PreparedPresentation> {
        self.candidate
            .as_ref()
            .and_then(|c| c.presentation.as_ref())
    }
    pub(crate) fn freezes_world(&self) -> bool {
        self.hidden().is_some()
    }
    pub fn pending(&self) -> bool {
        self.candidate.is_some() || self.fresh.is_some()
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
            candidate: None,
            fresh: None,
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
fn process_request(
    world: &mut World,
    requests: &mut SaveRequests,
    request: &Request,
) -> Result<String, SaveError> {
    let reference = world.resource::<RuntimeReference>().0.clone();
    let invalid = |message: &str| SaveError::ReconstructionFailure(message.into());
    match request.kind.as_str() {
        "Discard" => {
            if requests
                .fresh
                .is_some_and(|id| Some(u64::from(id)) == request.payload.as_u64())
            {
                requests.fresh = None;
                world.resource_mut::<Drive>().runtime.started = false;
                return Ok(String::new());
            }
            if requests
                .candidate
                .as_ref()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                let candidate = requests.candidate.as_mut().unwrap();
                candidate.canceled = true;
                if let Some(presentation) = &mut candidate.presentation {
                    presentation.cancel();
                }
            }
            Ok(String::new())
        }
        "Commit" => {
            if requests
                .fresh
                .is_some_and(|id| Some(u64::from(id)) == request.payload.as_u64())
            {
                requests.fresh = None;
                return Ok(String::new());
            }
            if !requests
                .candidate
                .as_ref()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                return Err(invalid("Prepared session is missing"));
            }
            if !requests
                .candidate
                .as_ref()
                .is_some_and(|c| c.ready && !c.canceled)
            {
                return Err(invalid("Prepared renderer is not ready"));
            }
            let candidate = requests.candidate.as_ref().unwrap();
            candidate.presentation.as_ref().unwrap().validate_commit(
                &reference,
                &candidate.runtime,
                world,
            )?;
            let mut candidate = requests.candidate.take().unwrap();
            candidate
                .runtime
                .stream
                .set_overlap(crate::world::WorldMetrics::default())?;
            // The prepared commands and stable bindings make this boundary infallible.
            world.resource_scope(
                |world, mut food: Mut<crate::presentation::consumables::ConsumableVisuals>| {
                    food.reset(world)
                },
            );
            world.resource_scope(|world, mut drive: Mut<Drive>| {
                candidate
                    .presentation
                    .take()
                    .unwrap()
                    .commit(&mut drive.runtime, world);
            });
            let mut drive = world.resource_mut::<Drive>();
            let session = drive
                .runtime
                .audio
                .as_ref()
                .map_or(1, |audio| audio.session.wrapping_add(1));
            drive.runtime = candidate.runtime;
            let runtime = &mut drive.runtime;
            if let Some(audio) = &mut runtime.audio {
                audio.reset(session);
                audio.sync_lamps(&reference.lighting, &runtime.lighting);
            }
            drive.camera = candidate.camera;
            drive.current = drive.runtime.area.player.view_pose(&drive.runtime.physics);
            drive.previous = drive.current;
            drive.rendered = drive.current;
            drive.input.clear();
            drive.clock = StepClock::default();
            drive.frame_steps = 0;
            drive.render_delta_seconds = 0.;
            drive.ready = false;
            drive.runtime.paused = true;
            drive.skip_elapsed = true;
            drive.last_feedback = None;
            for (index, vehicle) in reference.vehicles.iter().enumerate() {
                let sample = super::instrument_sample(&reference, &drive.runtime, index);
                drive.instruments[index].initialize_from_sample(&vehicle.instruments, sample);
            }
            drop(drive);
            crate::browser::game_loading();
            crate::presentation::view_frame::restart_scene(world);
            crate::presentation::mirrors::reset_mirrors(world);
            crate::presentation::plates::reset_plate_text(world);
            Ok(String::new())
        }
        "Capture" | "Prepare" | "PrepareNewGame" => {
            let drive = world.resource::<Drive>();
            if !drive.ready {
                return Err(invalid("The garage is not ready"));
            }
            if drive.runtime.started && !drive.runtime.paused {
                return Err(invalid("Pause the game first"));
            }
            if request.kind == "Capture" {
                let mut drive = world.resource_mut::<Drive>();
                super::clear_input(&mut drive);
                let snapshot = drive.runtime.capture_save(
                    &reference,
                    &requests.source,
                    if drive.camera == CameraMode::Chase {
                        CameraSave::Chase
                    } else {
                        CameraSave::FirstPerson
                    },
                    request
                        .payload
                        .as_str()
                        .ok_or_else(|| invalid("Missing capture date"))?,
                )?;
                return serde_json::to_string(&snapshot)
                    .map_err(|error| SaveError::CorruptData(error.to_string()));
            }
            if requests.candidate.is_some() || requests.fresh.is_some() {
                return Err(invalid("A prepared session is pending"));
            }
            // The loaded world is already a new game until the player starts it.
            if request.kind == "PrepareNewGame" && !world.resource::<Drive>().runtime.started {
                let mut drive = world.resource_mut::<Drive>();
                drive.runtime.started = true;
                drive.runtime.paused = true;
                requests.fresh = Some(request.id);
                game_save_result(request.id, &request.kind, &request.id.to_string(), "");
                return Ok(String::new());
            }
            let (mut runtime, camera) = if request.kind == "Prepare" {
                let saved: SaveSnapshot = serde_json::from_value(request.payload.clone())
                    .map_err(|error| SaveError::CorruptData(error.to_string()))?;
                let runtime = GarageRuntime::from_save(&reference, &requests.source, &saved)?;
                (
                    runtime,
                    if saved.view.camera == CameraSave::Chase {
                        CameraMode::Chase
                    } else {
                        CameraMode::Cockpit
                    },
                )
            } else {
                (GarageRuntime::new(&reference)?, CameraMode::default())
            };
            runtime.started = true;
            runtime.paused = true;

            requests.candidate = Some(Candidate {
                id: request.id,
                runtime,
                camera,
                presentation: None,
                kind: request.kind.clone(),
                ready: false,
                canceled: false,
            });
            Ok(request.id.to_string())
        }
        _ => Err(invalid("Unknown save request")),
    }
}

fn advance(world: &mut World) {
    world.resource_scope(|world, mut requests: Mut<SaveRequests>| {
        let Some(mut candidate) = requests.candidate.take() else {
            return;
        };
        if candidate.canceled {
            if candidate
                .presentation
                .as_mut()
                .is_none_or(|p| p.drained(world))
            {
                if let Some(p) = candidate.presentation.take() {
                    p.release(&mut candidate.runtime, world);
                }
                game_save_result(candidate.id, &candidate.kind, "", "Save operation canceled");
                return;
            }
            requests.candidate = Some(candidate);
            return;
        }
        if candidate.ready {
            requests.candidate = Some(candidate);
            return;
        }
        let reference = world.resource::<RuntimeReference>().0.clone();
        let result = (|| -> Result<bool, SaveError> {
            if candidate.presentation.is_none() {
                if !PreparedPresentation::can_start(world) {
                    return Ok(false);
                }
                candidate.presentation = Some(PreparedPresentation::new(
                    &reference,
                    &mut candidate.runtime,
                    world,
                )?);
            }
            candidate.presentation.as_mut().unwrap().advance(
                &reference,
                &mut candidate.runtime,
                candidate.camera,
                world,
            )
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
        requests.candidate = Some(candidate);
    });
}
