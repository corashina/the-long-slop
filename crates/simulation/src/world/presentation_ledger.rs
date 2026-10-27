//! Renderer budget credit. The stream ledger checks the limits. The renderer spends the
//! credit the stream gives it and reports its use once per frame.
use super::PresentationAllocation;
#[cfg(any(target_arch = "wasm32", test))]
use super::WorldRequest;
#[cfg(any(target_arch = "wasm32", test))]
use super::renderer_pending::{PipelineContinuation, RendererOwner};
use crate::reference::ReferenceError;

impl PresentationAllocation {
    /// Applies `f` to each pair of fields.
    pub fn zip(self, other: Self, f: impl Fn(usize, usize) -> usize) -> Self {
        Self {
            grass_worker_wasm_bytes: f(self.grass_worker_wasm_bytes, other.grass_worker_wasm_bytes),
            grass_worker_cache_bytes: f(
                self.grass_worker_cache_bytes,
                other.grass_worker_cache_bytes,
            ),
            grass_worker_reserved_bytes: f(
                self.grass_worker_reserved_bytes,
                other.grass_worker_reserved_bytes,
            ),
            active_cpu: f(self.active_cpu, other.active_cpu),
            active_gpu: f(self.active_gpu, other.active_gpu),
            prototype_cpu: f(self.prototype_cpu, other.prototype_cpu),
            prototype_gpu: f(self.prototype_gpu, other.prototype_gpu),
            jobs: f(self.jobs, other.jobs),
            pending: f(self.pending, other.pending),
            staging_bytes: f(self.staging_bytes, other.staging_bytes),
            dependency_staging_bytes: f(
                self.dependency_staging_bytes,
                other.dependency_staging_bytes,
            ),
            objects: f(self.objects, other.objects),
        }
    }
}

/// The renderer use of one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PresentationReport {
    pub used: PresentationAllocation,
    /// Totals the renderer wanted to reach this frame.
    pub want: PresentationAllocation,
    /// Credit the renderer gave back. It no longer spends it.
    pub release: PresentationAllocation,
}

/// The stream answer: added credit, and whether the stream waits for room.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PresentationGrant {
    pub credit: PresentationAllocation,
    pub reclaim: bool,
    /// The stream lends its publication slot of this frame.
    pub publication: bool,
}

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
    pub fn hand_over_pending(&mut self) -> Result<(), ReferenceError> {
        if self.used.pending == 0 || self.credit.pending == 0 {
            return Err(ReferenceError("Missing instance renderer claim".into()));
        }
        self.used.pending -= 1;
        self.credit.pending -= 1;
        Ok(())
    }
    /// Takes one pending claim for a Grass renderer.
    #[cfg(any(target_arch = "wasm32", test))]
    pub fn claim_grass(&mut self, request: WorldRequest, entity: u64) -> PipelineContinuation {
        self.used.pending += 1;
        PipelineContinuation::counted(RendererOwner::Grass(request, entity))
    }
    /// Gives back a drained Grass or structure claim.
    #[cfg(any(target_arch = "wasm32", test))]
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
    pub fn exchange(&mut self, stream: &mut super::WorldStream) {
        let report = self.report();
        let grant = stream.exchange_presentation(report);
        self.grant(grant);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_gates_admission_and_idle_credit_returns() {
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
            reclaim: false,
            ..Default::default()
        });
        assert!(ledger.can_request_independent(1));
        let mut used = ledger.allocation();
        used.pending = 1;
        ledger.set_allocation(used).unwrap();
        // A claim in flight keeps the credit.
        let report = ledger.report();
        assert_eq!(report.release, PresentationAllocation::default());
        // With nothing in flight and nothing wanted, all credit goes back.
        ledger.release_pending().unwrap();
        let report = ledger.report();
        assert_eq!((report.release.jobs, report.release.pending), (1, 1));
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
}
