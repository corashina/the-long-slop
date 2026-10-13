use super::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedValues {
    pub hp: f32,
    pub food: f32,
    pub water: f32,
    pub bowel: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumptionEffect {
    pub food: f32,
    pub water: f32,
    pub bowel: f32,
    pub hp: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoodKind {
    Sausage,
    Turd,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedEvidence {
    pub assembly_sha256: String,
    pub methods: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodCollider {
    pub source: SourceId,
    pub node: NodeId,
    pub membership: u32,
    pub filter: u32,
    pub sensor: bool,
    pub enabled: bool,
    pub geometry: ColliderGeometry,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodReference {
    pub source: SourceId,
    pub kind: FoodKind,
    pub effect: ConsumptionEffect,
    pub bites: u8,
    pub index: u8,
    pub destroy_on_end: bool,
    pub asset: AssetId,
    pub root: NodeId,
    pub mass_kg: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub pickup: PickupReference,
    pub parts: Vec<Vec<NodeId>>,
    pub colliders: Vec<FoodCollider>,
    pub particle_type: u8,
    pub min_particle: u32,
    pub max_particle: u32,
    pub sound: u8,
    pub text: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInput {
    pub action: String,
    pub index: u32,
    pub key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToiletReference {
    pub source: SourceId,
    pub bladder: SourceId,
    pub radius: f32,
    pub no_angle: bool,
    pub valve_gain: f32,
    pub empty_amount: f32,
    pub bowel_fraction: f32,
    pub fart_burp_ratio: f32,
    pub fart_shit_ratio: f32,
    pub min_fart_pitch: f32,
    pub max_fart_pitch: f32,
    pub min_fart_volume: f32,
    pub max_fart_volume: f32,
    pub min_burp_pitch: f32,
    pub max_burp_pitch: f32,
    pub min_burp_volume: f32,
    pub max_burp_volume: f32,
    pub max_rate: f32,
    pub min_rate: f32,
    pub max_speed: f32,
    pub force_color: [f32; 4],
    pub force_color_m: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurvivalReference {
    pub mode_source: SourceId,
    pub mode: u8,
    pub death_seconds: f32,
    pub drinking: DrinkingReference,
    pub source: SourceId,
    pub player: SourceId,
    pub blood: SourceId,
    pub initial: NeedValues,
    pub maximum: NeedValues,
    pub initial_bladder: [f32; 6],
    pub bladder_capacity_liters: f32,
    pub initial_queue: ConsumptionEffect,
    pub starter_food: ItemId,
    pub foods: Vec<FoodReference>,
    pub starter_parts: Vec<Vec<NodeId>>,
    pub starter_parent: SourceId,
    pub starter_pose: Transform,
    pub evidence: ManagedEvidence,
    pub toilet: ToiletReference,
    pub inputs: Vec<SourceInput>,
    pub drink_modifier: f32,
    pub death_velocity_change: f32,
    pub food_stat_show_time: f32,
    pub food_stat_hide_time: f32,
    pub water_to_piss_ratio: f32,
    pub food_to_shit_ratio: f32,
    pub blood_to_hp_ratio: f32,
    pub alcohol_heal: f32,
    pub alcohol_convert_to_piss: f32,
    pub car_fluids_damage: f32,
    pub max_healing_speed: f32,
    pub healing_toilet_full_modfiier: f32,
    pub water_damage: f32,
    pub food_damage: f32,
    pub def_temp: f32,
    pub cold_temp: f32,
    pub hot_temp: f32,
    pub food_loss: f32,
    pub cold_food_loss_modifier: f32,
    pub hot_food_loss_modifier: f32,
    pub water_loss: f32,
    pub cold_water_loss_modifier: f32,
    pub hot_water_loss_modifier: f32,
    pub run_food_loss_modifier: f32,
    pub run_water_loss_modifier: f32,
    pub delay_speed: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SleepReference {
    pub source: SourceId,
    /// survivalscript.sleepModifier, source level1:73239.
    pub need_modifier: f32,
    pub hold_gain: f32,
    pub sleep_real_seconds: f32,
    pub sleep_cycle_seconds: f32,
    pub cooldown_seconds: f32,
    pub sleep_cam_ring_min: f32,
    pub sleep_cam_ring_max: f32,
    pub sleep_cam_height_min: f32,
    pub sleep_cam_height_max: f32,
    pub sleep_layer_mask: u32,
    pub cloud_low_min_speed: f32,
    pub cloud_low_mid_speed: f32,
    pub cloud_low_max_speed: f32,
    pub cloud_high_min_speed: f32,
    pub cloud_high_mid_speed: f32,
    pub cloud_high_max_speed: f32,
    pub evidence: ManagedEvidence,
}
impl NeedValues {
    fn values(self) -> [f32; 4] {
        [self.hp, self.food, self.water, self.bowel]
    }
}
impl ConsumptionEffect {
    fn valid(self) -> bool {
        finite(&[self.food, self.water, self.bowel, self.hp])
    }
}
impl ManagedEvidence {
    pub(super) fn valid(&self) -> bool {
        self.assembly_sha256 == "9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e"
            && !self.methods.is_empty()
    }
}
impl SurvivalReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.evidence.valid() && self.source.file == "level1" && self.source.path_id == 73239,
            "survival source",
        )?;
        require(
            self.player == m.player.source
                && self
                    .initial
                    .values()
                    .iter()
                    .zip(self.maximum.values())
                    .all(|(a, b)| a.is_finite() && b.is_finite() && *a >= 0. && *a <= b && b > 0.),
            "initial needs",
        )?;
        require(
            finite(&self.initial_bladder)
                && self.initial_bladder.iter().all(|v| *v >= 0.)
                && self.bladder_capacity_liters.is_finite()
                && self.bladder_capacity_liters > 0.
                && self.initial_bladder.iter().sum::<f32>() <= self.bladder_capacity_liters
                && self.initial_queue.valid(),
            "initial bladder and queue",
        )?;
        let starter = m
            .items
            .iter()
            .find(|i| i.id == self.starter_food)
            .filter(|i| matches!(i.kind, ItemKind::Food))
            .and_then(|i| {
                m.nodes
                    .iter()
                    .find(|n| n.id == i.node && n.source == i.source)
            })
            .filter(|n| n.source.file == "level1" && n.source.path_id == 24573)
            .ok_or_else(|| ReferenceError("Invalid starter food source".into()))?;
        require(transform(&self.starter_pose), "starter pose")?;
        require(
            self.foods.len() == 2
                && ids(self.foods.iter().map(|f| f.kind), "food kinds")?.len() == 2,
            "food prototypes",
        )?;
        for f in &self.foods {
            require(
                f.bites > 0
                    && f.index < f.bites
                    && f.parts.len() == f.bites as usize
                    && f.destroy_on_end
                    && f.effect.valid(),
                "food consumption",
            )?;
            require(
                m.assets
                    .iter()
                    .any(|a| a.id == f.asset && a.usage == AssetUsage::FoodPrototype)
                    && m.nodes.iter().any(|n| n.id == f.root && n.asset == f.asset),
                "food asset",
            )?;
            require(
                finite(&[f.mass_kg, f.linear_damping, f.angular_damping])
                    && f.mass_kg > 0.
                    && f.linear_damping >= 0.
                    && f.angular_damping >= 0.
                    && !f.colliders.is_empty(),
                "food mass",
            )?;
            for group in &f.parts {
                require(
                    !group.is_empty()
                        && group
                            .iter()
                            .all(|id| m.nodes.iter().any(|n| n.id == *id && n.asset == f.asset)),
                    "food part",
                )?;
            }
            for c in &f.colliders {
                require(
                    c.membership != 0
                        && m.nodes.iter().any(|n| n.id == c.node && n.asset == f.asset)
                        && super::weather::valid_geometry(&c.geometry),
                    "food collider",
                )?;
            }
        }
        require(
            self.starter_parts.len() == 6
                && self.starter_parts.iter().all(|g| {
                    !g.is_empty()
                        && g.iter().all(|id| {
                            m.nodes
                                .iter()
                                .any(|n| n.id == *id && n.asset == starter.asset)
                        })
                }),
            "starter parts",
        )?;
        require(
            finite(&[
                self.food_loss,
                self.water_loss,
                self.food_damage,
                self.water_damage,
                self.max_healing_speed,
                self.delay_speed,
                self.drink_modifier,
                self.death_velocity_change,
                self.water_to_piss_ratio,
                self.food_to_shit_ratio,
                self.blood_to_hp_ratio,
                self.alcohol_heal,
                self.alcohol_convert_to_piss,
                self.car_fluids_damage,
                self.healing_toilet_full_modfiier,
                self.def_temp,
                self.cold_temp,
                self.hot_temp,
                self.cold_food_loss_modifier,
                self.hot_food_loss_modifier,
                self.cold_water_loss_modifier,
                self.hot_water_loss_modifier,
                self.run_food_loss_modifier,
                self.run_water_loss_modifier,
                self.food_stat_show_time,
                self.food_stat_hide_time,
            ]) && self.food_loss >= 0.
                && self.delay_speed > 0.
                && self.cold_temp < self.def_temp
                && self.def_temp < self.hot_temp,
            "survival rates",
        )?;
        require(
            self.mode == 2 && self.death_seconds.is_finite() && self.death_seconds > 0.,
            "survival mode and death",
        )?;
        let d = &self.drinking;
        require(
            d.evidence.valid()
                && super::weather::valid_geometry(&d.geometry)
                && finite(&d.local_position)
                && finite(&[d.radius, d.valve, d.particle_contact_radius_multiplier])
                && d.radius > 0.
                && d.valve > 0.
                && d.is_drink_cap
                && d.tank == self.toilet.bladder,
            "drinking contacts",
        )?;
        let t = &self.toilet;
        require(
            finite(&[
                t.radius,
                t.valve_gain,
                t.empty_amount,
                t.bowel_fraction,
                t.fart_burp_ratio,
                t.fart_shit_ratio,
                t.min_fart_pitch,
                t.max_fart_pitch,
                t.min_burp_pitch,
                t.max_burp_pitch,
                t.min_fart_volume,
                t.max_fart_volume,
                t.min_burp_volume,
                t.max_burp_volume,
                t.max_rate,
                t.min_rate,
                t.max_speed,
                t.force_color_m,
            ]) && finite(&t.force_color)
                && t.radius > 0.
                && t.min_fart_pitch > 0.
                && t.max_fart_pitch >= t.min_fart_pitch
                && t.min_burp_pitch > 0.
                && t.max_burp_pitch >= t.min_burp_pitch,
            "toilet rules",
        )?;
        require(
            self.inputs.len() == 4
                && ids(
                    self.inputs.iter().map(|i| i.action.as_str()),
                    "input action",
                )?
                .len()
                    == 4
                && self.inputs.iter().all(|i| !i.key.is_empty()),
            "survival inputs",
        )
    }
}
impl SleepReference {
    pub fn validate(&self, _: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.evidence.valid()
                && finite(&[
                    self.need_modifier,
                    self.hold_gain,
                    self.sleep_real_seconds,
                    self.sleep_cycle_seconds,
                    self.cooldown_seconds,
                    self.sleep_cam_ring_min,
                    self.sleep_cam_ring_max,
                    self.sleep_cam_height_min,
                    self.sleep_cam_height_max,
                    self.cloud_low_min_speed,
                    self.cloud_low_mid_speed,
                    self.cloud_low_max_speed,
                    self.cloud_high_min_speed,
                    self.cloud_high_mid_speed,
                    self.cloud_high_max_speed,
                ])
                && self.need_modifier > 0.
                && self.hold_gain > 0.
                && self.sleep_real_seconds > 0.
                && self.sleep_cycle_seconds > 0.
                && self.cooldown_seconds >= 0.
                && self.sleep_cam_ring_min > 0.
                && self.sleep_cam_ring_max >= self.sleep_cam_ring_min
                && self.sleep_cam_height_min > 0.
                && self.sleep_cam_height_max >= self.sleep_cam_height_min
                && self.sleep_layer_mask != 0,
            "sleep rules",
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrinkingReference {
    pub source: SourceId,
    pub tank: SourceId,
    pub collider: SourceId,
    pub geometry: ColliderGeometry,
    pub local_position: [f32; 3],
    pub radius: f32,
    pub valve: f32,
    pub no_angle: bool,
    pub no_pour: bool,
    pub is_drink_cap: bool,
    pub particle_contact_radius_multiplier: f32,
    pub evidence: ManagedEvidence,
}
