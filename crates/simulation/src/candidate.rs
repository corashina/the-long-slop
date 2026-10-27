//! Prepares a loaded or new runtime offscreen while the active runtime runs.
use crate::boundary::{CandidateCommand, Change, Handle, SimFrame, StateBlock};
use crate::driver::{CameraMode, LOADING_BUDGET_MS};
use crate::reference::ReferenceManifest;
use crate::simulation::FIXED_DT;
use crate::starting_area::{GarageRuntime, WorldInput};
use crate::world::{PresentationGrant, WorldMetrics};

pub struct CandidateDriver {
    pub runtime: GarageRuntime,
    pub camera: CameraMode,
    world: WorldInput,
    /// Renderer bytes that both renderers share. See `CandidateCommand`.
    shared: [usize; 2],
    fixed: [usize; 2],
    /// Answer of the last world step to the renderer report.
    grant: Option<PresentationGrant>,
    /// The saved links are restored.
    restored: bool,
    /// The first failure. The candidate stops after it.
    failure: Option<String>,
    failed: bool,
}

impl CandidateDriver {
    pub fn new(runtime: GarageRuntime, camera: CameraMode) -> Self {
        Self {
            runtime,
            camera,
            world: WorldInput::default(),
            shared: [0; 2],
            fixed: [0; 2],
            grant: None,
            restored: false,
            failure: None,
            failed: false,
        }
    }

    pub fn accept(&mut self, command: CandidateCommand) {
        self.world.accept(command.world);
        self.shared = command.shared;
        self.fixed = command.fixed;
    }

    /// One loading step: storage startup, then the world step. `active` is the running world.
    pub fn advance(
        &mut self,
        reference: &ReferenceManifest,
        active: WorldMetrics,
        calendar: [u8; 2],
    ) {
        if self.failed {
            return;
        }
        if let Err(error) = self.step(reference, active, calendar) {
            self.failure = Some(error);
            self.failed = true;
        }
    }

    fn step(
        &mut self,
        reference: &ReferenceManifest,
        active: WorldMetrics,
        calendar: [u8; 2],
    ) -> Result<(), String> {
        if let Some(failure) = self.world.take_failure() {
            return Err(failure);
        }
        let text = |e: &dyn std::fmt::Display| e.to_string();
        let runtime = &mut self.runtime;
        runtime.stream.advance_generation();
        let overlap = overlap(active, self.shared, self.fixed).ok_or("Shared overlap overflow")?;
        runtime.stream.set_overlap(overlap).map_err(|e| text(&e))?;
        runtime
            .initialize_storage_frame(FIXED_DT)
            .map_err(|e| text(&e))?;
        self.grant = runtime
            .step_world(reference, &mut self.world, LOADING_BUDGET_MS)
            .map_err(|e| text(&e))?;
        if let Some(failure) = runtime.stream.frame().failure {
            return Err(failure.to_owned());
        }
        runtime
            .align_startup(reference, calendar)
            .map_err(|e| text(&e))?;
        self.restored = runtime
            .advance_saved_restore(reference)
            .map_err(|e| text(&e))?
            && !runtime.restore_pending();
        runtime.sync_world_activity(reference).map_err(|e| text(&e))
    }

    /// Writes the candidate state into `frame`. The camera is the player view without interpolation.
    pub fn fill(&mut self, reference: &ReferenceManifest, frame: &mut SimFrame) {
        self.runtime.fill_frame(reference, 1., frame);
        let player = &self.runtime.area.player;
        let mut look = player.look(&self.runtime.physics);
        look.gain = 1.;
        look.position = [player.view(&self.runtime.physics).camera_pose.position; 2];
        look.parent[0] = look.parent[1];
        frame.state.look = look;
        if self.camera == CameraMode::Chase {
            frame.state.flags |= StateBlock::CHASE;
        }
        if self.restored {
            frame.state.flags |= StateBlock::RESTORED;
        }
        if let Some(grant) = self.grant.take() {
            frame.changes.push(Change::Grant(grant));
        }
        if let Some(failure) = self.failure.take() {
            frame.changes.push(Change::Failure(failure));
        }
    }

    /// The renderer dropped a retired pose handle.
    pub fn release_pose(&mut self, handle: Handle) {
        self.runtime.release_pose(handle);
    }
}

/// The active world use that the candidate world must leave room for.
/// The shared prototype and fixed bytes count once.
fn overlap(active: WorldMetrics, shared: [usize; 2], fixed: [usize; 2]) -> Option<WorldMetrics> {
    let mut overlap = active;
    overlap.prototype_cpu_bytes = overlap
        .prototype_cpu_bytes
        .saturating_sub(overlap.shared_input_cpu_bytes + shared[0])
        .checked_sub(fixed[0])?;
    overlap.prototype_gpu_bytes = overlap
        .prototype_gpu_bytes
        .saturating_sub(shared[1])
        .checked_sub(fixed[1])?;
    Some(overlap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_counts_shared_bytes_once() {
        let active = WorldMetrics {
            prototype_cpu_bytes: 100,
            prototype_gpu_bytes: 50,
            shared_input_cpu_bytes: 10,
            ..WorldMetrics::default()
        };
        let overlap = overlap(active, [20, 5], [30, 15]).unwrap();
        assert_eq!(overlap.prototype_cpu_bytes, 40);
        assert_eq!(overlap.prototype_gpu_bytes, 30);
        assert!(super::overlap(active, [0; 2], [101, 0]).is_none());
    }
}
