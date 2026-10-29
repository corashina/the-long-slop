use super::*;
use crate::presentation::save::PreparedPresentation;
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
    /// The newest candidate frame and its pose index.
    frame: SimFrameResource,
    /// The candidate world copy until the hidden scene takes it.
    mirror: Option<Box<crate::world::RendererWorld>>,
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
                candidate.presentation.as_ref().unwrap().validate_commit(
                    &reference,
                    &candidate.frame,
                    world,
                )?;
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
        // The candidate copy starts empty and shares the fixed lookups.
        let mirror = world
            .resource::<crate::presentation::world::runtime::SourcePresentationAllocation>()
            .world
            .as_ref()
            .map_or_else(
                || {
                    crate::world::RendererWorld::new(
                        world.resource::<RuntimeReference>().0.world.clone(),
                    )
                },
                |w| w.fork(),
            );
        requests.pending = Some(Pending::Candidate(Box::new(Candidate {
            id: *id,
            kind: kind.clone(),
            frame: SimFrameResource::default(),
            mirror: Some(Box::new(mirror)),
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
    world.resource_scope(
        |world, mut creatures: Mut<crate::presentation::creatures::CreatureVisuals>| {
            creatures.reset(world)
        },
    );
    // The prepared commands and stable bindings make this boundary infallible.
    let mut presentation = candidate.presentation.take().unwrap();
    // The candidate ledger becomes the active ledger. Its last report and the last answer of
    // its stream were not exchanged yet. They go to the active paths, or the credit drifts.
    let unsent = presentation.take_command().world;
    presentation.commit(world);
    world.resource_mut::<Drive>().world.merge(unsent);
    if let Some(grant) = candidate.frame.grant.take() {
        let mut sim = world.resource_mut::<SimFrameResource>();
        sim.grant = Some(sim.grant.map_or(grant, |g| g.merge(grant)));
    }
    // The active renderer has the candidate copy now. The world changes of this frame follow it.
    world.resource_scope(|world, sim: Mut<SimFrameResource>| {
        let reference = world.resource::<RuntimeReference>().0.world.clone();
        world
            .resource_mut::<crate::presentation::world::runtime::SourcePresentationAllocation>()
            .apply_world(&reference, &sim.frame.world);
    });
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

/// Takes the candidate frame: the pose index and the world copy follow every frame.
fn receive(world: &mut World, candidate: &mut Candidate) {
    let mut sim = world.resource_mut::<SimFrameResource>();
    let filled = sim.frame.state.flags & StateBlock::CANDIDATE != 0;
    let Some(frame) = sim.frame.candidate.as_mut().filter(|_| filled) else {
        return;
    };
    std::mem::swap(&mut **frame, &mut candidate.frame.frame);
    candidate.frame.apply_changes();
    let delta = &candidate.frame.frame.world;
    match (&mut candidate.presentation, &mut candidate.mirror) {
        (Some(presentation), _) => presentation.apply_world(delta),
        (None, Some(mirror)) => mirror.apply(delta),
        (None, None) => {}
    }
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
    receive(world, candidate);
    if candidate.ready {
        return;
    }
    let reference = world.resource::<RuntimeReference>().0.clone();
    let _profile = cpu_profile::Timer::new(cpu_profile::Stage::LoadPrepare);
    let result = (|| -> Result<bool, crate::save::SaveError> {
        let Some(presentation) = &mut candidate.presentation else {
            if PreparedPresentation::can_start(world) {
                let mirror = candidate.mirror.take().expect("candidate world copy");
                candidate.presentation =
                    Some(PreparedPresentation::new(&reference, mirror, world)?);
            }
            return Ok(false);
        };
        if !candidate.stepped {
            return Ok(false);
        }
        presentation.advance(&reference, &mut candidate.frame, world)
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
