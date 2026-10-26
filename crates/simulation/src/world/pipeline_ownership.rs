#[derive(Debug, PartialEq, Eq)]
pub enum HeldEntryPhase {
    Held,
    Retired,
}
#[derive(Debug, PartialEq, Eq)]
pub enum HeldEntryAction {
    Wait,
    Prepare,
    Retire,
    Release,
}
pub struct HeldEntryObservation {
    pub cancelled: bool,
    pub isolated: bool,
    pub pipeline_ready: bool,
    pub preparation_valid: bool,
}
pub fn held_entry_step(
    phase: &mut HeldEntryPhase,
    observation: HeldEntryObservation,
) -> HeldEntryAction {
    if *phase == HeldEntryPhase::Retired {
        return HeldEntryAction::Wait;
    }
    if observation.cancelled {
        *phase = HeldEntryPhase::Retired;
        return HeldEntryAction::Retire;
    }
    if !observation.isolated {
        assert!(observation.pipeline_ready, "Promoted pipeline is not ready");
        return HeldEntryAction::Release;
    }
    if !observation.pipeline_ready && observation.preparation_valid {
        HeldEntryAction::Prepare
    } else {
        HeldEntryAction::Wait
    }
}
