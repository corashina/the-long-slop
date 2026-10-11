//! Source radio: `radioscript` controls and the `mainscript.radioCH` station clocks.
//! Stations play in world time whether a radio listens or not.

use crate::reference::{AudioReference, RadioClip, RadioReference, RadioStation};
use crate::starting_area::start_sample;

const SEED: u64 = 0x5eed_7ad1_0000_0001;

/// Button presses on the radio in one step. Steps are signed knob notches.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadioInput {
    pub volume: i32,
    pub tune: i32,
    pub band: bool,
}

/// The clip a station plays now. `play` numbers each new clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationClip {
    pub clip: RadioClip,
    pub seconds: f32,
    pub play: u32,
}

#[derive(Clone, Debug)]
struct StationClock {
    current: Option<StationClip>,
    music: bool,
    open_music: Vec<usize>,
    open_talk: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Radio {
    pub on: bool,
    /// `radioscript.v` from zero to one.
    pub volume: f32,
    /// `radioscript.currentTeker`: the dial from zero to one.
    pub tune: f32,
    pub am: bool,
    /// Counts on, off, and band clicks for the click sound.
    pub clicks: u32,
    fm: Vec<StationClock>,
    am_band: Vec<StationClock>,
    samples: u64,
    plays: u32,
}

impl Radio {
    pub fn new(reference: &RadioReference) -> Self {
        let clocks = |stations: &[RadioStation]| {
            stations
                .iter()
                .map(|_| StationClock {
                    current: None,
                    music: false,
                    open_music: Vec::new(),
                    open_talk: Vec::new(),
                })
                .collect()
        };
        Self {
            on: false,
            volume: 0.0,
            tune: reference.tune,
            am: false,
            clicks: 0,
            fm: clocks(&reference.fm_stations),
            am_band: clocks(&reference.am_stations),
            samples: 0,
            plays: 0,
        }
    }

    pub fn step(&mut self, audio: &AudioReference, input: RadioInput, dt: f32) {
        let radio = &audio.radio;
        for _ in 0..input.volume.unsigned_abs() {
            let step = radio.volume_step * input.volume.signum() as f32;
            self.volume = (self.volume + step).clamp(0.0, 1.0);
            if self.on != (self.volume > 0.0) {
                self.on = self.volume > 0.0;
                self.clicks = self.clicks.wrapping_add(1);
            }
        }
        for _ in 0..input.tune.unsigned_abs() {
            let (band, step) = self.band(radio);
            let notch = 1.0 / ((band[1] - band[0]) / step);
            let tune = (self.tune + notch * input.tune.signum() as f32).clamp(0.0, 1.0);
            self.tune = (tune / notch).round() * notch;
        }
        if input.band {
            self.am = !self.am;
            self.clicks = self.clicks.wrapping_add(1);
        }
        let mut clocks = std::mem::take(&mut self.fm);
        self.advance(audio, &radio.fm_stations, &mut clocks, dt);
        self.fm = clocks;
        let mut clocks = std::mem::take(&mut self.am_band);
        self.advance(audio, &radio.am_stations, &mut clocks, dt);
        self.am_band = clocks;
    }

    /// `radioscript.FR`: the dial rounded to the band step.
    pub fn frequency(&self, radio: &RadioReference) -> f32 {
        let (band, step) = self.band(radio);
        (mix(band[0], band[1], self.tune) / step).round() * step
    }

    pub fn stations<'a>(&self, radio: &'a RadioReference) -> &'a [RadioStation] {
        if self.am {
            &radio.am_stations
        } else {
            &radio.fm_stations
        }
    }

    /// The clip of a station in the current band.
    pub fn playing(&self, station: usize) -> Option<StationClip> {
        let clocks = if self.am { &self.am_band } else { &self.fm };
        clocks[station].current
    }

    /// `radioscript.SetRadio` station search: the nearest station and the one it replaced.
    pub fn nearest(&self, radio: &RadioReference) -> ((usize, f32), Option<(usize, f32)>) {
        let frequency = self.frequency(radio);
        let mut best = (0, f32::MAX);
        let mut second = None;
        for (i, station) in self.stations(radio).iter().enumerate() {
            let distance = (station.frequency - frequency).abs();
            if distance < best.1 {
                if best.1 < f32::MAX {
                    second = Some(best);
                }
                best = (i, distance);
            }
        }
        (best, second)
    }

    fn band(&self, radio: &RadioReference) -> ([f32; 2], f32) {
        if self.am {
            (radio.am, radio.am_step)
        } else {
            (radio.fm, radio.fm_step)
        }
    }

    /// `mainscript.RADIO`.
    fn advance(
        &mut self,
        audio: &AudioReference,
        stations: &[RadioStation],
        clocks: &mut [StationClock],
        dt: f32,
    ) {
        for (station, clock) in stations.iter().zip(clocks) {
            match &mut clock.current {
                Some(current)
                    if current.seconds + dt < audio.clips[current.clip.clip.0 as usize].seconds =>
                {
                    current.seconds += dt;
                }
                _ => self.next(audio, station, clock),
            }
        }
    }

    /// `radioCH.AdvanceRadio`: talk follows music when the station has talk.
    fn next(&mut self, audio: &AudioReference, station: &RadioStation, clock: &mut StationClock) {
        if station.custom {
            return;
        }
        let music = if clock.music {
            station.talk.is_empty()
        } else {
            !station.music.is_empty()
        };
        let (clips, open) = if music {
            (&station.music, &mut clock.open_music)
        } else {
            (&station.talk, &mut clock.open_talk)
        };
        if clips.is_empty() {
            return;
        }
        if open.is_empty() {
            open.extend(0..clips.len());
        }
        self.samples = self.samples.wrapping_add(1);
        let pick =
            ((start_sample(SEED, self.samples) * open.len() as f32) as usize).min(open.len() - 1);
        let index = open.remove(pick);
        self.plays = self.plays.wrapping_add(1).max(1);
        clock.music = music;
        clock.current = Some(StationClip {
            clip: clips[index],
            seconds: 0.0,
            play: self.plays,
        });
        debug_assert!(audio.clips[clips[index].clip.0 as usize].seconds > 0.0);
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
