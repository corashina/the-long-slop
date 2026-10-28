//! Browser save requests in the simulation: capture, prepare and commit.
use crate::boundary::{Change, SaveCommand, SimFrame, StateBlock};
use crate::candidate::CandidateDriver;
use crate::driver::{CameraMode, SimDriver};
use crate::reference::ReferenceManifest;
use crate::save::fresh_permit::FreshSessionPermit;
use crate::save::{CameraSave, SaveError, SaveSnapshot, SourceStamp};
use crate::starting_area::GarageRuntime;
use crate::world::GameMode;
use serde::{Deserialize, Serialize};

/// The prepared session.
enum Prepared {
    /// A new game in the current runtime.
    Fresh(FreshSessionPermit),
    /// A loaded or new runtime that the renderer prepares offscreen.
    Candidate {
        id: u32,
        driver: Box<CandidateDriver>,
    },
}

pub(crate) struct SaveState {
    source: SourceStamp,
    prepared: Option<Prepared>,
    /// Results that the next frame carries.
    results: Vec<Change>,
}

#[derive(Deserialize)]
struct Request {
    id: u32,
    kind: String,
    #[serde(default)]
    payload: serde_json::Value,
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

impl SaveState {
    pub(crate) fn new(source: SourceStamp) -> Self {
        Self {
            source,
            prepared: None,
            results: Vec::new(),
        }
    }
}

impl SimDriver {
    /// Applies the save commands of a frame, after its input.
    pub(crate) fn accept_saves(&mut self, reference: &ReferenceManifest, saves: Vec<SaveCommand>) {
        for save in saves {
            match save {
                SaveCommand::Request { json, mode, epoch } => {
                    let (id, kind, result) = match serde_json::from_str::<Request>(&json) {
                        Ok(request) => {
                            let result = self.request(reference, &request, mode, epoch);
                            (request.id, request.kind, result)
                        }
                        Err(error) => (
                            0,
                            "Invalid".into(),
                            Err(SaveError::CorruptData(error.to_string())),
                        ),
                    };
                    let (payload, error) = match result {
                        Ok(None) => continue,
                        Ok(Some(payload)) => (payload, String::new()),
                        Err(error) => (String::new(), error.to_string()),
                    };
                    self.save.results.push(Change::SaveResult {
                        id,
                        kind,
                        payload,
                        error,
                    });
                }
                SaveCommand::Cancel(id) => {
                    let current = match &self.save.prepared {
                        Some(Prepared::Fresh(permit)) => permit.request_id,
                        Some(Prepared::Candidate { id, .. }) => *id,
                        None => continue,
                    };
                    if current == id {
                        self.save.prepared = None;
                    }
                }
            }
        }
    }

    /// Runs one request. `None`: the renderer answers when the candidate is ready.
    fn request(
        &mut self,
        reference: &ReferenceManifest,
        request: &Request,
        mode: GameMode,
        epoch: u32,
    ) -> Result<Option<String>, SaveError> {
        let invalid = |message: &str| SaveError::ReconstructionFailure(message.into());
        match request.kind.as_str() {
            "Commit" => {
                #[cfg(target_arch = "wasm32")]
                let _profile =
                    crate::cpu_profile::Timer::new(crate::cpu_profile::Stage::SaveCommit);
                let id = request
                    .payload
                    .as_u64()
                    .and_then(|id| u32::try_from(id).ok());
                match self.save.prepared.take() {
                    Some(Prepared::Fresh(permit)) => {
                        let id = id.ok_or_else(|| invalid("Prepared session is stale"));
                        let result = id.and_then(|id| {
                            if !self.ready {
                                return Err(invalid("The garage is not ready"));
                            }
                            permit.commit(id, &mut self.runtime, epoch)
                        });
                        if let Err(error) = result {
                            self.save.prepared = Some(Prepared::Fresh(permit));
                            return Err(error);
                        }
                        self.clear_input();
                        self.frame_steps = 0;
                        Ok(Some(String::new()))
                    }
                    Some(Prepared::Candidate {
                        id: prepared,
                        driver,
                    }) if Some(prepared) == id => {
                        self.commit(reference, *driver);
                        Ok(Some(String::new()))
                    }
                    other => {
                        self.save.prepared = other;
                        Err(invalid("Prepared session is missing"))
                    }
                }
            }
            "Capture" | "Prepare" | "PrepareNewGame" => {
                #[cfg(target_arch = "wasm32")]
                let _profile = crate::cpu_profile::Timer::new(if request.kind == "Prepare" {
                    crate::cpu_profile::Stage::LoadPrepare
                } else {
                    crate::cpu_profile::Stage::SavePrepare
                });
                if !self.ready {
                    return Err(invalid("The garage is not ready"));
                }
                let payload = &request.payload;
                let autosave = request.kind == "Capture"
                    && payload
                        .get("autosave")
                        .and_then(serde_json::Value::as_bool)
                        .ok_or_else(|| invalid("Missing autosave flag"))?;
                if !autosave && self.runtime.started && !self.runtime.paused {
                    return Err(invalid("Pause the game first"));
                }
                if request.kind == "Capture" {
                    if !autosave {
                        self.clear_input();
                    }
                    let snapshot = self.runtime.capture_save(
                        reference,
                        &self.save.source,
                        if self.camera == CameraMode::Chase {
                            CameraSave::Chase
                        } else {
                            CameraSave::FirstPerson
                        },
                        payload
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
                    .map(Some)
                    .map_err(|error| SaveError::CorruptData(error.to_string()));
                }
                if self.save.prepared.is_some() {
                    return Err(invalid("A prepared session is pending"));
                }
                let draw_distance = payload
                    .get("draw_distance")
                    .and_then(serde_json::Value::as_f64)
                    .ok_or_else(|| invalid("Missing draw distance"))?
                    as f32;
                let runtime = &self.runtime;
                if request.kind == "PrepareNewGame"
                    && !runtime.started
                    && runtime.stream.mode() == mode
                    && runtime
                        .distances
                        .matches(&reference.world.draw_distance, draw_distance)
                {
                    let permit = FreshSessionPermit::prepare(request.id, runtime, epoch)?;
                    self.save.prepared = Some(Prepared::Fresh(permit));
                    return Ok(Some(request.id.to_string()));
                }
                let (mut runtime, camera) = if request.kind == "Prepare" {
                    let saved = payload.get("save").ok_or_else(|| invalid("Missing save"))?;
                    let saved = SaveSnapshot::deserialize(saved)
                        .map_err(|error| SaveError::CorruptData(error.to_string()))?;
                    let runtime = GarageRuntime::from_save(
                        reference,
                        &self.save.source,
                        &saved,
                        mode,
                        draw_distance,
                    )?;
                    let camera = if saved.view.camera == CameraSave::Chase {
                        CameraMode::Chase
                    } else {
                        CameraMode::Cockpit
                    };
                    (runtime, camera)
                } else {
                    (
                        GarageRuntime::new(reference, mode, draw_distance)?,
                        CameraMode::default(),
                    )
                };
                runtime.started = true;
                runtime.paused = true;
                self.save.prepared = Some(Prepared::Candidate {
                    id: request.id,
                    driver: Box::new(CandidateDriver::new(runtime, camera, self.sent.fork())),
                });
                Ok(None)
            }
            _ => Err(invalid("Unknown save request")),
        }
    }

    /// Replaces the runtime with the prepared candidate. The renderer prepared its scene.
    fn commit(&mut self, reference: &ReferenceManifest, driver: CandidateDriver) {
        let session = self
            .runtime
            .audio
            .as_ref()
            .map_or(1, |audio| audio.session.wrapping_add(1));
        let settings = self.runtime.driving_input.settings;
        let camera = driver.camera;
        // Assignment drops the old stream and terminates its road actor first.
        self.runtime = driver.runtime;
        // The renderer takes the candidate copy with the candidate scene.
        self.sent = driver.sent;
        // The candidate renderer becomes the active renderer. Its input and credit answer go on.
        // The input of the old renderer belongs to the dropped stream.
        self.world = driver.world;
        self.grant = driver.grant;
        self.runtime.driving_input.settings = settings;
        self.runtime
            .stream
            .set_overlap(crate::world::WorldMetrics::default())
            .expect("retired world overlap only removes charges");
        let runtime = &mut self.runtime;
        if let Some(audio) = &mut runtime.audio {
            audio.reset(session);
            audio.sync_lamps(&reference.lighting, &runtime.lighting);
        }
        self.restart(camera);
        self.runtime.advance_instruments(reference, 0., true);
        // The candidate frames consumed the sent state of the new frame stream.
        self.runtime.reset_frame_outputs();
        self.save.results.push(Change::Restarted);
    }

    /// One candidate step from the renderer input, after the active world step.
    pub(crate) fn advance_candidate(
        &mut self,
        reference: &ReferenceManifest,
        command: Option<crate::boundary::CandidateCommand>,
    ) {
        let (Some(command), Some(Prepared::Candidate { driver, .. })) =
            (command, &mut self.save.prepared)
        else {
            return;
        };
        driver.accept(command);
        driver.advance(reference, self.runtime.stream.metrics(), self.calendar);
    }

    /// Writes the save results and the candidate frame into `frame`.
    pub(crate) fn fill_saves(&mut self, reference: &ReferenceManifest, frame: &mut SimFrame) {
        frame.changes.append(&mut self.save.results);
        match &mut self.save.prepared {
            Some(Prepared::Candidate { driver, .. }) => {
                frame.state.flags |= StateBlock::CANDIDATE;
                driver.fill(reference, frame.candidate.get_or_insert_default());
            }
            _ => frame.candidate = None,
        }
    }
}
