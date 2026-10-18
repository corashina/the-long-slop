//! Contiguous motion runs share an authored mesh and one GPU buffer binding.
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionInstance {
    pub index: u32,
    pub dynamic_offset: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MotionRun<M, E> {
    pub mesh: M,
    pub first: E,
    pub instances: Range<u32>,
    pub dynamic_offset: Option<u32>,
}

pub struct MotionRunPlanner<M, E> {
    pending: Option<MotionRun<M, E>>,
}
impl<M: Eq, E> MotionRunPlanner<M, E> {
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// A missing instance ends the run, even when packed GPU indices remain adjacent.
    pub fn push(&mut self, entry: Option<(M, E, MotionInstance)>) -> Option<MotionRun<M, E>> {
        let Some((mesh, entity, instance)) = entry else {
            return self.finish();
        };
        if let Some(run) = self.pending.as_mut()
            && run.mesh == mesh
            && run.dynamic_offset == instance.dynamic_offset
            && run.instances.end == instance.index
        {
            run.instances.end += 1;
            return None;
        }
        self.pending.replace(MotionRun {
            mesh,
            first: entity,
            instances: instance.index..instance.index + 1,
            dynamic_offset: instance.dynamic_offset,
        })
    }

    pub fn finish(&mut self) -> Option<MotionRun<M, E>> {
        self.pending.take()
    }
}
