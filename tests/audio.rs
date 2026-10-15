mod common;
use my_summer_slop::reference::{AudioReference, ReferenceManifest};

#[test]
fn bounded_events_keep_priority_and_reset_session() {
    use my_summer_slop::audio::{AudioEvent, AudioState};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    let make = |sequence, priority| AudioEvent {
        session: 1,
        sequence,
        binding: 29,
        emitter: 56,
        position: [0.; 3],
        gain: 1.,
        pitch: 1.,
        time_seconds: 0.,
        priority,
    };
    state.push_event(make(1, 25));
    state.push_event(make(2, 25));
    assert_eq!(state.pop_event().unwrap().sequence, 1);
    assert_eq!(state.pop_event().unwrap().sequence, 2);
    assert!(state.pop_event().is_none());
    for sequence in 1..=65 {
        state.push_event(make(sequence, if sequence == 1 { 255 } else { 10 }));
    }
    assert_eq!(state.event_count(), 64);
    assert_eq!(state.pop_event().unwrap().sequence, 2);
    state.reset(2);
    assert!(state.pop_event().is_none());
    state.push_event(make(66, 10));
    assert!(state.pop_event().is_none());
}

fn interaction_audio() -> (ReferenceManifest, my_summer_slop::audio::AudioState) {
    use my_summer_slop::{audio::AudioState, starting_area::GarageRuntime};
    let mut game = common::reference();
    let source = AudioReference::from_value(&game.audio, &game)
        .unwrap()
        .bindings
        .iter()
        .find(|b| b.trigger == "crafted")
        .unwrap()
        .source
        .clone();
    game.items[0].pickup.as_mut().unwrap().source = source;
    let run = GarageRuntime::new(&game).unwrap();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    state.bind_game(&game, &run.area, &run.physics);
    (game, state)
}
#[test]
fn removal_discards_pending_owned_events() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    state.action(InteractionAction::PartInstall(game.items[0].id), 1.);
    let id = state
        .reference
        .bindings
        .iter()
        .find(|b| b.trigger == "crafted")
        .unwrap()
        .emitter;
    state.remove_owner(id);
    state.action(InteractionAction::PartInstall(game.items[0].id), 2.);
    assert_eq!(state.event_count(), 0);
}
