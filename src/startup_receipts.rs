//! Two completed renderer receipts after startup readiness.

#[derive(Default)]
pub(crate) struct StartupFrames {
    scene: u32,
    after_token: u32,
    tokens: [u32; 2],
    generations: [u32; 2],
    count: usize,
    armed: bool,
}

impl StartupFrames {
    pub(crate) fn arm(&mut self, scene: u32, after_token: u32) {
        *self = Self {
            scene,
            after_token,
            armed: true,
            ..Self::default()
        };
    }

    pub(crate) fn accepts(&self, scene: u32, token: u32) -> bool {
        self.armed
            && self.scene == scene
            && self.count < 2
            && token > self.after_token
            && (self.count == 0 || token > self.tokens[self.count - 1])
    }

    pub(crate) fn record(&mut self, scene: u32, token: u32, generation: u32) {
        assert!(self.accepts(scene, token));
        self.tokens[self.count] = token;
        self.generations[self.count] = generation;
        self.count += 1;
    }

    pub(crate) fn validated_count(&self, scene: u32, validated: u32, healthy: bool) -> u32 {
        if !self.armed || self.scene != scene || !healthy {
            return 0;
        }
        self.generations[..self.count]
            .iter()
            .take_while(|generation| **generation != 0 && validated >= **generation)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::StartupFrames;

    #[test]
    fn counts_only_validated_post_ready_frames_in_the_current_scene() {
        let mut frames = StartupFrames::default();
        frames.arm(3, 7); // Token 7 was prepared before readiness but is still in flight.
        assert!(!frames.accepts(3, 7));
        frames.record(3, 8, 1);
        frames.record(3, 9, 2);
        assert!(!frames.accepts(3, 10));
        assert_eq!(frames.validated_count(3, 0, true), 0); // Both CPU receipts; GPU pending.
        assert_eq!(frames.validated_count(3, 1, true), 1);
        assert_eq!(frames.validated_count(3, 2, true), 2);
        assert_eq!(frames.validated_count(3, 2, false), 0); // Rejected after error poll.
        assert_eq!(frames.validated_count(4, 2, true), 0); // Stale scene.
        frames.arm(4, 9);
        assert_eq!(frames.validated_count(4, 2, true), 0);
    }
}
