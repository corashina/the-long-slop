//! Renderer budget credit. The stream ledger checks the limits. The renderer spends the
//! credit the stream gives it and reports its use once per frame.
use super::renderer_pending::{PipelineContinuation, RendererOwner};
use crate::reference::ReferenceError;
use crate::world::{
    GeneratedId, PresentationAllocation, PresentationGrant, PresentationReport, Reclaim,
    WorldRequest,
};

/// Renderer side of the budget. Admission checks spend credit; charges always apply.
#[derive(Debug, Default)]
pub struct PresentationLedger {
    credit: PresentationAllocation,
    used: PresentationAllocation,
    want: PresentationAllocation,
    reclaim: Reclaim,
    /// A stream prototype hold waits for room this frame.
    hold_waits: bool,
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
    /// Free job slots. Asks for one more, so the report keeps a free slot in the credit.
    pub fn available_external_jobs(&mut self) -> usize {
        self.want.jobs = self.want.jobs.max(self.used.jobs + 1);
        self.credit.jobs.saturating_sub(self.used.jobs)
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
    /// A stream prototype hold does not fit. This frame asks for no more prototype CPU, so
    /// the stream has room for the hold.
    pub fn wait_for_hold(&mut self) {
        self.hold_waits = true;
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
    /// The report of this frame. Credit above the wants of this frame goes back, and the
    /// unused credit of the fields that stream work waits for.
    pub fn report(&mut self) -> PresentationReport {
        let mut want = self.want.zip(self.used, usize::max);
        if std::mem::take(&mut self.hold_waits) {
            want.prototype_cpu = self.used.prototype_cpu;
        }
        // Credit above the wants of this frame goes back. Wanted credit stays, so work that
        // needs several fields at once collects them.
        let mut release = self.credit.zip(want, usize::saturating_sub);
        release = release.zip(
            self.reclaim
                .take(self.credit.zip(self.used, usize::saturating_sub)),
            usize::max,
        );
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_gates_admission_and_unused_credit_returns() {
        let mut ledger = PresentationLedger::default();
        assert!(!ledger.can_request_independent(1));
        assert_eq!(ledger.available_external_jobs(), 0);
        let report = ledger.report();
        assert_eq!((report.want.pending, report.want.jobs), (1, 1));
        let mut credit = PresentationAllocation::default();
        credit.pending = 1;
        credit.jobs = 1;
        ledger.grant(PresentationGrant {
            credit,
            ..Default::default()
        });
        assert!(ledger.can_request_independent(1));
        let mut used = ledger.allocation();
        used.pending = 1;
        ledger.set_allocation(used).unwrap();
        // The claim keeps its use. The unused job goes back.
        let report = ledger.report();
        assert_eq!((report.used.pending, report.release.jobs), (1, 1));
        assert_eq!(report.release.pending, 0);
        // A freed claim gives its credit back at the next report.
        ledger.release_pending().unwrap();
        assert_eq!(ledger.report().release.pending, 1);
    }

    #[test]
    fn reclaim_gives_back_the_unused_credit_of_the_waited_field() {
        let mut credit = PresentationAllocation::default();
        credit.jobs = 2;
        credit.staging_bytes = 64;
        let mut ledger = PresentationLedger::default();
        ledger.grant(PresentationGrant {
            credit,
            reclaim: Reclaim {
                jobs: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(ledger.available_external_jobs(), 2);
        assert!(ledger.request_staging(64));
        // Both are still wanted. The stream waits for jobs only.
        let report = ledger.report();
        assert_eq!((report.release.jobs, report.release.staging_bytes), (2, 0));
    }

    #[test]
    fn waiting_hold_asks_for_no_prototype_cpu() {
        let mut ledger = PresentationLedger::default();
        assert!(!ledger.request_prototype(64, 0));
        ledger.wait_for_hold();
        assert_eq!(ledger.report().want.prototype_cpu, 0);
        assert!(!ledger.request_prototype(64, 0));
        assert_eq!(ledger.report().want.prototype_cpu, 64);
    }

    #[test]
    fn grass_claims_count_pending_and_publication_is_lent_once() {
        let mut ledger = PresentationLedger::default();
        let mut claim = ledger.claim_grass(
            WorldRequest {
                session: 1,
                revision: 0,
                player: [0.; 3],
                vehicle: [0.; 3],
            },
            7,
        );
        assert_eq!(ledger.allocation().pending, 1);
        ledger.release_claim(&mut claim).unwrap();
        assert_eq!(ledger.allocation().pending, 0);
        assert!(ledger.release_claim(&mut claim).is_err());
        ledger.grant(PresentationGrant {
            publication: true,
            ..Default::default()
        });
        assert!(ledger.take_publication());
        assert!(!ledger.take_publication());
    }

    #[test]
    fn grants_that_wait_for_one_renderer_step_keep_all_credit() {
        // Two worker frames can arrive before a renderer step takes the grant.
        let mut first = PresentationGrant::default();
        first.credit.jobs = 1;
        first.publication = true;
        let mut second = PresentationGrant::default();
        second.credit.jobs = 1;
        second.credit.staging_bytes = 64;
        let mut ledger = PresentationLedger::default();
        ledger.grant(first.merge(second));
        assert_eq!(ledger.available_external_jobs(), 2);
        assert_eq!(ledger.available_staging_bytes(), 64);
        // The lend belongs to the latest stream step only.
        assert!(!ledger.take_publication());
    }
}
