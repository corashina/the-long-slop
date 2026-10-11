//! Original sound clips and the source sound rules.
use super::*;

id!(ClipId, u32);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipRecord {
    /// Zero-based index into `AudioReference::clips`.
    pub id: ClipId,
    pub source: SourceId,
    pub name: String,
    pub path: String,
    pub seconds: f32,
    pub sha256: String,
}

/// A source `AudioSource`. The offset is local to the item pose.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceReference {
    pub source: SourceId,
    pub clip: Option<ClipId>,
    pub volume: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// Unity spatial blend: 0 is flat, 1 is positional.
    pub spatial: f32,
    pub item: Option<ItemId>,
    pub offset: [f32; 3],
}

/// `mainscript.Audio`, the prefab that `PlayClipAtPoint` copies.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneShotReference {
    pub min_distance: f32,
    pub max_distance: f32,
    pub spatial: f32,
    pub random_pitch: [f32; 2],
}

/// `fpscontroller.SetOutSideAudio`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusReference {
    pub default_interior: f32,
    pub interior_lerp: f32,
    pub car_multiplier: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSoundReference {
    pub idle: VoiceReference,
    pub motor: VoiceReference,
    pub start: ClipId,
    pub start_volume: f32,
    pub stop: ClipId,
    pub stop_volume: f32,
    pub idle_rpm: [f32; 2],
    pub idle_pitch: [f32; 2],
    pub idle_volume: f32,
    pub motor_rpm: [f32; 2],
    pub motor_pitch: [f32; 2],
    pub motor_volume: f32,
    pub reverse_volume: f32,
}

/// `mainscript.CarCoolingSounds`. Each pair is a range at the low and high temperature.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoolingSoundReference {
    pub clips: Vec<ClipId>,
    pub temperature: [f32; 2],
    pub time: [[f32; 2]; 2],
    pub volume: [[f32; 2]; 2],
    pub pitch: [[f32; 2]; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelSoundReference {
    pub ground: VoiceReference,
    pub spring: VoiceReference,
    pub slide: VoiceReference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkidReference {
    pub clip: ClipId,
    pub volume: f32,
    pub pitch: [f32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarSoundReference {
    /// `mainscript.collide0` to `collide3`.
    pub collide: [Vec<ClipId>; 4],
    pub collision_modifier: f32,
    pub brake: VoiceReference,
    pub brake_modifier: f32,
    pub reverse: VoiceReference,
    pub handbrake: ClipId,
    pub handbrake_voice: VoiceReference,
    pub boil: VoiceReference,
    pub boil_volume: f32,
    pub wheel: WheelSoundReference,
    pub ground_modifier: f32,
    pub spring_modifier: f32,
    pub spring_pitch_modifier: f32,
    pub slide_angle: f32,
    pub max_slide_volume_speed: f32,
    pub skid_road: SkidReference,
    pub skid_dirt: SkidReference,
    pub speed_difference: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Road,
    Desert,
    Winter,
    Grass,
    Wood,
    /// The sand layer. Steps use the biome sound.
    Terrain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkSoundReference {
    pub clips: Vec<ClipId>,
    pub volume: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRecord {
    pub collider: ColliderId,
    pub surface: Surface,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepSoundReference {
    /// `mainscript.walkSounds` in road, desert, winter, grass, wood order.
    pub surfaces: [WalkSoundReference; 5],
    pub modifier: f32,
    pub ray_distance: f32,
    pub bob_speed: f32,
    pub bob_speed_velocity: f32,
    pub bob_height: f32,
    pub bob_threshold: f32,
    pub run_bonus: f32,
    /// Colliders that are not road.
    pub colliders: Vec<SurfaceRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindSoundReference {
    pub voice: VoiceReference,
    pub max_speed_kmh: f32,
    pub lerp: f32,
    pub view_modifier: f32,
}

/// `napszakvaltakozas` ambient sources in desert, winter, grass order.
/// Each volume pair holds the day value and the night value.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbientSoundReference {
    pub day: [VoiceReference; 3],
    pub night: [VoiceReference; 3],
    pub day_volume: [[f32; 3]; 2],
    pub night_volume: [[f32; 3]; 2],
    pub fade_rate: f32,
}

/// An `interiorscript` door. A door without an item stays closed.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteriorDoor {
    pub item: Option<ItemId>,
    /// A removable door counts as open when it is not fitted.
    pub removable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteriorReference {
    pub source: SourceId,
    pub collider: ColliderId,
    /// Index into `AudioReference::interiors`.
    pub parent: Option<usize>,
    pub doors: Vec<InteriorDoor>,
    pub parent_door: Option<InteriorDoor>,
    pub min_doors_to_max: u32,
    pub max_value: f32,
    pub value: Option<f32>,
    pub inverted: bool,
    pub vehicle: bool,
}

/// `usablescript` limit clips. Min and max are the hinge limits.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HingeSoundReference {
    pub item: ItemId,
    pub min_open: Option<ClipId>,
    pub min_close: Option<ClipId>,
    pub max_open: Option<ClipId>,
    pub max_close: Option<ClipId>,
    pub modifier: f32,
    pub inverted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningSoundReference {
    pub opening: OpeningId,
    pub item: ItemId,
    pub open: Option<ClipId>,
    pub open_volume: f32,
    pub close: Option<ClipId>,
    pub close_volume: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemSoundReference {
    pub item: ItemId,
    pub clip: ClipId,
    pub modifier: f32,
}

/// `buttonSoundMain` menu sounds. The browser plays them.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuSoundReference {
    pub pitch: [f32; 2],
    pub volume: f32,
    pub touch_volume: f32,
    pub down: Vec<ClipId>,
    pub up: Vec<ClipId>,
    pub touch_down: Vec<ClipId>,
    pub touch_up: Vec<ClipId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioReference {
    pub clips: Vec<ClipRecord>,
    pub one_shot: OneShotReference,
    pub buses: BusReference,
    pub engine: EngineSoundReference,
    pub cooling: CoolingSoundReference,
    pub car: CarSoundReference,
    pub steps: StepSoundReference,
    pub wind: WindSoundReference,
    pub ambient: AmbientSoundReference,
    pub interiors: Vec<InteriorReference>,
    pub hinges: Vec<HingeSoundReference>,
    pub openings: Vec<OpeningSoundReference>,
    pub items: Vec<ItemSoundReference>,
    pub menu: MenuSoundReference,
}

impl AudioReference {
    pub fn validate(
        &self,
        items: &HashSet<ItemId>,
        colliders: &HashSet<ColliderId>,
        openings: &HashSet<OpeningId>,
    ) -> Result<(), ReferenceError> {
        require(
            self.clips
                .iter()
                .enumerate()
                .all(|(i, clip)| clip.id.0 == i as u32 && hash(&clip.sha256) && clip.seconds > 0.0),
            "audio.clips",
        )?;
        let clip = |id: &ClipId| (id.0 as usize) < self.clips.len();
        let optional = |id: &Option<ClipId>| id.as_ref().is_none_or(clip);
        let voice = |v: &VoiceReference| {
            optional(&v.clip)
                && v.item.as_ref().is_none_or(|item| items.contains(item))
                && finite(&[v.volume, v.min_distance, v.max_distance, v.spatial])
                && finite(&v.offset)
                && v.min_distance > 0.0
                && v.max_distance >= v.min_distance
        };
        let engine = &self.engine;
        require(
            voice(&engine.idle)
                && voice(&engine.motor)
                && clip(&engine.start)
                && clip(&engine.stop)
                && engine.idle_rpm[0] < engine.idle_rpm[1]
                && engine.motor_rpm[0] < engine.motor_rpm[1],
            "audio.engine",
        )?;
        require(
            !self.cooling.clips.is_empty() && self.cooling.clips.iter().all(clip),
            "audio.cooling",
        )?;
        let car = &self.car;
        require(
            car.collide
                .iter()
                .all(|tier| !tier.is_empty() && tier.iter().all(clip))
                && voice(&car.brake)
                && voice(&car.reverse)
                && clip(&car.handbrake)
                && voice(&car.handbrake_voice)
                && voice(&car.boil)
                && voice(&car.wheel.ground)
                && voice(&car.wheel.spring)
                && voice(&car.wheel.slide)
                && clip(&car.skid_road.clip)
                && clip(&car.skid_dirt.clip)
                && car.slide_angle < 90.0
                && car.speed_difference[0] < car.speed_difference[1],
            "audio.car",
        )?;
        require(
            self.steps
                .surfaces
                .iter()
                .all(|s| !s.clips.is_empty() && s.clips.iter().all(clip))
                && self
                    .steps
                    .colliders
                    .iter()
                    .all(|record| colliders.contains(&record.collider)),
            "audio.steps",
        )?;
        require(
            voice(&self.wind.voice)
                && self.wind.max_speed_kmh > 0.0
                && self
                    .ambient
                    .day
                    .iter()
                    .chain(&self.ambient.night)
                    .all(voice),
            "audio.ambient",
        )?;
        let door = |d: &InteriorDoor| d.item.as_ref().is_none_or(|item| items.contains(item));
        require(
            self.interiors.iter().all(|interior| {
                colliders.contains(&interior.collider)
                    && interior
                        .parent
                        .is_none_or(|parent| parent < self.interiors.len())
                    && interior.doors.iter().all(door)
                    && interior.parent_door.as_ref().is_none_or(door)
            }),
            "audio.interiors",
        )?;
        require(
            self.hinges.iter().all(|h| {
                items.contains(&h.item)
                    && [h.min_open, h.min_close, h.max_open, h.max_close]
                        .iter()
                        .all(optional)
            }) && self.openings.iter().all(|o| {
                openings.contains(&o.opening)
                    && items.contains(&o.item)
                    && optional(&o.open)
                    && optional(&o.close)
            }) && self
                .items
                .iter()
                .all(|i| items.contains(&i.item) && clip(&i.clip)),
            "audio.objects",
        )?;
        let menu = &self.menu;
        require(
            [&menu.down, &menu.up, &menu.touch_down, &menu.touch_up]
                .iter()
                .all(|list| list.iter().all(clip)),
            "audio.menu",
        )
    }
}
