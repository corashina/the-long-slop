//! Renderer budget credit. The stream ledger checks the limits. The renderer spends the
//! credit the stream gives it and reports its use once per frame.
use super::renderer_pending::{PipelineContinuation, RendererOwner};
use crate::reference::ReferenceError;
use crate::world::{
    GeneratedId, PresentationAllocation, PresentationGrant, PresentationReport, WorldRequest,
    WorldStream,
};

/// Renderer side of the budget. Admission checks spend credit; charges always apply.
#[derive(Debug, Default)]
pub struct PresentationLedger {
    credit: PresentationAllocation,
    used: PresentationAllocation,
    want: PresentationAllocation,
    reclaim: bool,
    publication: bool,
}

impl PresentationLedger {
    /// The renderer allocation.
    pub fn allocation(&self) -> PresentationAllocation {
        self.used
    }
    /// Sets the renderer allocation. Use above the credit waits for the stream to cover it.
    pub fn set_allocation(&mut self, next: PresentationAllocation) -> Result<(), ReferenceError> {
        if next.dependency_staging_bytes > next.staging_bytes {
            return Err(ReferenceError(format!(
                "Dependency staging budget exceeded: requested {} bytes, limit {} bytes.",
                next.dependency_staging_bytes, next.staging_bytes
            )));
        }
        self.used = next;
        Ok(())
    }
    /// True if `extra` fits the credit. Otherwise asks for it. Only fields of `extra` above 0 count.
    pub fn request(&mut self, extra: PresentationAllocation) -> bool {
        let total = self.used.zip(extra, usize::saturating_add);
        self.want = self.want.zip(total, usize::max);
        let over = total.zip(self.credit, usize::saturating_sub);
        over.zip(extra, |over, extra| if extra > 0 { over } else { 0 })
            == PresentationAllocation::default()
    }
    /// Takes `claims` pending claims if the credit holds them.
    pub fn can_request_independent(&mut self, claims: usize) -> bool {
        let total = self.used.pending + claims;
        self.want.pending = self.want.pending.max(total);
        total <= self.credit.pending
    }
    /// Free job slots. None free asks for one.
    pub fn available_external_jobs(&mut self) -> usize {
        let free = self.credit.jobs.saturating_sub(self.used.jobs);
        if free == 0 {
            self.want.jobs = self.want.jobs.max(self.used.jobs + 1);
        }
        free
    }
    pub fn available_staging_bytes(&self) -> usize {
        self.credit
            .staging_bytes
            .saturating_sub(self.used.staging_bytes)
    }
    /// True if `bytes` more staging fit the credit. Otherwise asks for them.
    pub fn request_staging(&mut self, bytes: usize) -> bool {
        let total = self.used.staging_bytes + bytes;
        self.want.staging_bytes = self.want.staging_bytes.max(total);
        self.want.dependency_staging_bytes = self
            .want
            .dependency_staging_bytes
            .max(self.used.dependency_staging_bytes + bytes);
        total <= self.credit.staging_bytes
    }
    pub fn available_prototype_cpu_bytes(&self) -> usize {
        self.credit
            .prototype_cpu
            .saturating_sub(self.used.prototype_cpu)
    }
    pub fn available_prototype_gpu_bytes(&self) -> usize {
        self.credit
            .prototype_gpu
            .saturating_sub(self.used.prototype_gpu)
    }
    /// True if `cpu` and `gpu` more prototype bytes fit the credit. Otherwise asks for them.
    pub fn request_prototype(&mut self, cpu: usize, gpu: usize) -> bool {
        let want = &mut self.want;
        want.prototype_cpu = want.prototype_cpu.max(self.used.prototype_cpu + cpu);
        want.prototype_gpu = want.prototype_gpu.max(self.used.prototype_gpu + gpu);
        cpu <= self.available_prototype_cpu_bytes() && gpu <= self.available_prototype_gpu_bytes()
    }
    /// Gives one pending claim and its credit to the stream ready queue.
    fn hand_over_pending(&mut self) -> Result<(), ReferenceError> {
        if self.used.pending == 0 || self.credit.pending == 0 {
            return Err(ReferenceError("Missing instance renderer claim".into()));
        }
        self.used.pending -= 1;
        self.credit.pending -= 1;
        Ok(())
    }
    /// Takes one pending claim for a Grass renderer.
    pub fn claim_grass(&mut self, request: WorldRequest, entity: u64) -> PipelineContinuation {
        self.used.pending += 1;
        PipelineContinuation::counted(RendererOwner::Grass(request, entity))
    }
    /// Takes one pending claim for a structure renderer.
    pub fn claim_structure(
        &mut self,
        request: WorldRequest,
        id: GeneratedId,
    ) -> PipelineContinuation {
        self.used.pending += 1;
        PipelineContinuation::counted(RendererOwner::Structure(request, id))
    }
    /// Gives a drained structure claim and its credit to the stream ready queue.
    pub fn hand_over_claim(
        &mut self,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        if !matches!(claim.owner, RendererOwner::Structure(..)) {
            return Err(ReferenceError("Wrong structure claim".into()));
        }
        claim.release()?;
        self.hand_over_pending()
    }
    /// Gives back a drained Grass or structure claim.
    pub fn release_claim(
        &mut self,
        claim: &mut PipelineContinuation,
    ) -> Result<(), ReferenceError> {
        if matches!(claim.owner, RendererOwner::Surface(..)) {
            return Err(ReferenceError(
                "Surface claim belongs to its ready owner".into(),
            ));
        }
        claim.release()?;
        self.release_pending()
    }
    /// True once per frame when the stream lends its publication slot.
    pub fn take_publication(&mut self) -> bool {
        std::mem::take(&mut self.publication)
    }
    /// Frees one pending claim. Its credit stays until the report.
    pub fn release_pending(&mut self) -> Result<(), ReferenceError> {
        self.used.pending = self
            .used
            .pending
            .checked_sub(1)
            .ok_or_else(|| ReferenceError("Missing renderer pending claim".into()))?;
        Ok(())
    }
    /// The report of this frame. Unused credit goes back when the renderer is idle, or when
    /// the stream waits for room. Idle means no wants, no pending claims and no jobs, so
    /// work that spans frames keeps its credit.
    pub fn report(&mut self) -> PresentationReport {
        let want = self.want.zip(self.used, usize::max);
        let idle = want.zip(self.used, usize::saturating_sub) == PresentationAllocation::default()
            && self.used.pending == 0
            && self.used.jobs == 0;
        let release = if idle || self.reclaim {
            self.credit.zip(self.used, usize::saturating_sub)
        } else {
            PresentationAllocation::default()
        };
        self.credit = self.credit.zip(release, usize::saturating_sub);
        self.want = PresentationAllocation::default();
        PresentationReport {
            used: self.used,
            want,
            release,
        }
    }
    pub fn grant(&mut self, grant: PresentationGrant) {
        self.credit = self.credit.zip(grant.credit, usize::saturating_add);
        self.reclaim = grant.reclaim;
        self.publication = grant.publication;
    }
    /// Reports and takes the answer of `stream` in one call.
    pub fn exchange(&mut self, stream: &mut WorldStream) {
        let report = self.report();
        let grant = stream.exchange_presentation(report);
        self.grant(grant);
    }
}
