//! Initial scene cadence and complete-scene capture proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadingKey {
    pub session: u64,
    pub scene: u32,
    pub extent: [u32; 2],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    pub fn fresh(&self, key: LoadingKey, revision: u64, enabled: u32) -> bool {
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
        p.snapshot(k, 9, false, true, 3, 1);
        assert!(!p.fresh(k, 3, 1));
        p.capture_started(k, 9, 0);
        p.publication_validated(k, 0);
        assert!(p.fresh(k, 3, 1));
        p.invalidate();
        assert!(!p.fresh(k, 3, 1));
        p.snapshot(k, 10, true, false, 3, 1);
        let resized = LoadingKey {
            extent: [900, 600],
            ..k
        };
        p.snapshot(resized, 11, false, false, 3, 1);
        assert!(p.allows(resized, 11));
        assert!(p.fresh(resized, 3, 1));
    }
}
