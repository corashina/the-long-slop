//! The renderer end of the simulation. One command is in flight; newer commands merge into the next.
use simulation::boundary::{DecodeError, SimCommand, SimFrame, decode, encode_command};
use simulation::session::Session;

pub(crate) struct SimLink {
    session: Session,
    /// A sent command has no frame yet.
    in_flight: bool,
    /// Commands since the last send, merged.
    pending: Option<SimCommand>,
    command: Vec<u8>,
    /// The frame of the command in flight, when it is ready.
    answer: Option<Vec<u8>>,
}

impl SimLink {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            in_flight: false,
            pending: None,
            command: Vec::new(),
            answer: None,
        }
    }

    pub(crate) fn send(&mut self, command: SimCommand) -> Result<(), DecodeError> {
        match &mut self.pending {
            Some(pending) => pending.merge(command),
            None => self.pending = Some(command),
        }
        self.flush()
    }

    /// Reads the frame of the command in flight into `frame`. False: no new frame.
    pub(crate) fn receive(&mut self, frame: &mut SimFrame) -> Result<bool, DecodeError> {
        let Some(answer) = self.answer.as_ref().filter(|_| self.in_flight) else {
            return Ok(false);
        };
        decode(answer, frame)?;
        self.in_flight = false;
        self.flush()?;
        Ok(true)
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        if self.in_flight {
            return Ok(());
        }
        let Some(command) = self.pending.take() else {
            return Ok(());
        };
        encode_command(&command, &mut self.command);
        let frame = self.session.tick(&self.command)?;
        let answer = self.answer.get_or_insert_default();
        answer.clear();
        answer.extend_from_slice(frame);
        self.in_flight = true;
        Ok(())
    }
}
