//! Initial scene cadence and complete-scene capture proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct LoadingKey {
    pub session: u64,
    pub scene: u32,
    pub extent: [u32; 2],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
enum Mode {
    #[default]
    Cadenced,
    Full,
    Bypass,
}
#[derive(Clone, Copy, Debug)]
pub struct LoadingDispatch<const N: usize> {
    mode: Mode,
    key: Option<LoadingKey>,
    decision_frame: u32,
    content: Option<(u64, u32, u32)>,
    capture_start: [u32; N],
    published_start: [u32; N],
}
impl<const N: usize> Default for LoadingDispatch<N> {
    fn default() -> Self {
        Self {
            mode: Mode::Cadenced,
            key: None,
            decision_frame: 0,
            content: None,
            capture_start: [0; N],
            published_start: [0; N],
        }
    }
}
impl<const N: usize> LoadingDispatch<N> {
    pub fn snapshot(
        &mut self,
        key: LoadingKey,
        frame: u32,
        bypass: bool,
        content: bool,
        revision: u64,
        enabled: u32,
    ) {
        if self.key != Some(key) {
            self.key = Some(key);
            self.content = None;
            self.capture_start = [0; N];
            self.published_start = [0; N];
            if self.mode != Mode::Bypass {
                self.mode = Mode::Cadenced;
            }
        }
        self.decision_frame = frame;
        if bypass {
            self.mode = Mode::Bypass;
            self.content = None;
            return;
        }
        if self.mode == Mode::Bypass {
            return;
        }
        if !content {
            self.content = None;
            return;
        }
        self.mode = Mode::Full;
        if self
            .content
            .is_none_or(|(r, mask, _)| r != revision || mask != enabled)
        {
            self.content = Some((revision, enabled, frame));
        }
    }
    pub fn allows(&self, key: LoadingKey, frame: u32) -> bool {
        self.mode != Mode::Cadenced
            || self.key != Some(key)
            || self.decision_frame != frame
            || frame % 4 == 0
    }
    pub fn invalidate(&mut self) {
        self.content = None;
    }
    pub fn capture_started(&mut self, key: LoadingKey, frame: u32, index: usize) {
        if self.key == Some(key) {
            self.capture_start[index] = frame;
        }
    }
    pub fn publication_validated(&mut self, key: LoadingKey, index: usize) {
        if self.key == Some(key) {
            self.published_start[index] = self.capture_start[index];
        }
    }
    pub fn current_fresh(&self) -> bool {
        match self.mode {
            Mode::Cadenced | Mode::Bypass => true,
            Mode::Full => self
                .key
                .zip(self.content)
                .is_some_and(|(key, (revision, enabled, _))| self.fresh(key, revision, enabled)),
        }
    }
    pub fn fresh(&self, key: LoadingKey, revision: u64, enabled: u32) -> bool {
        if self.key != Some(key) {
            return false;
        }
        if self.mode == Mode::Bypass {
            return true;
        }
        self.mode == Mode::Full
            && self.key == Some(key)
            && self.content.is_some_and(|(r, mask, floor)| {
                r == revision
                    && mask == enabled
                    && self.published_start.iter().enumerate().all(|(i, start)| {
                        enabled & (1 << i) == 0 || (*start != 0 && *start >= floor)
                    })
            })
    }
}
impl<const N: usize> serde::Serialize for LoadingDispatch<N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut audit = serializer.serialize_struct("LoadingDispatch", 9)?;
        audit.serialize_field("mode", &self.mode)?;
        audit.serialize_field("key", &self.key)?;
        audit.serialize_field("decision_frame", &self.decision_frame)?;
        audit.serialize_field("required_revision", &self.content.map(|v| v.0))?;
        audit.serialize_field("enabled_mask", &self.content.map(|v| v.1))?;
        audit.serialize_field("minimum_capture_frame", &self.content.map(|v| v.2))?;
        audit.serialize_field("capture_start", &self.capture_start.as_slice())?;
        audit.serialize_field("published_start", &self.published_start.as_slice())?;
        audit.serialize_field(
            "fresh_complete_scene",
            &self
                .key
                .zip(self.content)
                .is_some_and(|(key, (revision, enabled, _))| self.fresh(key, revision, enabled)),
        )?;
        audit.end()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> LoadingKey {
        LoadingKey {
            session: 1,
            scene: 1,
            extent: [800, 600],
        }
    }
    #[test]
    fn every_probe_stage_waits_for_three_skipped_snapshots() {
        use crate::reference::reflections::{ProbeJob, ProbeSchedule};
        let k = key();
        let mut policy = LoadingDispatch::<2>::default();
        let mut schedule = ProbeSchedule::default();
        for stage in 0..8 {
            let job = schedule.job();
            assert_eq!(
                job,
                match stage {
                    0..=5 => ProbeJob::Face(stage),
                    6 => ProbeJob::FirstMip,
                    _ => ProbeJob::RemainingMipsAndPublish,
                }
            );
            for offset in 1..=3 {
                let frame = u32::from(stage) * 4 + offset;
                policy.snapshot(k, frame, false, false, 1, 3);
                assert!(!policy.allows(k, frame));
                assert_eq!(schedule.job(), job);
            }
            let frame = u32::from(stage + 1) * 4;
            policy.snapshot(k, frame, false, false, 1, 3);
            assert!(policy.allows(k, frame));
            schedule.complete(job).unwrap();
        }
        assert!(schedule.published());
    }
    #[test]
    fn keys_masks_failures_and_later_presentations_reset_the_proof() {
        use crate::save::fresh_permit::PresentedFrames;
        let k = key();
        let mut p = LoadingDispatch::<2>::default();
        p.snapshot(k, 4, false, true, 1, 1);
        p.capture_started(k, 4, 0);
        p.publication_validated(k, 0);
        assert!(p.fresh(k, 1, 1));
        let mut frames = PresentedFrames::default();
        assert!(!frames.observe(k.session, k.scene, 8, 8));
        assert!(!frames.observe(k.session, k.scene, 8, 8));
        assert!(!frames.observe(k.session, k.scene, 9, 9));
        assert!(frames.observe(k.session, k.scene, 10, 10));
        p.snapshot(k, 5, false, true, 1, 3);
        assert!(!p.fresh(k, 1, 3));
        p.capture_started(k, 5, 0);
        p.publication_validated(k, 0);
        p.capture_started(k, 6, 1);
        p.publication_validated(k, 1);
        assert!(p.fresh(k, 1, 3));
        let resized = LoadingKey {
            extent: [900, 600],
            ..k
        };
        assert!(!p.fresh(resized, 1, 3));
        assert!(p.allows(resized, 7));
        p.snapshot(resized, 7, false, true, 1, 3);
        assert!(!p.fresh(resized, 1, 3));
        p.capture_started(k, 8, 0);
        p.publication_validated(k, 0);
        assert!(!p.fresh(resized, 1, 3));
        p.snapshot(resized, 8, false, true, 1, 0);
        assert!(p.fresh(resized, 1, 0));
        p.invalidate();
        assert!(!p.current_fresh());
        assert!(p.allows(resized, 9));
        for next in [
            LoadingKey {
                scene: 2,
                ..resized
            },
            LoadingKey {
                session: 2,
                ..resized
            },
        ] {
            p.snapshot(next, 9, false, false, 1, 3);
            assert!(!p.fresh(next, 1, 3));
            assert!(!p.allows(next, 9));
        }
    }
    #[test]
    fn loading_cadence_and_fresh_publications_use_snapshot_frames() {
        let k = key();
        let mut p = LoadingDispatch::<2>::default();
        for frame in 1..=4 {
            p.snapshot(k, frame, false, false, 2, 3);
            assert_eq!(p.allows(k, frame), frame == 4);
            assert!(p.allows(k, frame + 1));
        }
        p.capture_started(k, 4, 0);
        p.snapshot(k, 5, false, true, 2, 3);
        assert!(p.allows(k, 5));
        p.publication_validated(k, 0);
        assert!(!p.fresh(k, 2, 3));
        p.capture_started(k, 6, 0);
        p.publication_validated(k, 0);
        assert!(!p.fresh(k, 2, 3));
        p.capture_started(k, 7, 1);
        p.publication_validated(k, 1);
        assert!(p.fresh(k, 2, 3));
        p.snapshot(k, 8, false, false, 2, 3);
        assert!(p.allows(k, 8));
        assert!(!p.fresh(k, 2, 3));
        p.snapshot(k, 9, false, false, 2, 3);
        assert!(p.allows(k, 9));
        p.snapshot(k, 10, false, true, 3, 1);
        assert!(!p.fresh(k, 3, 1));
        p.capture_started(k, 10, 0);
        p.publication_validated(k, 0);
        assert!(p.fresh(k, 3, 1));
        p.invalidate();
        assert!(!p.fresh(k, 3, 1));
        p.snapshot(k, 11, true, false, 3, 1);
        let resized = LoadingKey {
            extent: [900, 600],
            ..k
        };
        p.snapshot(resized, 12, false, false, 3, 1);
        assert!(p.allows(resized, 12));
        assert!(p.fresh(resized, 3, 1));
    }
}
