//! One simulation behind encoded commands and frames. The worker module runs it.
use crate::boundary::{DecodeError, SimInit, SimStartup, decode_command, encode};
use crate::driver::SimDriver;
use crate::reference::{ReferenceError, ReferenceManifest};
use crate::save::SourceStamp;
use crate::starting_area::GarageRuntime;
use std::sync::Arc;

pub struct Session {
    reference: Arc<ReferenceManifest>,
    driver: SimDriver,
    /// The frame of the last tick. The next tick fills it again.
    frame: crate::boundary::SimFrame,
    bytes: Vec<u8>,
}

impl Session {
    pub fn new(
        reference: Arc<ReferenceManifest>,
        init: SimInit,
    ) -> Result<(Self, SimStartup), ReferenceError> {
        let runtime = GarageRuntime::new(&reference, init.mode, init.draw_distance)
            .map_err(|e| ReferenceError(e.to_string()))?;
        let source = SourceStamp::new(&reference, init.manifest_sha256);
        let driver = SimDriver::new(runtime, source);
        let startup = driver.startup();
        let session = Self {
            reference,
            driver,
            frame: Default::default(),
            bytes: Vec::new(),
        };
        Ok((session, startup))
    }

    /// Runs one encoded command. Returns the encoded frame.
    pub fn tick(&mut self, command: &[u8]) -> Result<&[u8], DecodeError> {
        let command = decode_command(command)?;
        self.driver.tick(&self.reference, command, &mut self.frame);
        encode(&self.frame, &mut self.bytes);
        Ok(&self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boundary::{SimCommand, SimFrame, decode, encode_command};

    #[test]
    #[ignore = "requires the installed source export"]
    fn tick_answers_a_decodable_frame() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/assets/reference/manifest.json"
        ))
        .unwrap();
        let reference = Arc::new(ReferenceManifest::from_runtime_json(&bytes).unwrap());
        let init = SimInit {
            mode: Default::default(),
            draw_distance: reference.world.draw_distance.default,
            manifest_sha256: String::new(),
        };
        let (mut session, startup) = Session::new(reference, init).unwrap();
        let mut command = Vec::new();
        encode_command(
            &SimCommand {
                seq: 1,
                delta_seconds: 1. / 60.,
                ..Default::default()
            },
            &mut command,
        );
        let mut frame = SimFrame::default();
        decode(session.tick(&command).unwrap(), &mut frame).unwrap();
        assert_eq!(frame.seq_consumed, 1);
        assert_eq!(frame.state.session, startup.session);
    }
}
