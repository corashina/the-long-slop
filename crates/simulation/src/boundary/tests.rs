use super::*;
use crate::reference::ItemId;

#[test]
fn frame_round_trips() {
    let row = |x: f32| PoseRow {
        position: [x, 2., 3.],
        flags: PoseRow::VISIBLE,
        rotation: [0., 0., 0., 1.],
    };
    let frame = SimFrame {
        seq_consumed: 42,
        state: StateBlock {
            simulation_seconds: 12.5,
            alpha: 0.25,
            flags: StateBlock::STARTED | StateBlock::SEATED,
        },
        previous: vec![row(1.), row(4.)],
        current: vec![row(2.), row(5.)],
        changes: vec![
            Change::Spawn {
                handle: Handle(1),
                key: PoseKey::Item(ItemId(7)),
            },
            Change::Retire(Handle(0)),
            Change::Failure("stream".into()),
        ],
    };
    let mut bytes = Vec::new();
    encode(&frame, &mut bytes);
    let mut decoded = SimFrame::default();
    decode(&bytes, &mut decoded).unwrap();
    assert_eq!(decoded, frame);
    assert!(decode(&bytes[..20], &mut decoded).is_err());
}

#[test]
fn command_round_trips() {
    let mut command = SimCommand {
        seq: 9,
        delta_seconds: 1. / 60.,
        ..Default::default()
    };
    command.input.pressed = 5;
    command.input.mouse = [0.5, -1.];
    command.input.carry.owner = Some(ItemId(3));
    command.input.typing.text[..2].copy_from_slice(b"hi");
    command.input.typing.len = 2;
    command.input.gamepad.axes[31] = 0.75;
    let mut bytes = Vec::new();
    encode_command(&command, &mut bytes);
    assert_eq!(decode_command(&bytes).unwrap(), command);
}

#[test]
fn handle_is_reused_only_after_release() {
    let mut handles = Handles::default();
    let (a, new) = handles.assign(PoseKey::Item(ItemId(1)));
    assert!(new);
    assert_eq!(handles.assign(PoseKey::Item(ItemId(1))), (a, false));
    handles.retire(a);
    let (b, _) = handles.assign(PoseKey::Item(ItemId(2)));
    assert_ne!(a, b);
    handles.release(a);
    let (c, _) = handles.assign(PoseKey::Item(ItemId(3)));
    assert_eq!(c, a);
    assert_eq!(handles.len(), 2);
}
