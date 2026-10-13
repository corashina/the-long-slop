use super::*;
use crate::presentation::save::{PreparedPresentation, SaveAssetView, prepare_presentation};
use crate::save::{CameraSave, SaveError, SaveSnapshot, SourceHash, SourceStamp};
use serde::Deserialize;

#[derive(Resource)]
pub struct SaveRequests {
    source: SourceStamp,
    candidate: Option<Candidate>,
    #[cfg(feature = "save-fixture")]
    removed_asset: Option<(bevy::asset::AssetId<bevy::gltf::Gltf>, bevy::gltf::Gltf)>,
    #[cfg(feature = "save-fixture")]
    fixture_material: Option<bevy::asset::AssetId<presentation::materials::SourceMaterial>>,
}
struct Candidate {
    id: u32,
    runtime: GarageRuntime,
    camera: CameraMode,
    presentation: PreparedPresentation,
}
#[derive(Deserialize)]
struct Request {
    id: u32,
    kind: String,
    #[serde(default)]
    payload: serde_json::Value,
}
impl SaveRequests {
    pub fn pending(&self) -> bool {
        self.candidate.is_some()
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
            #[cfg(feature = "save-fixture")]
            removed_asset: None,
            #[cfg(feature = "save-fixture")]
            fixture_material: None,
        }
    }
}

/// Runs after lifecycle input, between fixed-step batches and before binding updates.
pub fn dispatch(world: &mut World) {
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
        let result = process(world, &mut requests, &request);
        match result {
            Ok(payload) => game_save_result(request.id, &request.kind, &payload, ""),
            Err(error) => game_save_result(request.id, &request.kind, "", &error.to_string()),
        }
    });
}
fn process(
    world: &mut World,
    requests: &mut SaveRequests,
    request: &Request,
) -> Result<String, SaveError> {
    #[cfg(feature = "save-fixture")]
    let removed = if request.kind == "Prepare" {
        requests.fixture_material.and_then(|id| {
            world
                .resource_mut::<Assets<presentation::materials::SourceMaterial>>()
                .remove(id)
                .map(|material| (id, material))
        })
    } else {
        None
    };
    let result = process_request(world, requests, request);
    #[cfg(feature = "save-fixture")]
    if let Some((id, material)) = removed {
        world
            .resource_mut::<Assets<presentation::materials::SourceMaterial>>()
            .insert(id, material)
            .map_err(|error| SaveError::ReconstructionFailure(error.to_string()))?;
    }
    result
}
fn process_request(
    world: &mut World,
    requests: &mut SaveRequests,
    request: &Request,
) -> Result<String, SaveError> {
    let reference = world.resource::<RuntimeReference>().0.clone();
    let invalid = |message: &str| SaveError::ReconstructionFailure(message.into());
    match request.kind.as_str() {
        #[cfg(feature = "save-fixture")]
        "FixtureRemoveAsset" => {
            if requests.removed_asset.is_some() {
                return Err(invalid("A fixture asset is already removed"));
            }
            let visuals = world.resource::<ReferenceVisuals>();
            let (path, handle) = visuals
                .handles
                .first()
                .ok_or_else(|| invalid("No fixture asset"))?;
            let (path, id) = (path.clone(), handle.id());
            let asset = world
                .resource_mut::<Assets<bevy::gltf::Gltf>>()
                .remove(id)
                .ok_or_else(|| invalid("Fixture asset is missing"))?;
            requests.removed_asset = Some((id, asset));
            Ok(serde_json::to_string(&path).unwrap())
        }
        #[cfg(feature = "save-fixture")]
        "FixtureFailMaterialPreparation" => {
            if requests.fixture_material.is_some() {
                return Err(invalid("A fixture material fault is already set"));
            }
            requests.fixture_material = Some(world.resource::<WorldVisuals>().fixture_material());
            Ok(serde_json::to_string("generated chunk terrain material").unwrap())
        }
        #[cfg(feature = "save-fixture")]
        "FixtureRestoreAsset" => {
            requests.fixture_material = None;
            if let Some((id, asset)) = requests.removed_asset.take() {
                world
                    .resource_mut::<Assets<bevy::gltf::Gltf>>()
                    .insert(id, asset)
                    .map_err(|error| invalid(&error.to_string()))?;
            }
            Ok(String::new())
        }

        #[cfg(feature = "save-fixture")]
        "FixtureSetupFleet"
        | "FixtureSetupDriving"
        | "FixtureDetachAndCarry"
        | "FixtureTransferFluid"
        | "FixtureSetupCycle"
        | "FixtureInspect" => super::save_fixture::process(world, &request.kind),
        "Discard" => {
            if requests
                .candidate
                .as_ref()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                requests
                    .candidate
                    .take()
                    .unwrap()
                    .presentation
                    .discard(world);
            }
            Ok(String::new())
        }
        "Commit" => {
            if !requests
                .candidate
                .as_ref()
                .is_some_and(|candidate| Some(candidate.id as u64) == request.payload.as_u64())
            {
                return Err(invalid("Prepared session is missing"));
            }
            let candidate = requests.candidate.take().unwrap();
            // The prepared commands and stable bindings make this boundary infallible.
            candidate.presentation.commit(world);
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
            if requests.candidate.is_some() {
                return Err(invalid("A prepared session is pending"));
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
            let presentation = prepare_presentation(&reference, &runtime, SaveAssetView { world })?;
            requests.candidate = Some(Candidate {
                id: request.id,
                runtime,
                camera,
                presentation,
            });
            Ok(request.id.to_string())
        }
        _ => Err(invalid("Unknown save request")),
    }
}
