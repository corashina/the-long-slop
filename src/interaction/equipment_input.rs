//! Source Tab timing and right-hand release gates.
use crate::simulation::{FrameInput, keys};
#[derive(Clone, Copy)]
pub(crate) struct EquipmentContext {
    pub carried_can_equip: bool,
    pub has_carry: bool,
    pub has_equipment: bool,
    pub seated: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EquipAction {
    Equip,
    Recenter,
}
pub(crate) struct EquipmentInput {
    hold_seconds: f32,
    elapsed: f32,
    pressed: bool,
    can_equip: bool,
    can_recenter: bool,
    equipped_now: bool,
}
impl EquipmentInput {
    pub fn new(hold_seconds: f32) -> Self {
        Self {
            hold_seconds,
            elapsed: 0.,
            pressed: false,
            can_equip: true,
            can_recenter: false,
            equipped_now: false,
        }
    }
    pub fn begin_frame(
        &mut self,
        input: FrameInput,
        dt: f32,
        context: EquipmentContext,
    ) -> Option<EquipAction> {
        self.equipped_now = false;
        if input.pressed & keys::RECENTER != 0 {
            self.elapsed = 0.;
            self.pressed = true;
            self.can_recenter = false;
        } else {
            self.elapsed += dt;
        }
        if self.elapsed > self.hold_seconds {
            if input.held & keys::RECENTER != 0 {
                self.can_recenter = true;
            } else {
                self.can_equip = true;
            }
        }
        let short_release = input.released & keys::RECENTER != 0
            && self.pressed
            && self.elapsed < self.hold_seconds;
        if input.released & keys::RECENTER != 0 {
            self.pressed = false;
        }
        if !context.has_carry {
            return None;
        }
        if context.carried_can_equip && !context.has_equipment && self.can_equip && short_release {
            Some(EquipAction::Equip)
        } else if input.held & keys::RECENTER != 0
            && (self.can_recenter || !context.carried_can_equip)
        {
            Some(EquipAction::Recenter)
        } else {
            None
        }
    }
    pub fn record_equipped(&mut self) {
        self.equipped_now = true;
    }
    pub fn record_released(&mut self) {
        self.can_equip = false;
        self.can_recenter = false;
        self.pressed = false;
    }
    pub fn should_release(&self, input: FrameInput, context: EquipmentContext) -> bool {
        !self.equipped_now
            && context.has_equipment
            && !context.has_carry
            && (input.held & keys::ROTATE != 0
                || (!context.seated && input.pressed & keys::RECENTER != 0))
    }
    pub fn finish_frame(&mut self, input: FrameInput) {
        if input.released & keys::RECENTER != 0 {
            self.can_equip = true;
            self.can_recenter = true;
        }
    }
    pub fn cancel_input(&mut self) {
        self.elapsed = 0.;
        self.pressed = false;
        self.can_equip = false;
        self.can_recenter = false;
        self.equipped_now = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{FrameInput, keys};
    const ELIGIBLE: EquipmentContext = EquipmentContext {
        carried_can_equip: true,
        has_carry: true,
        has_equipment: false,
        seated: false,
    };
    fn down() -> FrameInput {
        FrameInput {
            pressed: keys::RECENTER,
            held: keys::RECENTER,
            ..Default::default()
        }
    }
    fn up() -> FrameInput {
        FrameInput {
            released: keys::RECENTER,
            ..Default::default()
        }
    }
    #[test]
    fn strict_hold_threshold_and_source_recenter() {
        for (elapsed, expected) in [(0.125, Some(EquipAction::Equip)), (0.25, None), (0.5, None)] {
            let mut g = EquipmentInput::new(0.25);
            g.begin_frame(FrameInput::default(), 1., ELIGIBLE);
            g.begin_frame(down(), 0.02, ELIGIBLE);
            assert_eq!(g.begin_frame(up(), elapsed, ELIGIBLE), expected);
        }
        for occupied in [false, true] {
            let context = EquipmentContext {
                has_equipment: occupied,
                ..ELIGIBLE
            };
            let mut g = EquipmentInput::new(0.25);
            assert_eq!(g.begin_frame(down(), 0.02, context), None);
            assert_eq!(
                g.begin_frame(
                    FrameInput {
                        held: keys::RECENTER,
                        ..Default::default()
                    },
                    0.25,
                    context
                ),
                None
            );
            assert_eq!(
                g.begin_frame(
                    FrameInput {
                        held: keys::RECENTER,
                        ..Default::default()
                    },
                    0.01,
                    context
                ),
                Some(EquipAction::Recenter)
            );
        }
        let occupied = EquipmentContext {
            has_equipment: true,
            ..ELIGIBLE
        };
        let mut occupied_gesture = EquipmentInput::new(0.25);
        occupied_gesture.begin_frame(down(), 0.02, occupied);
        assert_eq!(occupied_gesture.begin_frame(up(), 0.125, occupied), None);
        let mut g = EquipmentInput::new(0.25);
        assert_eq!(
            g.begin_frame(
                down(),
                0.02,
                EquipmentContext {
                    carried_can_equip: false,
                    ..ELIGIBLE
                }
            ),
            Some(EquipAction::Recenter)
        );
    }
    #[test]
    fn release_occupancy_late_rearm_and_same_frame_guard() {
        let equipment = EquipmentContext {
            has_carry: false,
            has_equipment: true,
            ..ELIGIBLE
        };
        let mut g = EquipmentInput::new(0.25);
        assert!(g.should_release(down(), equipment));
        assert!(!g.should_release(
            FrameInput {
                held: keys::TRANSLATE,
                ..Default::default()
            },
            equipment
        ));
        assert!(!g.should_release(
            down(),
            EquipmentContext {
                seated: true,
                ..equipment
            }
        ));
        let mmb = FrameInput {
            held: keys::ROTATE,
            ..Default::default()
        };
        assert!(g.should_release(
            mmb,
            EquipmentContext {
                seated: true,
                ..equipment
            }
        ));
        assert!(!g.should_release(
            mmb,
            EquipmentContext {
                has_carry: true,
                ..equipment
            }
        ));
        g.begin_frame(down(), 0.02, ELIGIBLE);
        assert_eq!(g.begin_frame(up(), 0.1, ELIGIBLE), Some(EquipAction::Equip));
        g.record_equipped();
        assert!(!g.should_release(mmb, equipment));
        g.begin_frame(FrameInput::default(), 0.01, equipment);
        assert!(g.should_release(mmb, equipment));
        g.record_released();
        assert_eq!(g.begin_frame(up(), 0.01, ELIGIBLE), None);
        g.finish_frame(up());
        g.begin_frame(down(), 0.02, ELIGIBLE);
        assert_eq!(g.begin_frame(up(), 0.1, ELIGIBLE), Some(EquipAction::Equip));
        g.cancel_input();
        assert_eq!(g.begin_frame(up(), 0.1, ELIGIBLE), None);
        g.finish_frame(up());
        g.begin_frame(down(), 0.02, ELIGIBLE);
        assert_eq!(g.begin_frame(up(), 0.1, ELIGIBLE), Some(EquipAction::Equip));
    }
}
