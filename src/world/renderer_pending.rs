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
    pub(crate) fn validate(&self, stream: &WorldStream) -> Result<(), ReferenceError> {
        if !self.held
            || self.request().session != stream.frame().request.session
            || self.flight.as_ref().is_some_and(UploadReceipt::stopped)
        {
            return Err(ReferenceError("Stale renderer continuation proof".into()));
        }
        Ok(())
    }
    pub(crate) fn stop_flight(&self) {
        if let Some(flight) = &self.flight {
            flight.stop();
        }
    }
    pub(crate) fn shares(&self, receipt: &UploadReceipt) -> bool {
        self.flight
            .as_ref()
            .is_some_and(|flight| flight.same(receipt))
    }
    pub(crate) fn start(
        &mut self,
        stream: &mut WorldStream,
        cpu: usize,
        staging: usize,
    ) -> Result<Option<PipelineOperation>, ReferenceError> {
        self.validate(stream)?;
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
    pub(crate) fn reserve_pair(
        &self,
        stream: &mut WorldStream,
        cpu: usize,
    ) -> Result<(), ReferenceError> {
        if self.physical.completed() || self.physical.stopped() {
            return Err(ReferenceError("Stale source pipeline batch".into()));
        }
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu += cpu;
        stream.set_presentation_allocation(allocation)
    }
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
/// Each admitted pair reserves this payload, allocated only when a diagnostic exists.
pub(crate) const PIPELINE_DIAGNOSTIC_BYTES: usize = 4096;
const DIAGNOSTIC_TRUNCATED: &str = " [truncated]";
struct PipelineDiagnosticWriter<'a> {
    text: &'a mut String,
    truncated: bool,
}
impl std::fmt::Write for PipelineDiagnosticWriter<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        if self.truncated {
            return Ok(());
        }
        if value.len() <= PIPELINE_DIAGNOSTIC_BYTES - self.text.len() {
            self.text.push_str(value);
            return Ok(());
        }
        let limit = PIPELINE_DIAGNOSTIC_BYTES - DIAGNOSTIC_TRUNCATED.len();
        if self.text.len() > limit {
            let mut end = limit;
            while !self.text.is_char_boundary(end) {
                end -= 1;
            }
            self.text.truncate(end);
        }
        let mut end = (limit - self.text.len()).min(value.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.text.push_str(&value[..end]);
        self.text.push_str(DIAGNOSTIC_TRUNCATED);
        self.truncated = true;
        Ok(())
    }
}
pub(crate) fn write_pipeline_diagnostic(
    output: &mut Option<String>,
    args: std::fmt::Arguments<'_>,
) {
    let text = output.get_or_insert_with(|| String::with_capacity(PIPELINE_DIAGNOSTIC_BYTES));
    assert_eq!(text.capacity(), PIPELINE_DIAGNOSTIC_BYTES);
    text.clear();
    let mut writer = PipelineDiagnosticWriter {
        text,
        truncated: false,
    };
    std::fmt::write(&mut writer, args).expect("pipeline diagnostic formatter failed");
    assert!(writer.text.len() <= PIPELINE_DIAGNOSTIC_BYTES);
    assert_eq!(writer.text.capacity(), PIPELINE_DIAGNOSTIC_BYTES);
}
pub(crate) fn pipeline_diagnostic(args: std::fmt::Arguments<'_>) -> String {
    let mut output = None;
    write_pipeline_diagnostic(&mut output, args);
    output.unwrap()
}
pub(crate) struct PipelineBufferGrowth {
    pub(crate) capacity: usize,
    pub(crate) admitted: usize,
    pub(crate) released: usize,
}
impl PipelineBufferGrowth {
    pub(crate) fn new(capacity: usize, needed: usize, item: usize) -> Self {
        if needed <= capacity {
            Self {
                capacity,
                admitted: 0,
                released: 0,
            }
        } else {
            let next = capacity.saturating_mul(2).max(needed).max(4);
            Self {
                capacity: next,
                admitted: next * item,
                released: capacity * item,
            }
        }
    }
}
pub(crate) fn pipeline_pair_ready(receipt: &UploadReceipt, spill: bool) -> bool {
    !spill && !receipt.stopped() && receipt.completed()
}
/// Base requests include cache hits. Companions retain their existing constructors.
pub(crate) const PIPELINE_BATCH_KEYS: usize = 32;
#[derive(Default)]
pub(crate) struct PipelineBatchWork {
    pub(crate) pairs: usize,
    pub(crate) base_keys: usize,
}
impl PipelineBatchWork {
    pub(crate) fn accept(&mut self, keys: usize) -> Result<bool, ReferenceError> {
        if keys == 0 || keys > PIPELINE_BATCH_KEYS {
            return Err(ReferenceError(
                "Source material pipeline reservation exceeded".into(),
            ));
        }
        if self.base_keys + keys > PIPELINE_BATCH_KEYS {
            return Ok(false);
        }
        self.base_keys += keys;
        self.pairs += 1;
        Ok(true)
    }
}
/// Fixed render-stack keys preserve first occurrence and shadow/view order.
pub(crate) struct PipelineKeys<K> {
    values: [Option<K>; PIPELINE_BATCH_KEYS],
    pub(crate) len: usize,
}
impl<K: Copy + PartialEq> PipelineKeys<K> {
    pub(crate) fn new() -> Self {
        Self {
            values: [None; PIPELINE_BATCH_KEYS],
            len: 0,
        }
    }
    pub(crate) fn push(&mut self, key: K) -> Result<(), ReferenceError> {
        if self.values[..self.len].contains(&Some(key)) {
            return Ok(());
        }
        if self.len == PIPELINE_BATCH_KEYS {
            return Err(ReferenceError(
                "Source material pipeline reservation exceeded".into(),
            ));
        }
        self.values[self.len] = Some(key);
        self.len += 1;
        Ok(())
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = K> + '_ {
        self.values[..self.len].iter().map(|v| v.unwrap())
    }
}
pub(crate) struct PipelineBatch<P> {
    pub(crate) operation: PipelineOperation,
    pub(crate) pairs: [std::sync::Weak<P>; PIPELINE_BATCH_KEYS],
    pub(crate) len: usize,
    pub(crate) next: usize,
    pub(crate) open: bool,
    pub(crate) stopped: bool,
    pub(crate) work: PipelineBatchWork,
    #[cfg(target_arch = "wasm32")]
    pub(crate) dispatched_keys: usize,
}
impl<P> PipelineBatch<P> {
    pub(crate) fn new(operation: PipelineOperation) -> Self {
        Self {
            operation,
            pairs: std::array::from_fn(|_| std::sync::Weak::new()),
            len: 0,
            next: 0,
            open: true,
            stopped: false,
            work: PipelineBatchWork::default(),
            #[cfg(target_arch = "wasm32")]
            dispatched_keys: 0,
        }
    }
    pub(crate) fn push(&mut self, pair: &std::sync::Arc<P>) {
        assert!(self.open && self.len < PIPELINE_BATCH_KEYS);
        self.pairs[self.len] = std::sync::Arc::downgrade(pair);
        self.len += 1;
    }
    pub(crate) fn select(&mut self, keys: Option<usize>) -> Result<bool, ReferenceError> {
        let Some(keys) = keys else {
            self.stopped |= self.next != 0;
            return Ok(false);
        };
        match self.work.accept(keys) {
            Ok(true) => {
                self.next += 1;
                Ok(true)
            }
            Ok(false) => {
                self.stopped = true;
                Ok(false)
            }
            Err(error) => {
                self.stopped = true;
                Err(error)
            }
        }
    }
    pub(crate) fn register(&self, settled: impl Iterator<Item = bool>) -> bool {
        let drained = settled.fold(true, |all, pair| pair & all);
        (self.stopped || self.next == self.len) && drained && self.operation.physical.register()
    }
    pub(crate) fn retire_pairs(&mut self) {
        if self.operation.physical.completed() {
            for pair in &mut self.pairs[..self.len] {
                if pair.strong_count() <= 1 {
                    *pair = std::sync::Weak::new();
                }
            }
        }
    }
    pub(crate) fn retain(&self) -> bool {
        if !self.operation.physical.completed() {
            return true;
        }
        let mut receipt_owners = 1;
        for pair in &self.pairs[..self.len] {
            let owners = pair.strong_count();
            if owners > 1 {
                return true;
            }
            receipt_owners += usize::from(owners != 0);
        }
        // The parent can still retain the completed flight. Charge its control until release.
        self.operation.physical.owners() > receipt_owners
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct PipelineAccounting {
    pub(crate) jobs: usize,
    pub(crate) pending: usize,
    pub(crate) cpu: usize,
}
impl PipelineAccounting {
    pub(crate) fn admitted(&mut self, operation: &PipelineOperation) {
        self.jobs += 1;
        self.pending += usize::from(operation.owns_pending);
        self.cpu += UploadReceipt::allocation_bytes();
    }
    pub(crate) fn pair_admitted(&mut self, cpu: usize) {
        self.cpu += cpu;
    }
    pub(crate) fn reconcile(
        &mut self,
        stream: &mut WorldStream,
        operations: impl Iterator<Item = PipelineOperation>,
        cpu: usize,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let mut next = Self {
            cpu,
            ..Self::default()
        };
        for operation in operations {
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
        allocation.prototype_cpu =
            allocation.prototype_cpu.checked_sub(self.cpu).unwrap() + next.cpu;
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

#[cfg(test)]
mod batch_tests {
    use super::*;
    #[test]
    fn pipeline_diagnostics_preserve_short_text_and_bound_utf8_without_growth() {
        let short = pipeline_diagnostic(format_args!(
            "Source material pipeline: {}",
            "missing shader"
        ));
        assert_eq!(short, "Source material pipeline: missing shader");
        assert_eq!(short.capacity(), PIPELINE_DIAGNOSTIC_BYTES);
        let exact = "a".repeat(PIPELINE_DIAGNOSTIC_BYTES);
        assert_eq!(pipeline_diagnostic(format_args!("{exact}")), exact);
        let unicode = "🦀".repeat(PIPELINE_DIAGNOSTIC_BYTES);
        let long = pipeline_diagnostic(format_args!("Source material shader: {unicode}"));
        assert!(long.ends_with(" [truncated]"));
        assert!(long.len() <= PIPELINE_DIAGNOSTIC_BYTES);
        assert_eq!(long.capacity(), PIPELINE_DIAGNOSTIC_BYTES);
        let chunks = pipeline_diagnostic(format_args!("{exact}é{unicode}"));
        assert!(chunks.ends_with(" [truncated]"));
        assert_eq!(chunks.len(), PIPELINE_DIAGNOSTIC_BYTES);
        let misaligned = pipeline_diagnostic(format_args!("x{unicode}"));
        assert_eq!(
            misaligned,
            format!(
                "x{} [truncated]",
                "🦀".repeat((PIPELINE_DIAGNOSTIC_BYTES - 13) / 4)
            )
        );
        let previous = "€".repeat((PIPELINE_DIAGNOSTIC_BYTES - 1) / 3);
        let trimmed = pipeline_diagnostic(format_args!("{previous}é"));
        assert_eq!(
            trimmed,
            format!(
                "{} [truncated]",
                "€".repeat((PIPELINE_DIAGNOSTIC_BYTES - 12) / 3)
            )
        );
        assert_eq!(trimmed.capacity(), PIPELINE_DIAGNOSTIC_BYTES);
        let mut output = Some(long);
        let storage = output.as_ref().unwrap().as_ptr();
        write_pipeline_diagnostic(&mut output, format_args!("Source material shader: failure"));
        assert_eq!(output.as_ref().unwrap(), "Source material shader: failure");
        assert_eq!(output.as_ref().unwrap().as_ptr(), storage);
        assert_eq!(
            output.as_ref().unwrap().capacity(),
            PIPELINE_DIAGNOSTIC_BYTES
        );
    }
    #[test]
    fn failed_siblings_keep_admitted_diagnostics_until_shared_drain_and_first_error() {
        use std::sync::{Arc, Mutex};
        struct Pair {
            receipt: UploadReceipt,
            error: Option<String>,
        }
        let operation = PipelineOperation {
            owns_pending: false,
            physical: UploadReceipt::default(),
        };
        let mut batch = PipelineBatch::new(operation);
        let pairs: Vec<_> = (0..3)
            .map(|_| {
                Arc::new(Mutex::new(Pair {
                    receipt: batch.operation.physical.clone(),
                    error: None,
                }))
            })
            .collect();
        for pair in &pairs {
            batch.push(pair);
            assert!(batch.select(Some(1)).unwrap());
        }
        batch.open = false;
        batch.stopped = true;
        write_pipeline_diagnostic(
            &mut pairs[2].lock().unwrap().error,
            format_args!("later failure"),
        );
        assert!(
            !batch.register(
                pairs
                    .iter()
                    .map(|pair| pair.lock().unwrap().error.is_some())
            )
        );
        for index in 0..2 {
            write_pipeline_diagnostic(
                &mut pairs[index].lock().unwrap().error,
                format_args!("failure {index}"),
            );
        }
        let retained: usize = pairs
            .iter()
            .map(|pair| pair.lock().unwrap().error.as_ref().unwrap().capacity())
            .sum();
        assert_eq!(retained, 3 * PIPELINE_DIAGNOSTIC_BYTES);
        assert!(
            !batch.register([true, false, true].into_iter()),
            "a companion still owns the batch"
        );
        assert!(
            batch.register(
                pairs
                    .iter()
                    .map(|pair| pair.lock().unwrap().error.is_some())
            )
        );
        assert!(
            pairs
                .iter()
                .all(|pair| !pipeline_pair_ready(&pair.lock().unwrap().receipt, false))
        );
        batch.operation.physical.complete();
        let first = pairs.iter().find_map(|pair| {
            let pair = pair.lock().unwrap();
            pipeline_pair_ready(&pair.receipt, false)
                .then(|| pair.error.clone())
                .flatten()
        });
        assert_eq!(first.as_deref(), Some("failure 0"));
        assert_eq!(
            pairs
                .iter()
                .map(|pair| pair.lock().unwrap().error.as_ref().unwrap().capacity())
                .sum::<usize>(),
            retained
        );
    }
    #[test]
    fn whole_pair_prefix_spills_without_consuming_the_next_pair() {
        let mut work = PipelineBatchWork::default();
        assert!(work.accept(7).unwrap());
        assert!(work.accept(9).unwrap());
        assert!(!work.accept(17).unwrap());
        assert_eq!((work.pairs, work.base_keys), (2, 16));
        let mut next = PipelineBatchWork::default();
        assert!(next.accept(17).unwrap());
        assert!(!next.accept(32).unwrap());
        let mut full = PipelineBatchWork::default();
        assert!(full.accept(32).unwrap());
        assert_eq!(full.base_keys, 32);
        assert!(PipelineBatchWork::default().accept(33).is_err());
        assert!(PipelineBatchWork::default().accept(0).is_err());
    }
    #[test]
    fn frozen_keys_preserve_order_and_variants_against_the_previous_vec_path() {
        for material_bits in [0, 4, 8] {
            for shadows in [false, true] {
                for discard in [false, true] {
                    let mut old = Vec::new();
                    let mut keys = PipelineKeys::new();
                    for view in [1u64, 2, 1, 16] {
                        let key = (view | material_bits | 32, false);
                        if !old.contains(&key) {
                            old.push(key);
                        }
                        keys.push(key).unwrap();
                    }
                    if shadows {
                        for light in [64, 128] {
                            let key = (light | 32 | if discard { 256 } else { 0 }, true);
                            if !old.contains(&key) {
                                old.push(key);
                            }
                            keys.push(key).unwrap();
                        }
                    }
                    assert_eq!(keys.iter().collect::<Vec<_>>(), old);
                }
            }
        }
        let mut full = PipelineKeys::new();
        for key in 0..32 {
            full.push((key, false)).unwrap();
        }
        full.push((31, false)).unwrap();
        assert_eq!(full.len, 32);
        assert!(full.push((32, false)).is_err());
    }
    #[test]
    fn missing_inputs_seal_only_a_nonempty_prefix_and_spills_retry_with_a_full_budget() {
        use std::sync::Arc;
        let operation = || PipelineOperation {
            owns_pending: false,
            physical: UploadReceipt::default(),
        };
        let mut batch = PipelineBatch::new(operation());
        let pair = Arc::new(());
        batch.push(&pair);
        batch.push(&pair);
        batch.push(&pair);
        batch.open = false;
        assert!(!batch.select(None).unwrap());
        assert!(!batch.stopped);
        assert!(!batch.register(std::iter::empty()));
        assert!(batch.select(Some(8)).unwrap());
        assert!(batch.select(Some(16)).unwrap());
        assert!(!batch.select(Some(16)).unwrap());
        assert!(batch.stopped);
        assert_eq!((batch.next, batch.work.base_keys), (2, 24));
        assert!(!batch.register([true, false].into_iter()));
        assert!(batch.register([true, true].into_iter()));
        assert!(!pipeline_pair_ready(&batch.operation.physical, true));
        batch.operation.physical.complete();
        let mut retry = PipelineBatch::new(operation());
        retry.push(&pair);
        retry.open = false;
        assert!(retry.select(Some(16)).unwrap());
        assert_eq!(retry.next, retry.len);
        let mut waiting = PipelineBatch::new(operation());
        waiting.push(&pair);
        waiting.push(&pair);
        waiting.open = false;
        assert!(waiting.select(Some(4)).unwrap());
        assert!(!waiting.select(None).unwrap());
        assert!(waiting.stopped);
        assert_eq!(waiting.work.base_keys, 4);
    }
    #[test]
    fn shared_receipt_withholds_errors_until_all_siblings_drain() {
        use std::sync::Arc;
        let operation = PipelineOperation {
            owns_pending: false,
            physical: UploadReceipt::default(),
        };
        let mut batch = PipelineBatch::new(operation);
        let first = Arc::new(batch.operation.physical.clone());
        let first_user = first.clone();
        let second = Arc::new(batch.operation.physical.clone());
        let second_user = second.clone();
        batch.push(&first);
        batch.push(&second);
        assert!(batch.work.accept(3).unwrap());
        assert!(batch.work.accept(4).unwrap());
        batch.next = 2;
        batch.open = false;
        let first_error = |errors: [Option<&'static str>; 2]| {
            [(&first, errors[0]), (&second, errors[1])]
                .into_iter()
                .find_map(|(receipt, error)| {
                    if pipeline_pair_ready(receipt, false) {
                        error
                    } else {
                        None
                    }
                })
        };
        assert_eq!(first_error([None, Some("later failure")]), None);
        // Later failure cannot expose an error while an earlier shader or companion waits.
        assert!(!batch.register([false, true].into_iter()));
        assert!(!pipeline_pair_ready(&first, false));
        assert!(!pipeline_pair_ready(&second, false));
        assert!(batch.retain());
        assert!(batch.register([true, true].into_iter()));
        assert!(!batch.register([true, true].into_iter()));
        assert!(!pipeline_pair_ready(&first, false));
        // This is the native receipt adapter. Render uses only the queue callback here.
        batch.operation.physical.complete();
        assert!(pipeline_pair_ready(&first, false));
        assert_eq!(
            first_error([Some("earlier failure"), Some("later failure")]),
            Some("earlier failure")
        );
        assert!(!pipeline_pair_ready(&first, true));
        drop(first_user);
        assert!(batch.retain());
        batch.retire_pairs();
        assert_eq!(Arc::weak_count(&first), 0);
        assert_eq!(Arc::weak_count(&second), 1);
        drop(first);
        drop(second_user);
        assert!(
            !batch.retain(),
            "Weak controls drop before bank pair charges release"
        );
    }
    #[test]
    fn stopped_flights_still_require_one_real_completion_and_keep_upload_ownership() {
        for stop_at in 0..3 {
            let receipt = UploadReceipt::default();
            if stop_at == 0 {
                receipt.stop();
                receipt.stop();
            }
            assert!(receipt.register());
            assert!(!receipt.register());
            if stop_at == 1 {
                receipt.stop();
            }
            assert!(!receipt.completed());
            assert_eq!(receipt.cpu_bytes(123, true), 123);
            receipt.complete();
            if stop_at == 2 {
                receipt.stop();
            }
            assert!(receipt.completed());
            assert!(receipt.stopped());
            assert!(!receipt.register());
            assert_eq!(receipt.cpu_bytes(123, true), 0);
            assert!(!pipeline_pair_ready(&receipt, false));
        }
        let request = WorldRequest {
            session: 7,
            revision: 1,
            player: [0.; 3],
            vehicle: [0.; 3],
        };
        let mut parent = PipelineContinuation::counted(RendererOwner::Grass(request, 9));
        let upload = UploadReceipt::default();
        parent.watch_upload(upload.clone());
        let flight = UploadReceipt::default();
        parent.flight = Some(flight.clone());
        parent.stop_flight();
        assert!(flight.stopped());
        assert!(!upload.stopped());
        assert!(!parent.drained());
        flight.complete();
        assert!(!parent.drained());
        upload.complete();
        assert!(parent.drained());
    }
    #[test]
    fn growth_admits_the_full_new_buffer_before_old_storage_is_released() {
        let growth = PipelineBufferGrowth::new(8, 9, 24);
        assert_eq!(
            (growth.capacity, growth.admitted, growth.released),
            (16, 384, 192)
        );
        let same = PipelineBufferGrowth::new(16, 9, 24);
        assert_eq!((same.capacity, same.admitted, same.released), (16, 0, 0));
    }
    #[test]
    #[ignore = "requires the installed source export"]
    fn batch_admission_and_reconciliation_keep_global_limits_and_parent_claims() {
        let reference = crate::reference::ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let mut stream = WorldStream::new_default(&reference, 42).unwrap();
        for _ in 0..2000 {
            if stream.available_external_jobs() > 0 {
                break;
            }
            stream.prepare(100.);
        }
        assert!(
            stream.available_external_jobs() > 0,
            "source frontiers precede external work"
        );
        let base = stream.presentation_allocation();
        let mut one_busy = base;
        one_busy.jobs += stream.available_external_jobs().saturating_sub(1);
        stream.set_presentation_allocation(one_busy).unwrap();
        let staging = 64 * 1024;
        let receipt_bytes = UploadReceipt::allocation_bytes();
        let pair_cpu = 1024 + PIPELINE_DIAGNOSTIC_BYTES;
        let mut parent = stream.reserve_grass_renderer(9, 0).unwrap();
        let operation = parent
            .start(&mut stream, receipt_bytes + pair_cpu, staging)
            .unwrap()
            .unwrap();
        let mut accounting = PipelineAccounting::default();
        accounting.admitted(&operation);
        accounting.pair_admitted(pair_cpu);
        operation.reserve_pair(&mut stream, pair_cpu).unwrap();
        accounting.pair_admitted(pair_cpu);
        assert_eq!(stream.available_external_jobs(), 0);
        assert_eq!(stream.presentation_allocation().pending, base.pending + 1);
        assert!(parent.shares(&operation.physical));
        assert!(parent.start(&mut stream, 0, staging).unwrap().is_none());
        assert!(
            PipelineOperation::independent(&mut stream, 0, staging)
                .unwrap()
                .is_none()
        );
        accounting
            .reconcile(
                &mut stream,
                std::iter::once(operation.clone()),
                receipt_bytes + 2 * pair_cpu,
                staging,
            )
            .unwrap();
        assert_eq!(stream.presentation_allocation().jobs, 2);
        assert_eq!(
            stream.presentation_allocation().staging_bytes,
            base.staging_bytes + staging
        );
        assert!(parent.release().is_err());
        operation.physical.complete();
        accounting
            .reconcile(
                &mut stream,
                std::iter::once(operation.clone()),
                receipt_bytes + 2 * pair_cpu,
                staging,
            )
            .unwrap();
        assert_eq!(stream.available_external_jobs(), 1);
        assert_eq!(
            stream.presentation_allocation().staging_bytes,
            base.staging_bytes
        );
        assert_eq!(
            stream.presentation_allocation().prototype_cpu,
            base.prototype_cpu + receipt_bytes + 2 * pair_cpu
        );
        stream.release_grass_renderer(9, &mut parent).unwrap();
        assert!(parent.validate(&stream).is_err());
        drop(parent);
        drop(operation);
        accounting
            .reconcile(&mut stream, std::iter::empty(), 0, staging)
            .unwrap();
        assert_eq!(
            stream.presentation_allocation().prototype_cpu,
            base.prototype_cpu
        );
        stream.set_presentation_allocation(base).unwrap();
        let mut full = base;
        full.pending = 32;
        stream.set_presentation_allocation(full).unwrap();
        assert!(
            PipelineOperation::independent(&mut stream, 0, staging)
                .unwrap()
                .is_none()
        );
        let mut counted =
            PipelineContinuation::counted(RendererOwner::Grass(stream.frame().request, 10));
        let counted_op = counted.start(&mut stream, 0, staging).unwrap().unwrap();
        assert_eq!(stream.presentation_allocation().pending, 32);
        counted_op.physical.complete();
        stream.set_presentation_allocation(base).unwrap();
        let mut stale_request = stream.frame().request;
        stale_request.session += 1;
        let stale = PipelineContinuation::counted(RendererOwner::Grass(stale_request, 11));
        assert!(stale.validate(&stream).is_err());
        let before = stream.presentation_allocation();
        assert!(PipelineOperation::independent(&mut stream, usize::MAX / 2, staging).is_err());
        assert_eq!(
            stream.presentation_allocation().prototype_cpu,
            before.prototype_cpu
        );
        assert_eq!(stream.presentation_allocation().jobs, before.jobs);
        assert!(PipelineOperation::independent(&mut stream, 0, 32 * 1024 * 1024 + 1).is_err());
        assert_eq!(
            stream.presentation_allocation().staging_bytes,
            before.staging_bytes
        );
        let diagnostic_budget = stream
            .presentation_budgets(before)
            .into_iter()
            .find(|b| b.name == "Asset CPU")
            .unwrap();
        let mut diagnostic_tight = before;
        diagnostic_tight.prototype_cpu +=
            diagnostic_budget.limit - diagnostic_budget.used - PIPELINE_DIAGNOSTIC_BYTES + 1;
        stream
            .set_presentation_allocation(diagnostic_tight)
            .unwrap();
        let mut formatted = false;
        let rejected = PipelineOperation::independent(
            &mut stream,
            PIPELINE_DIAGNOSTIC_BYTES,
            staging,
        )
        .map(|operation| {
            operation.map(|_| {
                formatted = true;
                pipeline_diagnostic(format_args!("must not allocate after rejected admission"))
            })
        });
        assert!(rejected.is_err());
        assert!(!formatted);
        assert_eq!(
            stream.presentation_allocation().prototype_cpu,
            diagnostic_tight.prototype_cpu
        );
        stream.set_presentation_allocation(before).unwrap();
        let growth = PipelineBufferGrowth::new(8, 9, 24);
        let budget = stream
            .presentation_budgets(before)
            .into_iter()
            .find(|b| b.name == "Asset CPU")
            .unwrap();
        let mut tight = before;
        tight.prototype_cpu += budget.limit - budget.used - growth.admitted + 1;
        stream.set_presentation_allocation(tight).unwrap();
        assert!(PipelineOperation::independent(&mut stream, growth.admitted, staging).is_err());
        assert_eq!(
            stream.presentation_allocation().prototype_cpu,
            tight.prototype_cpu
        );
        // The net size would fit. The full new buffer must fail while the old buffer exists.
        let mut net = tight;
        net.prototype_cpu += growth.admitted - growth.released;
        stream.set_presentation_allocation(net).unwrap();
        stream.set_presentation_allocation(before).unwrap();
        // A candidate cannot subtract active charges when it checks overlap.
        let mut active = before;
        let staging_budget = stream
            .presentation_budgets(before)
            .into_iter()
            .find(|b| b.name == "Staging")
            .unwrap();
        active.staging_bytes += staging_budget.limit - staging_budget.used - staging + 1;
        stream.set_presentation_allocation(active).unwrap();
        assert!(PipelineOperation::independent(&mut stream, 0, staging).is_err());
        assert_eq!(
            stream.presentation_allocation().staging_bytes,
            active.staging_bytes
        );
    }
}
