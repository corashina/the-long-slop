//! Native audio events. Bounded priority eviction is a runtime adapter, not a recovered Unity rule.
use crate::reference::AudioReference;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioKind {
    Eat = 0,
    Burp = 1,
    Fart = 2,
    Urinate = 3,
    Sleep = 4,
    Death = 5,
    Storm = 6,
    AmbientDay = 7,
    AmbientNight = 8,
    Shit = 9,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioAction {
    Start = 0,
    Update = 1,
    Stop = 2,
}
#[derive(Clone, Copy, Debug)]
pub struct AudioEvent {
    pub session: u32,
    pub emitter: u32,
    pub kind: AudioKind,
    pub action: AudioAction,
    pub clip: u32,
    pub gain: f32,
    pub pitch: f32,
    pub position: [f64; 3],
}
struct Voice {
    event: AudioEvent,
    remaining: f32,
}
pub struct AudioState {
    session: u32,
    voices: Vec<Voice>,
    voice_capacity: usize,
    durations: Vec<f32>,
    looped: [bool; 10],
    events: Vec<AudioEvent>,
    priorities: [u32; 10],
    capacity: usize,
    pub ids: Vec<u32>,
    pub values: Vec<f32>,
}
impl AudioState {
    pub fn new(reference: &AudioReference) -> Self {
        let mut priorities = [256; 10];
        let mut looped = [false; 10];
        let playback = reference.playback().ok();
        for b in playback.into_iter().flat_map(|p| &p.bindings) {
            priorities[b.kind as usize] = b.priority;
            looped[b.kind as usize] = b.looped;
        }
        let voice_capacity = playback.map_or(0, |p| p.real_voices);
        let capacity = playback.map_or(0, |p| p.queue_capacity);
        Self {
            session: 1,
            voices: Vec::with_capacity(voice_capacity),
            voice_capacity,
            durations: reference
                .clips
                .iter()
                .map(|c| c.duration_seconds as f32)
                .collect(),
            looped,
            events: Vec::with_capacity(capacity),
            priorities,
            capacity,
            ids: Vec::with_capacity(capacity * 5),
            values: Vec::with_capacity(capacity * 5),
        }
    }
    pub fn emit(&mut self, event: AudioEvent) {
        if self.capacity == 0
            || event.session != self.session
            || event.kind == AudioKind::Urinate
            || !event.gain.is_finite()
            || !event.pitch.is_finite()
            || event.pitch <= 0.
            || !event.position.iter().all(|v| v.is_finite())
        {
            return;
        }
        if event.action == AudioAction::Start {
            if event.kind != AudioKind::Eat {
                self.voices
                    .retain(|v| v.event.emitter != event.emitter || v.event.kind != event.kind);
            }
            if self.voices.len() == self.voice_capacity {
                let (index, old) = self
                    .voices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, v)| self.priorities[v.event.kind as usize])
                    .unwrap();
                if self.priorities[event.kind as usize] > self.priorities[old.event.kind as usize] {
                    return;
                }
                self.voices.swap_remove(index);
            }
            let duration = self
                .durations
                .get(event.clip as usize)
                .copied()
                .unwrap_or(0.);
            self.voices.push(Voice {
                event,
                remaining: if self.looped[event.kind as usize] {
                    f32::INFINITY
                } else if event.kind == AudioKind::Eat {
                    duration
                } else {
                    duration / event.pitch
                },
            });
        } else if event.action == AudioAction::Stop {
            self.voices
                .retain(|v| v.event.emitter != event.emitter || v.event.kind != event.kind);
        }
        if event.action == AudioAction::Update
            && let Some(old) = self.events.iter_mut().rev().find(|e| {
                e.session == event.session
                    && e.emitter == event.emitter
                    && e.kind == event.kind
                    && e.action == AudioAction::Update
            })
        {
            *old = event;
            return;
        }
        if self.events.len() == self.capacity {
            let (index, old) = self
                .events
                .iter()
                .enumerate()
                .max_by_key(|(_, e)| self.priorities[e.kind as usize])
                .unwrap();
            if event.action != AudioAction::Stop
                && self.priorities[event.kind as usize] > self.priorities[old.kind as usize]
            {
                return;
            }
            self.events.remove(index);
        }
        self.events.push(event);
    }
    pub fn advance(
        &mut self,
        dt: f32,
        reference: &AudioReference,
        pose: crate::vehicle::VehiclePose,
        origin: [f64; 3],
    ) {
        let Ok(playback) = reference.playback() else {
            return;
        };
        for index in (0..self.voices.len()).rev() {
            let voice = &mut self.voices[index];
            voice.remaining -= dt;
            if voice.remaining <= 0. {
                self.voices.swap_remove(index);
                continue;
            }
            if matches!(
                voice.event.kind,
                AudioKind::Burp | AudioKind::Fart | AudioKind::Shit | AudioKind::Death
            ) {
                let offset = playback.binding(voice.event.kind as u32).offset;
                let p =
                    crate::vehicle::to_rapier(pose) * rapier3d::math::Vector::from_array(offset);
                voice.event.position = std::array::from_fn(|i| p[i] as f64 + origin[i]);
                let mut event = voice.event;
                event.action = AudioAction::Update;
                self.emit(event);
            }
        }
    }
    pub fn reset(&mut self, session: u32) {
        self.session = session;
        self.voices.clear();
        self.clear_events();
    }
    pub fn events(&self) -> &[AudioEvent] {
        &self.events
    }
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
    pub fn pack(&mut self, origin: [f64; 3]) {
        self.ids.clear();
        self.values.clear();
        for e in &self.events {
            self.ids.extend_from_slice(&[
                e.session,
                e.emitter,
                e.kind as u32,
                e.action as u32,
                e.clip,
            ]);
            self.values.extend_from_slice(&[
                e.gain,
                e.pitch,
                (e.position[0] - origin[0]) as f32,
                (e.position[1] - origin[1]) as f32,
                (e.position[2] - origin[2]) as f32,
            ]);
        }
    }
}
