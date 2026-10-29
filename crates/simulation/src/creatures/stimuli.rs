//! Source `stimulscript` sources that rabbits sense.
//! The player head gives the visible stimulus. Each `PlayClipAtPoint` sound gives a sound
//! stimulus of its volume while its clip plays.
use crate::reference::{CreaturesReference, StimulusKind, StimulusSource};
use rapier3d::math::Vector;
use rapier3d::prelude::RigidBodyHandle;

/// Source `Stimul` SphereCollider radius on the player head.
pub const HEAD_RADIUS: f32 = 0.2;
/// Most one-shot sounds that live at once.
const SOUND_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stimulus {
    /// Local physics position of the stimulus collider.
    pub position: Vector,
    pub kind: StimulusKind,
    pub value: f32,
    pub visible: bool,
    pub sound: bool,
    /// Collider radius. A vision ray that reaches it hits the stimulus.
    pub radius: f32,
    /// The body that carries the stimulus collider. It does not block the vision ray.
    pub body: Option<RigidBodyHandle>,
}

/// One `mainscript.Audio` copy: its stimulus lives for the clip length.
#[derive(Clone, Copy, Debug)]
struct Sound {
    position: [f64; 3],
    value: f32,
    end: f64,
}

#[derive(Default)]
pub struct Stimuli {
    list: Vec<Stimulus>,
    sounds: Vec<Sound>,
}

impl Stimuli {
    pub fn new() -> Self {
        Self {
            list: Vec::with_capacity(SOUND_CAPACITY + 1),
            sounds: Vec::with_capacity(SOUND_CAPACITY),
        }
    }
    pub fn list(&self) -> &[Stimulus] {
        &self.list
    }
    /// `PlayClipAtPoint(clip, position, volume)`: `SetAudioValue(volume)` on the prefab stimulus.
    pub fn sound(&mut self, position: [f64; 3], volume: f32, now: f64, seconds: f32) {
        if self.sounds.len() == SOUND_CAPACITY {
            self.sounds.remove(0);
        }
        self.sounds.push(Sound {
            position,
            value: volume,
            end: now + f64::from(seconds),
        });
    }
    /// Rebuilds the list. `head` is the player head stimulus and its body.
    pub fn rebuild(
        &mut self,
        sources: &PlayerStimuli,
        head: (Vector, RigidBodyHandle),
        now: f64,
        local: impl Fn([f64; 3]) -> Vector,
    ) {
        self.sounds.retain(|s| s.end > now);
        self.list.clear();
        for v in &sources.head.stimuli {
            self.list.push(Stimulus {
                position: head.0,
                kind: v.kind,
                value: v.value,
                visible: sources.head.visible,
                sound: sources.head.sound,
                radius: HEAD_RADIUS,
                body: Some(head.1),
            });
        }
        for s in &self.sounds {
            self.list.push(Stimulus {
                position: local(s.position),
                kind: sources.sound.stimuli[0].kind,
                value: s.value * sources.sound.max_sound_value,
                visible: sources.sound.visible,
                sound: sources.sound.sound,
                radius: 0.,
                body: None,
            });
        }
    }
}

/// The stimulus rows that the port binds.
pub struct PlayerStimuli {
    /// `Stimul` on the player Head.
    pub head: StimulusSource,
    /// The stimulus on the `mainscript.Audio` prefab.
    pub sound: StimulusSource,
}

impl PlayerStimuli {
    pub fn new(r: &CreaturesReference) -> Option<Self> {
        Some(Self {
            head: r.stimulus(&r.player_stimulus)?.clone(),
            sound: r.stimulus(&r.sound_stimulus)?.clone(),
        })
    }
}
