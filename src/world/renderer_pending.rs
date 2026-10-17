//! Counted renderer owners serialize pipeline work through their existing pending claim.
use super::{GeneratedId, SurfaceKey, WorldRequest, WorldStream, upload_receipt::UploadReceipt};
use crate::reference::ReferenceError;

pub(crate) enum RendererOwner {
    Surface(WorldRequest, SurfaceKey),
    Structure(WorldRequest, GeneratedId),
    Grass(WorldRequest, u64),
}
impl RendererOwner {
    fn request(&self) -> WorldRequest {
        match self {
            Self::Surface(r, _) | Self::Structure(r, _) | Self::Grass(r, _) => *r,
        }
    }
}
/// This proof has no Clone implementation. Only checked stream admission constructs it.
pub(crate) struct PipelineContinuation {
    pub(super) owner: RendererOwner,
    held: bool,
    flight: Option<UploadReceipt>,
    upload: Option<UploadReceipt>,
}
impl PipelineContinuation {
    pub(super) fn counted(owner: RendererOwner) -> Self {
        Self {
            owner,
            held: true,
            flight: None,
            upload: None,
        }
    }
    pub(crate) fn request(&self) -> WorldRequest {
        self.owner.request()
    }
    pub(crate) fn watch_upload(&mut self, receipt: UploadReceipt) {
        self.upload = Some(receipt);
    }
    pub(crate) fn drained(&self) -> bool {
        self.flight.as_ref().is_none_or(UploadReceipt::completed)
            && self.upload.as_ref().is_none_or(UploadReceipt::completed)
    }
    pub(super) fn release(&mut self) -> Result<(), ReferenceError> {
        if !self.held || !self.drained() {
            return Err(ReferenceError("Renderer claim is live or released".into()));
        }
        self.held = false;
        Ok(())
    }
    pub(crate) fn start(
        &mut self,
        stream: &mut WorldStream,
        cpu: usize,
        staging: usize,
    ) -> Result<Option<PipelineOperation>, ReferenceError> {
        if !self.held || self.request().session != stream.frame().request.session {
            return Err(ReferenceError("Stale renderer continuation proof".into()));
        }
        if self.flight.as_ref().is_some_and(|f| !f.completed()) {
            return Ok(None);
        }
        let operation = PipelineOperation::admit(stream, false, cpu, staging)?;
        if let Some(operation) = &operation {
            self.flight = Some(operation.physical.clone());
        }
        Ok(operation)
    }
}
/// A shared ticket retains one physical operation. Copies add no pending claim.
#[derive(Clone)]
pub(crate) struct PipelineOperation {
    pub(crate) owns_pending: bool,
    pub(crate) physical: UploadReceipt,
}
impl PipelineOperation {
    pub(crate) fn independent(
        stream: &mut WorldStream,
        cpu: usize,
        staging: usize,
    ) -> Result<Option<Self>, ReferenceError> {
        Self::admit(stream, true, cpu, staging)
    }
    fn admit(
        stream: &mut WorldStream,
        owns_pending: bool,
        cpu: usize,
        staging: usize,
    ) -> Result<Option<Self>, ReferenceError> {
        if stream.available_external_jobs() == 0
            || (owns_pending && !stream.can_request_independent(1))
        {
            return Ok(None);
        }
        let mut allocation = stream.presentation_allocation();
        allocation.jobs += 1;
        allocation.pending += usize::from(owns_pending);
        allocation.prototype_cpu += cpu;
        allocation.staging_bytes += staging;
        stream.set_presentation_allocation(allocation)?;
        Ok(Some(Self {
            owns_pending,
            physical: UploadReceipt::default(),
        }))
    }
}
#[derive(Clone, Copy, Default)]
pub(crate) struct PipelineAccounting {
    pub(crate) jobs: usize,
    pub(crate) pending: usize,
    pub(crate) tickets: usize,
}
impl PipelineAccounting {
    pub(crate) fn admitted(&mut self, operation: &PipelineOperation) {
        self.jobs += 1;
        self.pending += usize::from(operation.owns_pending);
        self.tickets += 1;
    }
    pub(crate) fn reconcile(
        &mut self,
        stream: &mut WorldStream,
        operations: impl Iterator<Item = PipelineOperation>,
        cpu: usize,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let mut next = Self::default();
        for operation in operations {
            next.tickets += 1;
            if !operation.physical.completed() {
                next.jobs += 1;
                next.pending += usize::from(operation.owns_pending);
            }
        }
        let mut allocation = stream.presentation_allocation();
        allocation.jobs = allocation.jobs.checked_sub(self.jobs).unwrap() + next.jobs;
        allocation.pending = allocation.pending.checked_sub(self.pending).unwrap() + next.pending;
        allocation.staging_bytes = allocation
            .staging_bytes
            .checked_sub(self.jobs * staging)
            .unwrap()
            + next.jobs * staging;
        allocation.prototype_cpu = allocation
            .prototype_cpu
            .checked_sub(self.tickets * cpu)
            .unwrap()
            + next.tickets * cpu;
        stream.set_presentation_allocation(allocation)?;
        *self = next;
        Ok(())
    }
}

/// Physical drain checks inspect every admitted operation, including siblings after a failure.
#[derive(Clone, Copy)]
pub(crate) enum PhysicalPipelineState {
    Queued,
    Compiling,
    Retryable,
    Ready,
    Failed,
}
pub(crate) fn pipeline_states_drained(states: impl Iterator<Item = PhysicalPipelineState>) -> bool {
    states.fold(true, |drained, state| {
        matches!(
            state,
            PhysicalPipelineState::Ready | PhysicalPipelineState::Failed
        ) & drained
    })
}
/// Once retirement starts, a returning source must wait for a new parent record.
#[derive(Default)]
pub(crate) struct RendererRetirement(bool);
impl RendererRetirement {
    pub(crate) fn keep(&mut self, wanted: bool) -> bool {
        self.0 |= !wanted;
        !self.0
    }
    pub(crate) fn started(&self) -> bool {
        self.0
    }
    pub(crate) fn drained(&self, parent: bool, children: bool) -> bool {
        self.0 && parent && children
    }
}
