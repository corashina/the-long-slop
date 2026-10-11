use crate::common;
use my_summer_slop::reference::ClipId;
use my_summer_slop::simulation::radio::{Radio, RadioInput};

#[test]
fn radio_turns_on_tunes_and_plays_talk_after_music() {
    let reference = common::reference();
    let audio = &reference.audio;
    let mut radio = Radio::new(&audio.radio);
    assert!((radio.frequency(&audio.radio) - 94.5).abs() < 1e-3);
    radio.step(
        audio,
        RadioInput {
            volume: 1,
            ..RadioInput::default()
        },
        1.0 / 60.0,
    );
    assert!(radio.on && radio.clicks == 1);
    let ((station, distance), _) = radio.nearest(&audio.radio);
    assert_eq!(audio.radio.fm_stations[station].name, "Rock");
    assert!((distance - 0.4).abs() < 1e-3);
    // The first clip is music. Talk follows when the song ends.
    let first = radio.playing(station).unwrap();
    assert!([ClipId(1), ClipId(2)].contains(&first.clip.clip));
    let mut next = first;
    while next.play == first.play {
        radio.step(audio, RadioInput::default(), 1.0 / 60.0);
        next = radio.playing(station).unwrap();
    }
    assert_eq!(next.clip.clip, ClipId(0));
    radio.step(
        audio,
        RadioInput {
            tune: 1,
            ..RadioInput::default()
        },
        1.0 / 60.0,
    );
    assert!((radio.frequency(&audio.radio) - 94.6).abs() < 1e-3);
    radio.step(
        audio,
        RadioInput {
            volume: -1,
            band: true,
            ..RadioInput::default()
        },
        1.0 / 60.0,
    );
    assert!(!radio.on && radio.am && radio.clicks == 3);
}
