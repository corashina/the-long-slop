use super::*;
use crate::presentation::save::PreparedPresentation;
use crate::save::{SourceHash, SourceStamp};
use serde::Deserialize;
use serde_json::value::RawValue;
use simulation::boundary::{CandidateCommand, Change, SaveCommand};

/// The save request that waits for the simulation or the renderer.
enum Pending {
    /// The simulation runs a prepare request.
    Waiting { id: u32, kind: String },
    /// A new game in the current runtime waits for the commit.
    Fresh(u32),
    /// The renderer prepares the scene of the simulation candidate offscreen.
    Candidate(Box<Candidate>),
}
pub(crate) struct Candidate {
    id: u32,
    kind: String,
    pub(crate) presentation: Option<PreparedPresentation>,
    ready: bool,
    canceled: bool,
    /// The last command carried a candidate step.
    stepped: bool,
    /// The simulation has the commit. A Discard comes too late.
    committing: bool,
}
#[derive(Resource, Default)]
pub struct SaveRequests {
    pending: Option<Pending>,
    /// Save input of the next command.
    commands: Vec<SaveCommand>,
}
#[derive(Deserialize)]
struct Request {
    id: u32,
    kind: String,
    #[serde(default)]
    payload: Option<Box<RawValue>>,
}
impl Request {
    /// The prepared request that a Commit or Discard names.
    fn prepared(&self) -> Option<u32> {
        let payload = self.payload.as_ref()?;
        serde_json::from_str(payload.get()).ok()
    }
}

/// The save stamp of this build.
pub fn source_stamp(reference: &ReferenceManifest, hash: String) -> SourceStamp {
    SourceStamp {
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
    }
}

impl SaveRequests {
    fn candidate(&self) -> Option<&Candidate> {
        match self.pending.as_ref()? {
            Pending::Candidate(c) => Some(c),
            _ => None,
        }
    }
    fn candidate_mut(&mut self) -> Option<&mut Candidate> {
        match self.pending.as_mut()? {
            Pending::Candidate(c) => Some(c),
            _ => None,
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
    /// Save requests of the next command.
    pub(crate) fn take_commands(&mut self) -> Vec<SaveCommand> {
        std::mem::take(&mut self.commands)
    }
    /// Renderer input of the next candidate step. `None` while the scene is ready or not started.
    pub(crate) fn candidate_command(&mut self) -> Option<CandidateCommand> {
        let candidate = self.candidate_mut()?;
        candidate.stepped = false;
        if candidate.ready || candidate.canceled {
            return None;
        }
        let command = candidate.presentation.as_mut()?.take_command();
        candidate.stepped = true;
        Some(command)
    }
}

/// Runs after the frame fill: applies the save results, steps the hidden scene, reads the next request.
pub fn dispatch(world: &mut World) {
    world.resource_scope(|world, mut requests: Mut<SaveRequests>| {
        results(world, &mut requests);
        advance(world, &mut requests);
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
        if let Err(error) = request_input(world, &mut requests, &request, text) {
            game_save_result(request.id, &request.kind, "", &error.to_string());
        }
    });
}

/// Answers Discard and checks the hidden scene for Commit. The simulation runs the rest.
fn request_input(
    world: &mut World,
    requests: &mut SaveRequests,
    request: &Request,
    json: String,
) -> Result<(), crate::save::SaveError> {
    let invalid = |message: &str| crate::save::SaveError::ReconstructionFailure(message.into());
    let prepared = request.prepared();
    match request.kind.as_str() {
        "Discard" => {
            match &mut requests.pending {
                Some(Pending::Fresh(id) | Pending::Waiting { id, .. }) if Some(*id) == prepared => {
                    requests.commands.push(SaveCommand::Cancel(*id));
                    requests.pending = None;
                }
                Some(Pending::Candidate(candidate))
                    if Some(candidate.id) == prepared && !candidate.committing =>
                {
                    requests.commands.push(SaveCommand::Cancel(candidate.id));
                    candidate.canceled = true;
                    if let Some(presentation) = &mut candidate.presentation {
                        presentation.cancel();
                    }
                }
                _ => {}
            }
            game_save_result(request.id, &request.kind, "", "");
            return Ok(());
        }
        "Commit" => {
            if let Some(candidate) = requests.candidate_mut() {
                if Some(candidate.id) != prepared {
                    return Err(invalid("Prepared session is missing"));
                }
                if !candidate.ready || candidate.canceled {
                    return Err(invalid("Prepared renderer is not ready"));
                }
                let reference = world.resource::<RuntimeReference>().0.clone();
                candidate
                    .presentation
                    .as_ref()
                    .unwrap()
                    .validate_commit(&reference, world)?;
                candidate.committing = true;
            }
        }
        "Prepare" | "PrepareNewGame" => {
            if requests.pending.is_some() {
                return Err(invalid("A prepared session is pending"));
            }
            requests.pending = Some(Pending::Waiting {
                id: request.id,
                kind: request.kind.clone(),
            });
        }
        _ => {}
    }
    let reference = &world.resource::<RuntimeReference>().0;
    requests.commands.push(SaveCommand::Request {
        json,
        mode: super::game_mode(reference),
        epoch: world
            .resource::<crate::presentation::view_frame::SourceSceneEpoch>()
            .0,
    });
    Ok(())
}

/// Sends the simulation answers to the browser and follows the prepared session.
fn results(world: &mut World, requests: &mut SaveRequests) {
    let mut restarted = false;
    for change in &world.resource::<SimFrameResource>().frame.changes {
        match change {
            Change::SaveResult {
                id,
                kind,
                payload,
                error,
            } => {
                game_save_result(*id, kind, payload, error);
                match requests.pending {
                    Some(Pending::Waiting { id: waiting, .. }) if waiting == *id => {
                        // Only a fresh session answers a prepare request at once.
                        requests.pending = error.is_empty().then_some(Pending::Fresh(*id));
                    }
                    Some(Pending::Fresh(_)) if kind == "Commit" && error.is_empty() => {
                        requests.pending = None;
                    }
                    _ => {}
                }
            }
            Change::Restarted => restarted = true,
            _ => {}
        }
    }
    let flags = world.resource::<SimFrameResource>().frame.state.flags;
    if flags & StateBlock::CANDIDATE != 0
        && let Some(Pending::Waiting { id, kind }) = &requests.pending
    {
        requests.pending = Some(Pending::Candidate(Box::new(Candidate {
            id: *id,
            kind: kind.clone(),
            presentation: None,
            ready: false,
            canceled: false,
            stepped: false,
            committing: false,
        })));
    }
    if restarted {
        let Some(Pending::Candidate(candidate)) = requests.pending.take() else {
            unreachable!("the simulation restarts only with a prepared candidate")
        };
        commit(world, *candidate);
    }
}

/// Shows the prepared scene. The simulation already runs the candidate runtime.
fn commit(world: &mut World, mut candidate: Candidate) {
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::SaveCommit);
    world.resource_scope(
        |world, mut food: Mut<crate::presentation::consumables::ConsumableVisuals>| {
            food.reset(world)
        },
    );
    world.resource_scope(
        |world, mut items: Mut<crate::presentation::spawned::SpawnedVisuals>| items.reset(world),
    );
    world.resource_scope(
        |world, mut sky: Mut<crate::presentation::sky_traffic::SkyTrafficVisuals>| sky.reset(world),
    );
    // The prepared commands and stable bindings make this boundary infallible.
    candidate.presentation.take().unwrap().commit(world);
    let view = {
        let mut drive = world.resource_mut::<Drive>();
        drive.ready = false;
        drive.view
    };
    let state = world.resource::<SimFrameResource>().frame.state;
    let camera = super::frame_camera(&state, view);
    let (far, fov) = (state.far_plane, state.fov_degrees);
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
}

/// Prepares the hidden scene from the candidate frame of the last step.
fn advance(world: &mut World, requests: &mut SaveRequests) {
    let Some(candidate) = requests.candidate_mut() else {
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
            requests.pending = None;
        }
        return;
    }
    if candidate.ready {
        return;
    }
    let reference = world.resource::<RuntimeReference>().0.clone();
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::LoadPrepare);
    let result = (|| -> Result<bool, crate::save::SaveError> {
        let Some(presentation) = &mut candidate.presentation else {
            if PreparedPresentation::can_start(world) {
                world.resource_scope(|world, drive: Mut<Drive>| {
                    let stream = drive.sim.candidate_stream().expect("simulation candidate");
                    candidate.presentation =
                        Some(PreparedPresentation::new(&reference, stream, world)?);
                    Ok::<_, crate::save::SaveError>(())
                })?;
            }
            return Ok(false);
        };
        if !candidate.stepped {
            return Ok(false);
        }
        world.resource_scope(|world, mut sim: Mut<SimFrameResource>| {
            world.resource_scope(|world, mut drive: Mut<Drive>| {
                // The simulation fills the candidate frame after each candidate step.
                let stepped = sim.frame.state.flags & StateBlock::CANDIDATE != 0;
                let Some(frame) = sim.frame.candidate.as_mut().filter(|_| stepped) else {
                    return Ok(false);
                };
                std::mem::swap(&mut **frame, &mut presentation.frame.frame);
                presentation
                    .frame
                    .apply_changes(|handle| drive.sim.release_candidate_pose(handle));
                let stream = drive.sim.candidate_stream().expect("simulation candidate");
                presentation.advance(&reference, stream, world)
            })
        })
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
            let id = candidate.id;
            requests.commands.push(SaveCommand::Cancel(id));
        }
    }
}
