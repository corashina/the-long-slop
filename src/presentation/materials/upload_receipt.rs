use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

/// Shared by material forks, CPU accounting, and the render queue callback.
#[derive(Clone, Default)]
pub(crate) struct UploadReceipt(Arc<AtomicU8>);

impl UploadReceipt {
    pub fn cpu_bytes(&self, reservation: usize, started: bool) -> usize {
        if started && self.0.load(Ordering::Acquire) != 2 {
            reservation
        } else {
            0
        }
    }

    pub fn completed(&self) -> bool { self.0.load(Ordering::Acquire) == 2 }

    pub fn register(&self) -> bool {
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn complete(&self) {
        self.0.store(2, Ordering::Release);
    }
}
