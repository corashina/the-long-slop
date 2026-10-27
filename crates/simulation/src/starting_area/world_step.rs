//! World logic of one frame step. The renderer prepares after it.
use super::GarageRuntime;
use crate::{
    reference::{ReferenceError, ReferenceManifest},
    world::{GeneratedId, RequiredView, stream_profile},
};

impl GarageRuntime {
    /// Publishes the renderer-ready work, then prepares the next work.
    /// `admitted_lights` are the lamps the renderer admitted in its last step.
    pub fn advance_world(
        &mut self,
        reference: &ReferenceManifest,
        required_views: &[RequiredView],
        admitted_lights: Option<&[GeneratedId]>,
        budget_ms: f64,
    ) -> Result<(), ReferenceError> {
        let mut timer = stream_profile::Timer::new(stream_profile::Stage::Publication);
        // Light admission and complete renderer preparation precede collision mutation.
        if let Some(required) = admitted_lights {
            self.stream.publish(&mut self.physics, required)?;
        }
        timer.next(stream_profile::Stage::Supplies);
        self.stream.sync_supplies(
            reference,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.physics,
            &mut self.wheels,
            &mut self.vehicles,
            &mut self.powertrain,
        )?;
        timer.next(stream_profile::Stage::Capture);
        self.stream.capture_changes(
            reference,
            &mut self.area.items,
            &self.area.interaction,
            &self.physics,
        );
        timer.next(stream_profile::Stage::FinalCleanup);
        self.stream.retire_structures(
            &mut self.physics,
            &mut self.area.items,
            &mut self.area.interaction,
            &mut self.wheels,
            &mut self.vehicles,
            &mut self.powertrain,
        )?;
        timer.next(stream_profile::Stage::Stream);
        self.stream.prepare(budget_ms);
        self.stream.select_required(required_views);
        Ok(())
    }
}
