//! First-start authorization and complete-scene presentation tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreshSessionPermit {
    pub request_id: u32,
    pub session: u64,
    pub generation: u32,
}
impl FreshSessionPermit {
    pub fn prepare(
        request_id: u32,
        runtime: &crate::starting_area::GarageRuntime,
        generation: u32,
    ) -> Result<Self, super::SaveError> {
        if runtime.started {
            return Err(super::SaveError::ReconstructionFailure(
                "The session has started".into(),
            ));
        }
        Ok(Self {
            request_id,
            session: runtime.stream.frame().request.session,
            generation,
        })
    }
    pub fn commit(
        self,
        request_id: u32,
        runtime: &mut crate::starting_area::GarageRuntime,
        generation: u32,
    ) -> Result<(), super::SaveError> {
        if runtime.started
            || !self.matches(
                request_id,
                runtime.stream.frame().request.session,
                generation,
            )
        {
            return Err(super::SaveError::ReconstructionFailure(
                "Prepared session is stale".into(),
            ));
        }
        runtime.clear_input();
        runtime.started = true;
        runtime.paused = true;
        Ok(())
    }

    pub fn matches(self, request_id: u32, session: u64, generation: u32) -> bool {
        self == Self {
            request_id,
            session,
            generation,
        }
    }
}
#[derive(Default)]
pub struct PresentedFrames {
    scene: Option<(u64, u32)>,
    last: u32,
    count: u8,
}
impl PresentedFrames {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// Start above every token issued before this complete scene became ready.
    pub fn observe(&mut self, session: u64, generation: u32, issued: u32, submitted: u32) -> bool {
        if self.scene != Some((session, generation)) {
            self.scene = Some((session, generation));
            self.last = issued;
            self.count = 0;
        }
        if submitted > self.last {
            self.last = submitted;
            self.count = self.count.saturating_add(1);
        }
        self.count >= 2
    }
}
