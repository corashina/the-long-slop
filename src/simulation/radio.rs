//! Source controls and broadcasts. Playback belongs to the browser.
use crate::reference::{AudioClipReference, RadioAudioReference, RadioStationReference};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadioControl {
    VolumeUp,
    VolumeDown,
    TuneUp,
    TuneDown,
    Band,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct RadioPlayback {
    pub station: Option<u32>,
    pub clip: Option<u32>,
    pub offset_seconds: f64,
    pub gain: f32,
    pub next_clip: Option<u32>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct RadioFrame {
    pub power: bool,
    pub am: bool,
    pub frequency: f32,
    pub volume: f32,
    pub static_gain: f32,
    pub primary: RadioPlayback,
    pub secondary: RadioPlayback,
}
#[derive(Clone, Copy)]
struct Recording {
    clip: u32,
    duration: f64,
    gain: f32,
    holiday: u8,
}
#[derive(Clone, Copy)]
struct Selected {
    recording: Recording,
    music: bool,
}
struct Station {
    source: RadioStationReference,
    music: Vec<Recording>,
    talk: Vec<Recording>,
    music_bag: Vec<usize>,
    talk_bag: Vec<usize>,
    current: Option<Selected>,
    next: Option<Selected>,
    start: f64,
    random: u32,
}
pub struct RadioState {
    rules: RadioAudioReference,
    stations: Vec<Station>,
    pub on: bool,
    pub am: bool,
    pub tuning: f32,
    pub volume: f32,
    calendar: [u8; 2],
}
impl Station {
    fn select(
        &mut self,
        was_music: bool,
        calendar: [u8; 2],
        ranges: &[[u8; 4]; 2],
        reset: bool,
    ) -> Option<Selected> {
        let music = if was_music {
            self.talk.is_empty()
        } else {
            !self.music.is_empty()
        };
        let (recordings, bag) = if music {
            (&self.music, &mut self.music_bag)
        } else {
            (&self.talk, &mut self.talk_bag)
        };
        if bag.is_empty() {
            if !reset {
                return None;
            }
            for (i, r) in recordings.iter().enumerate() {
                if holiday(r.holiday, calendar, ranges) {
                    bag.push(i);
                }
            }
        }
        if bag.is_empty() {
            return None;
        }
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        let index = (u64::from(self.random) * bag.len() as u64 >> 32) as usize;
        let recording = recordings[bag.remove(index)];
        Some(Selected { recording, music })
    }
    fn advance(&mut self, seconds: f64, calendar: [u8; 2], ranges: &[[u8; 4]; 2]) {
        if self.source.custom {
            return;
        }
        if self.current.is_none() {
            self.current = self.select(self.source.initial_music, calendar, ranges, true);
            self.start = seconds;
            self.next = self
                .current
                .and_then(|s| self.select(s.music, calendar, ranges, false));
        } else if self
            .current
            .is_some_and(|s| seconds - self.start >= s.recording.duration)
        {
            self.current = self
                .next
                .or_else(|| self.select(self.current.unwrap().music, calendar, ranges, true));
            self.start = seconds;
            self.next = self
                .current
                .and_then(|s| self.select(s.music, calendar, ranges, false));
        }
    }
    fn playback(&self, seconds: f64, gain: f32, primary: bool, custom_gain: f32) -> RadioPlayback {
        RadioPlayback {
            station: Some(self.source.id),
            clip: self.current.map(|s| s.recording.clip),
            offset_seconds: if self.current.is_some() {
                (seconds - self.start).max(0.)
            } else {
                0.
            },
            gain: gain
                * if self.source.custom {
                    custom_gain
                } else if primary {
                    self.current.map_or(0., |s| s.recording.gain)
                } else {
                    1.
                },
            next_clip: self.next.map(|s| s.recording.clip),
        }
    }
}
fn holiday(value: u8, [month, day]: [u8; 2], ranges: &[[u8; 4]; 2]) -> bool {
    let date = u16::from(month) * 100 + u16::from(day);
    if value == 0 {
        return true;
    }
    let Some(&[sm, sd, em, ed]) = ranges.get(usize::from(value - 1)) else {
        return false;
    };
    let start = u16::from(sm) * 100 + u16::from(sd);
    let end = u16::from(em) * 100 + u16::from(ed);
    if start <= end {
        (start..=end).contains(&date)
    } else {
        date >= start || date <= end
    }
}
impl RadioState {
    pub fn new(
        reference: &RadioAudioReference,
        clips: &[AudioClipReference],
        calendar: [u8; 2],
    ) -> Self {
        let rules = reference.clone();
        let stations = rules
            .stations
            .iter()
            .map(|s| {
                let recordings = |ids: &[u32], gains: &[f32], holidays: &[u8]| {
                    ids.iter()
                        .enumerate()
                        .map(|(i, &clip)| Recording {
                            clip,
                            duration: clips
                                .iter()
                                .find(|c| c.id == clip)
                                .expect("validated radio clip")
                                .source_duration_seconds,
                            gain: gains.get(i).copied().filter(|&v| v != 0.).unwrap_or(1.),
                            holiday: holidays[i],
                        })
                        .collect::<Vec<_>>()
                };
                let music = recordings(&s.music, &s.music_gains, &s.music_holidays);
                let talk = recordings(&s.talk, &s.talk_gains, &s.talk_holidays);
                Station {
                    source: s.clone(),
                    music_bag: Vec::with_capacity(music.len()),
                    talk_bag: Vec::with_capacity(talk.len()),
                    music,
                    talk,
                    current: None,
                    next: None,
                    start: 0.,
                    random: s.id.wrapping_mul(0x9e3779b9),
                }
            })
            .collect();
        Self {
            on: rules.initial_on,
            am: rules.initial_am,
            tuning: rules.initial_tuning,
            volume: rules.initial_volume,
            rules,
            stations,
            calendar,
        }
    }
    pub fn calendar(&mut self, month: u8, day: u8) {
        self.calendar = [month, day];
    }
    fn range(&self) -> (f32, f32, f32) {
        let c = &self.rules.constants;
        if self.am {
            (c["AMmin"], c["AMmax"], c["AMStep"])
        } else {
            (c["FMmin"], c["FMmax"], c["FMStep"])
        }
    }
    pub fn control(&mut self, control: RadioControl) {
        match control {
            RadioControl::VolumeUp => {
                self.volume = (self.volume + self.rules.constants["vSpeed"]).clamp(0., 1.);
                if self.volume > 0. {
                    self.on = true;
                }
            }
            RadioControl::VolumeDown => {
                self.volume = (self.volume - self.rules.constants["vSpeed"]).clamp(0., 1.);
                if self.volume == 0. {
                    self.on = false;
                }
            }
            RadioControl::Band => {
                if self.rules.controls.band.is_some() {
                    self.am = !self.am;
                }
            }
            RadioControl::TuneUp | RadioControl::TuneDown => {
                let (min, max, step) = self.range();
                let delta = 1. / ((max - min) / step);
                let sign = if control == RadioControl::TuneUp {
                    1.
                } else {
                    -1.
                };
                self.tuning =
                    ((self.tuning + delta * sign).clamp(0., 1.) / delta).round_ties_even() * delta;
            }
        }
    }
    pub fn advance(&mut self, game_seconds: f64, electronics_output: f32) -> RadioFrame {
        for s in &mut self.stations {
            s.advance(game_seconds, self.calendar, &self.rules.holiday_ranges);
        }
        let (min, max, step) = self.range();
        let frequency =
            ((min + (max - min) * self.tuning.clamp(0., 1.)) / step).round_ties_even() * step;
        let power = self.on && electronics_output > self.rules.electronics_cutoff;
        let mut nearest = f32::MAX;
        let mut secondary_distance = f32::MAX;
        let mut primary = None;
        let mut secondary = None;
        for (i, s) in self.stations.iter().enumerate() {
            if (s.source.band == "am") != self.am {
                continue;
            }
            let d = (s.source.frequency - frequency).abs();
            if d < nearest {
                if primary.is_some() {
                    secondary = primary;
                    secondary_distance = nearest;
                }
                primary = Some(i);
                nearest = d;
            }
        }
        let tolerance = if self.am {
            self.rules.am_tolerance
        } else {
            self.rules.fm_tolerance
        };
        let signal = |d: f32| (1. - d / tolerance).clamp(0., 1.) * electronics_output;
        let gain = if power { self.volume } else { 0. };
        RadioFrame {
            power,
            am: self.am,
            frequency,
            volume: self.volume,
            static_gain: (1. - signal(nearest)) * gain,
            primary: primary.map_or(RadioPlayback::default(), |i| {
                self.stations[i].playback(
                    game_seconds,
                    signal(nearest) * gain,
                    true,
                    self.rules.custom_gain,
                )
            }),
            secondary: secondary.map_or(RadioPlayback::default(), |i| {
                self.stations[i].playback(
                    game_seconds,
                    signal(secondary_distance) * gain,
                    false,
                    self.rules.custom_gain,
                )
            }),
        }
    }
}
