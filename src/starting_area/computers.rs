//! Computer and TV actions of the player (fpscontroller look-at and held-item code).
use super::GarageRuntime;
use crate::interaction::AimTarget;
use crate::interaction::feedback::ComputerPrompt;
use crate::reference::ItemId;
use crate::simulation::{FIXED_DT, FrameInput, keys};

impl GarageRuntime {
    /// inputscript while usingComputer: the game reads no other input, and LMB ends typing.
    pub(super) fn step_typing(&mut self, input: &mut FrameInput) {
        let blocked = self.needs_output().dead || self.sleeping();
        let computers = &mut self.stream.computers;
        if computers.typing {
            if blocked || input.pressed & keys::PRIMARY != 0 {
                computers.typing = false;
            }
            *input = FrameInput::default();
        }
        computers.advance(FIXED_DT, None);
    }
    /// The aimed item within reach.
    fn looked_at(&self) -> Option<ItemId> {
        let reach = self.area.interaction.reach();
        self.aim
            .focus_target
            .filter(|t: &AimTarget| t.distance.is_finite() && (0. ..=reach).contains(&t.distance))
            .map(|t| t.item)
    }
    /// R turns the aimed computer on or off and LMB uses it. E connects or disconnects a
    /// held computer.
    pub(super) fn computer_actions(&mut self, input: FrameInput) {
        let looked = self.looked_at();
        let computers = &mut self.stream.computers;
        if let Some(item) = looked
            && let Some(b) = computers.computer(item)
        {
            if b.computer.on && input.pressed & keys::PRIMARY != 0 {
                computers.use_computer(item);
            }
            if input.pressed & keys::RELOAD != 0 {
                computers.turn(item);
            }
        }
        if let Some(held) = self.area.interaction.held()
            && let Some(b) = computers.computer(held)
            && input.pressed & keys::USE != 0
        {
            if b.display.is_some() {
                computers.disconnect(held);
            } else if let Some(display) = looked.filter(|d| computers.is_display(*d)) {
                computers.connect(held, Some(display));
            }
        }
    }
    /// `ComputerPrompt` bits of the aimed and held computers.
    pub fn computer_prompt(&self) -> u8 {
        let computers = &self.stream.computers;
        let looked = self.looked_at();
        let mut bits = 0;
        if computers.typing {
            bits |= ComputerPrompt::TYPING;
        }
        if let Some(b) = looked.and_then(|item| computers.computer(item)) {
            bits |= if b.computer.on {
                ComputerPrompt::TURN_OFF | ComputerPrompt::USE
            } else {
                ComputerPrompt::TURN_ON
            };
        }
        if let Some(b) = self
            .area
            .interaction
            .held()
            .and_then(|h| computers.computer(h))
        {
            if b.display.is_some() {
                bits |= ComputerPrompt::DISCONNECT;
            } else if looked.is_some_and(|d| computers.is_display(d)) {
                bits |= ComputerPrompt::CONNECT;
            }
        }
        bits
    }
    /// mainscript.PlayClipAtPoint with a random pitch for the display switch.
    pub(super) fn play_computer_sounds(&mut self) {
        for (clip, gain, position) in self.stream.computers.sounds.drain(..) {
            if let Some(audio) = &mut self.audio {
                let pitch = audio.random_pitch();
                audio.survival_event(crate::audio::SurvivalAudioEvent {
                    session: audio.session,
                    emitter: 0,
                    kind: crate::audio::AudioKind::ItemHit,
                    action: crate::audio::AudioAction::Start,
                    clip,
                    gain,
                    pitch,
                    position,
                });
            }
        }
    }
}
