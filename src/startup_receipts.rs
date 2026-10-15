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
