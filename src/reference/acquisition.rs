//! Checked acquisition inputs. Gameplay owns draw execution and publication.
use super::*;
fn required_field<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer)
}
pub mod motion;
pub use motion::*;

macro_rules! record {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(#[serde(deserialize_with="required_field")] pub $field: $ty),* }
    };
}
record!(AcquisitionReference {
    stations: Vec<StationAcquisitionReference>, categories: Vec<CategoryReference>,
    r#box: BoxAcquisitionReference, shovel: ShovelAcquisitionReference, packs: Vec<PackAcquisitionReference>,
    startup_packs: Vec<StartupPackReference>, holiday_pumpkin: SourceId, holiday_startup: HolidayPumpkinReference,
    pack_item_id: u32, cigarette_item_id: u32, evidence: AcquisitionEvidence, rules: AcquisitionRules
});
record!(AcquisitionEvidence { assembly_sha256: String, text_sha256: String, methods: Vec<AcquisitionMethod> });
record!(AcquisitionMethod {
    owner: String,
    signature: String,
    sha256: String
});
record!(AcquisitionMarker {
    source: SourceId,
    node: NodeId,
    transform: Transform
});
record!(AcquisitionPose { source: SourceId, parent: Option<SourceId>, coordinate_space: String, transform: Transform });
record!(AcquisitionDistance {
    source: SourceId,
    draw_distance: [f32; 2],
    item_distance: [f32; 2],
    authored_draw_distance: f32
});
record!(StationAcquisitionReference {
    source: SourceId, building: SourceId, asset: AssetId, root: NodeId, item_spawn_distance: AcquisitionDistance,
    building_chance: [f32;2], initial_items_spawned: bool, to_start: Vec<SourceId>, ordinary: Vec<OrdinaryAcquisitionReference>,
    underground: Vec<UndergroundAcquisitionReference>, misc4: Vec<SourceId>, flyers: StationFlyers
});
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionSelection {
    Category {
        name: String,
        category_id: u32,
    },
    Item {
        name: String,
        item_id: u32,
        #[serde(deserialize_with = "required_field")]
        prototype: Option<PrototypeId>,
    },
    Prefab {
        name: String,
        source: SourceId,
        #[serde(deserialize_with = "required_field")]
        item_id: Option<u32>,
        #[serde(deserialize_with = "required_field")]
        prototype: Option<PrototypeId>,
    },
}
record!(CategoryReference {
    id: u32, children: Vec<u32>, categories: bool, weights: Vec<u32>, constructor_weights: Option<Vec<u32>>,
    depend_on_car: bool, constructor_sha256: String, prototypes: Vec<Option<PrototypeId>>, invalid_children: Vec<InvalidCategory>
});
record!(InvalidCategory {
    ordinal: u32,
    target: u32,
    failure: String
});
record!(StationFlyers {
    force: bool,
    halloween: bool,
    vertical_offset: f32,
    vertical_scale: f32,
    horizontal: [f32; 2],
    prefab: SourceId,
    supported: bool
});
record!(AcquisitionTableEntry {
    selection: AcquisitionSelection,
    weight: f32,
    threshold: f32
});
record!(OrdinaryAcquisitionReference {
    source: SourceId, marker: AcquisitionMarker, dont_spawn: bool, holiday: u32,
    building_affects_this: bool, chance: f32, spawned: bool, entries: Vec<OrdinaryAcquisitionEntry>
});
record!(OrdinaryAcquisitionEntry {
    selection: AcquisitionSelection, weight: u32, threshold: u32, places: Vec<AcquisitionPlace>,
    random_rotation: bool, disable_random: bool, initialization: AcquisitionInitialization
});
record!(AcquisitionPlace { weight: u32, threshold: u32, marker: Option<AcquisitionMarker> });
record!(AcquisitionInitialization {
    integer_range: [u32;2], float_range: [f32;2], override_fluid_amount: bool, fluid_range: [f32;2],
    override_fluid_type: bool, fluids: Vec<AcquisitionFluid>, no_mixed_fluids: bool, override_mix_chance: bool, mix_chance: f32
});
record!(AcquisitionFluid {
    r#type: u32,
    weight: f32,
    authored_threshold: f32
});
record!(UndergroundAcquisitionReference {
    source: SourceId, center: AcquisitionMarker, chance: f32, count: [u32;2], radius: [f32;2], vertical: [f32;2],
    table: Vec<AcquisitionTableEntry>
});
record!(BoxAcquisitionReference {
    prototype: PrototypeId, source: SourceId, table: Vec<AcquisitionTableEntry>, disable_spawn: bool, spawn_before_open: bool,
    spawned: bool, opened: bool, spawn_position: AcquisitionPose, authored_spawn_position: Option<SourceId>,
    breakable: SourceId, save: SourceId, break_rules: BoxBreakRules, mount: BoxMountReference, release: Vec<BoxRelease>,
    debris: PrototypeId, reduced_debris: PrototypeId, clips: Vec<SourceId>, disabled_starter_boxes: Vec<DisabledBox>,
    content_counts: Vec<BoxContentCount>
});
record!(BoxBreakRules {
    health: f32,
    decrease_after_attack: bool,
    only_decrease_above: bool,
    decrease_above_health: f32,
    mass_modifier: f32,
    only_velocity: bool,
    no_trigger: bool,
    no_collider: bool,
    no_destroy: bool,
    no_gib: bool,
    destroyed: bool
});
record!(BoxMountReference {
    source: SourceId,
    parent_save: SourceId,
    collider: SourceId,
    only_manual: bool,
    no_physics_lock: bool,
    dont_lock_if_picked_up: bool
});
record!(BoxRelease {
    owner: SourceId,
    method: String,
    target: SourceId,
    target_method: String
});
record!(DisabledBox {
    source: SourceId,
    instance: SourceId,
    disable_spawn: bool
});
record!(BoxContentCount {
    prototype: PrototypeId,
    source: SourceId,
    maximum: u32
});
record!(AcquisitionAmmo {
    source: SourceId,
    count: u32,
    float_count: f32,
    maximum: f32,
    float_or_int: bool,
    r#type: u32,
    random_range: [f32; 2]
});
record!(PackAcquisitionReference {
    prototype: PrototypeId, source: SourceId, ammo: AcquisitionAmmo, cigarette: PrototypeId,
    cigarette_prefab: SourceId, take_one_order: Vec<String>, wearable_type: u32
});
record!(StartupPackReference {
    source: SourceId, pose: AcquisitionPose, active: bool, save: SourceId, item_id: u32, body: SourceId, pickup: SourceId,
    manual_start: bool, started: bool, loaded: bool, buried: bool, car: Option<SourceId>, fixed_in_car: u32,
    prototype: PrototypeId, group: SourceId, group_pose: AcquisitionPose, group_active: bool, vehicle: SourceId,
    pack: SourceId, cigarette_prefab: SourceId, ammo: AcquisitionAmmo
});
record!(HolidayPumpkinReference {
    source: SourceId, pose: AcquisitionPose, active: bool, save: SourceId, item_id: u32, body: SourceId, pickup: SourceId,
    manual_start: bool, started: bool, loaded: bool, buried: bool, car: Option<SourceId>, fixed_in_car: u32,
    prototype: PrototypeId, wearable: SourceId, terrain: SourceId, start: [u32;2], end: [u32;2],
    inclusive: bool, local_calendar: bool, new_game_only: bool, multiplayer: bool
});
record!(ShovelAcquisitionReference {
    prototype: PrototypeId, source: SourceId, pickup: SourceId, target: SourceId, trigger: SourceId, contact_mask: u32,
    speed: f32, shovel_speed: f32, recent_recovery_seconds: f32, attack_volume: f32, attack_clips: Vec<SourceId>,
    contact_clips: Vec<Vec<SourceId>>, motion: AcquisitionMotion, controller: SourceId, right_hand_poses: Vec<SourceId>, left_hand_poses: Vec<SourceId>, contact: ShovelContact
});

record!(ShovelContact {
    force: f32,
    damage: f32,
    damage_tank: f32,
    hurt_food: bool,
    all_particles: f32,
    blood: f32,
    sand: f32,
    plant: f32,
    wood: f32,
    spark: f32
});

fn source_is(s: &SourceId, file: &str, id: i64) -> bool {
    s.file == file && s.path_id == id
}
fn valid_source(s: &SourceId) -> bool {
    s.path_id > 0
        && [
            "level0",
            "level1",
            "sharedassets0.assets",
            "sharedassets1.assets",
        ]
        .contains(&s.file.as_str())
}
fn valid_pose(p: &AcquisitionPose) -> bool {
    valid_source(&p.source)
        && p.parent.as_ref().is_none_or(valid_source)
        && p.coordinate_space == "source_parent_local"
        && valid_transform(&p.transform)
        && checked_pose(p)
}
fn valid_transform(t: &Transform) -> bool {
    t.translation
        .iter()
        .chain(&t.rotation)
        .chain(&t.scale)
        .all(|v| v.is_finite())
        && t.scale.iter().all(|v| *v != 0.)
        && (t.rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() < 0.002
}
fn same_transform(a: &Transform, b: &Transform) -> bool {
    a.translation == b.translation && a.rotation == b.rotation && a.scale == b.scale
}
fn valid_marker(p: &AcquisitionMarker, asset: AssetId, m: &ReferenceManifest) -> bool {
    valid_source(&p.source)
        && valid_transform(&p.transform)
        && m.nodes.iter().any(|n| {
            n.id == p.node
                && n.asset == asset
                && n.source == p.source
                && same_transform(&n.transform, &p.transform)
        })
}
fn prototype<'a>(
    id: &PrototypeId,
    item: Option<u32>,
    m: &'a ReferenceManifest,
) -> Result<&'a ItemPrototype, ReferenceError> {
    m.item_prototypes
        .iter()
        .find(|p| p.id == *id && (item.is_none() || p.item_id == item))
        .ok_or_else(|| ReferenceError("acquisition prototype link".into()))
}

impl AcquisitionReference {
    fn selection(
        &self,
        s: &AcquisitionSelection,
        m: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        match s {
            AcquisitionSelection::Category { name, category_id } => require(
                self.categories.iter().any(|c| c.id == *category_id)
                    && category_name(name) == Some(*category_id),
                "acquisition category link",
            ),
            AcquisitionSelection::Item {
                name,
                item_id,
                prototype,
            } => require(
                name == "box01"
                    && *item_id == 62
                    && m.item_prototypes
                        .iter()
                        .find(|p| p.item_id == Some(*item_id))
                        .map(|p| &p.id)
                        == prototype.as_ref(),
                "acquisition item selector",
            ),
            AcquisitionSelection::Prefab {
                name,
                source,
                item_id,
                prototype,
            } => {
                require(
                    source.file == "sharedassets1.assets"
                        && prefab_identity(source.path_id) == Some((name.as_str(), *item_id)),
                    "acquisition prefab source item link",
                )?;
                let expected = prefab_root(source)
                    .and_then(|id| {
                        m.item_prototypes
                            .iter()
                            .find(|p| source_is(&p.id.0, "sharedassets1.assets", id))
                    })
                    .map(|p| &p.id);
                require(
                    expected == prototype.as_ref(),
                    "acquisition prefab prototype",
                )
            }
        }
    }
    fn table(
        &self,
        entries: &[AcquisitionTableEntry],
        m: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        require(!entries.is_empty(), "acquisition table empty")?;
        let mut threshold = 0.;
        for row in entries {
            self.selection(&row.selection, m)?;
            threshold += row.weight;
            require(
                row.weight.is_finite()
                    && row.weight > 0.
                    && row.threshold.is_finite()
                    && row.threshold == threshold,
                "acquisition table threshold",
            )?;
        }
        Ok(())
    }
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        self.rules.validate(&self.evidence)?;
        require(
            self.pack_item_id == 74
                && self.cigarette_item_id == 230
                && source_is(&self.holiday_pumpkin, "level1", 82),
            "acquisition identities",
        )?;
        require(
            self.categories.len() == CATEGORY_IDS.len()
                && self
                    .categories
                    .iter()
                    .map(|c| c.id)
                    .eq(CATEGORY_IDS.iter().copied()),
            "acquisition category closure",
        )?;
        for c in &self.categories {
            require(
                category_contract(c) && c.prototypes.len() == c.children.len(),
                "acquisition checked category",
            )?;
            let mut invalid = Vec::new();
            for (i, (child, p)) in c.children.iter().zip(&c.prototypes).enumerate() {
                if c.categories {
                    require(p.is_none(), "category cannot be prototype")?;
                    if !self.categories.iter().any(|c| c.id == *child) {
                        let known = match c.id {
                            12 => [186, 265, 303, 387].contains(child),
                            13 => [187, 266, 304, 386].contains(child),
                            _ => false,
                        };
                        require(known, "missing category link")?;
                        invalid.push((i as u32, *child));
                    }
                } else {
                    require(
                        m.item_prototypes
                            .iter()
                            .find(|p| p.item_id == Some(*child))
                            .map(|p| &p.id)
                            == p.as_ref(),
                        "acquisition leaf prototype",
                    )?;
                }
            }
            require(
                c.invalid_children
                    .iter()
                    .map(|v| (v.ordinal, v.target))
                    .eq(invalid)
                    && c.invalid_children
                        .iter()
                        .all(|v| v.failure == "category_index_out_of_range"),
                "native category error terminal",
            )?;
        }
        require(self.stations.len() == 3, "acquisition station count")?;
        for (index, (s, station)) in self.stations.iter().zip(&m.stations.types).enumerate() {
            require(
                s.source == station.source
                    && s.asset == station.asset
                    && s.root == station.root
                    && source_is(
                        &s.building,
                        "sharedassets1.assets",
                        [158348, 155446, 156988][index],
                    ),
                "acquisition station link",
            )?;
            let f = &s.flyers;
            require(
                !f.force
                    && f.halloween
                    && f.vertical_offset == 10.
                    && f.vertical_scale == 5.
                    && f.horizontal == [6., 16.]
                    && source_is(&f.prefab, "sharedassets1.assets", 32394)
                    && !f.supported,
                "station unsupported flyer contract",
            )?;
            let d = &s.item_spawn_distance;
            require(
                source_is(&d.source, "level1", 65886)
                    && d.draw_distance == [500., 25000.]
                    && d.item_distance == [140., 340.]
                    && d.authored_draw_distance == 2500.,
                "acquisition distance settings",
            )?;
            require(
                s.building_chance == [0., 0.] && !s.initial_items_spawned && s.to_start.is_empty(),
                "acquisition building startup",
            )?;
            require(
                s.ordinary
                    .iter()
                    .map(|o| o.source.path_id)
                    .eq(STATION_SPAWNERS[index].iter().copied()),
                "acquisition spawner source order",
            )?;
            let mut misc = Vec::new();
            for o in &s.ordinary {
                require(spawner_binding(o), "acquisition spawner source binding")?;
                require(
                    valid_source(&o.source)
                        && valid_marker(&o.marker, s.asset, m)
                        && o.holiday <= 2
                        && (0.0..=1.).contains(&o.chance)
                        && !o.spawned,
                    "ordinary acquisition source",
                )?;
                let mut threshold = 0u32;
                for e in &o.entries {
                    self.selection(&e.selection, m)?;
                    threshold = threshold
                        .checked_add(e.weight.max(1))
                        .ok_or_else(|| ReferenceError("acquisition threshold overflow".into()))?;
                    require(e.threshold == threshold, "ordinary item threshold")?;
                    let mut place_threshold = 0u32;
                    for p in &e.places {
                        place_threshold = place_threshold
                            .checked_add(p.weight.max(1))
                            .ok_or_else(|| ReferenceError("acquisition place overflow".into()))?;
                        require(
                            p.threshold == place_threshold
                                && p.marker
                                    .as_ref()
                                    .is_none_or(|p| valid_marker(p, s.asset, m)),
                            "ordinary place threshold or marker",
                        )?;
                    }
                    let v = &e.initialization;
                    require(
                        v.float_range
                            .iter()
                            .chain(&v.fluid_range)
                            .chain([&v.mix_chance])
                            .all(|v| v.is_finite())
                            && v.fluids
                                .iter()
                                .all(|v| v.weight.is_finite() && v.authored_threshold.is_finite()),
                        "acquisition initialization numeric fields",
                    )?;
                }
                if o.entries.iter().any(|e| {
                    matches!(
                        e.selection,
                        AcquisitionSelection::Category {
                            category_id: 35,
                            ..
                        }
                    )
                }) {
                    misc.push(&o.source);
                }
            }
            require(
                s.misc4.iter().eq(misc)
                    && s.misc4.len() == [18, 6, 38][index]
                    && s.underground.len() == 1,
                "misc4 and underground counts",
            )?;
            let u = &s.underground[0];
            require(
                source_is(
                    &u.source,
                    "sharedassets1.assets",
                    [154992, 132529, 157204][index],
                ) && valid_marker(&u.center, s.asset, m)
                    && u.chance == 0.4
                    && u.count == [1, 5]
                    && u.radius == [[3., 9.], [5., 11.], [13., 20.]][index]
                    && u.vertical == [-0.15, -0.4],
                "underground source rules",
            )?;
            self.table(&u.table, m)?;
            require(
                table_binding(&u.table, UNDERGROUND_TABLES[index]),
                "underground complete source table",
            )?;
        }
        self.validate_objects(m)?;
        require(
            self.evidence.assembly_sha256
                == "9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e"
                && self.evidence.text_sha256
                    == "22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee"
                && self
                    .evidence
                    .methods
                    .iter()
                    .map(|e| (e.owner.as_str(), e.signature.as_str(), e.sha256.as_str()))
                    .eq(METHODS.iter().copied()),
            "acquisition checked methods",
        )?;
        Ok(())
    }
}

impl AcquisitionReference {
    fn validate_objects(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        let b = &self.r#box;
        let p = prototype(&b.prototype, Some(62), m)?;
        let behavior = &p.behavior;
        require(
            behavior.spawn.as_ref() == Some(&b.source)
                && behavior.breakable.as_ref() == Some(&b.breakable)
                && behavior.debris.as_ref() == Some(&b.debris)
                && behavior.reduced_debris.as_ref() == Some(&b.reduced_debris),
            "Box behavior links",
        )?;
        prototype(&b.debris, None, m)?;
        prototype(&b.reduced_debris, None, m)?;
        self.table(&b.table, m)?;
        require(
            table_binding(&b.table, BOX_TABLE),
            "Box complete source table",
        )?;
        require(
            !b.disable_spawn
                && b.spawn_before_open
                && !b.spawned
                && !b.opened
                && b.authored_spawn_position.is_none()
                && (source_is(&b.spawn_position.source, "sharedassets1.assets", 51722)
                    && valid_pose(&b.spawn_position)),
            "Box spawn rules",
        )?;
        let r = &b.break_rules;
        require(
            r.health == 7.
                && !r.decrease_after_attack
                && !r.only_decrease_above
                && r.decrease_above_health == 0.
                && r.mass_modifier == 0.
                && !r.only_velocity
                && r.no_trigger
                && !r.no_collider
                && !r.no_destroy
                && !r.no_gib
                && !r.destroyed,
            "Box break thresholds",
        )?;
        let mount = &b.mount;
        require(
            behavior.mount.as_ref() == Some(&mount.source)
                && mount.parent_save == b.save
                && source_is(&b.save, "sharedassets1.assets", 145573)
                && p.colliders.iter().any(|c| c.source == mount.collider)
                && mount.only_manual
                && !mount.no_physics_lock
                && !mount.dont_lock_if_picked_up,
            "Box mount links",
        )?;
        require(
            b.release.len() == 2
                && b.release[0].owner == b.breakable
                && b.release[0].method == "Break"
                && b.release[0].target == b.save
                && b.release[0].target_method == "PreDestroy"
                && b.release[1].owner == b.save
                && b.release[1].method == "PreDestroy"
                && b.release[1].target == mount.source
                && b.release[1].target_method == "Unstore",
            "Box release operation links",
        )?;
        require(
            b.clips == behavior.clips && b.disabled_starter_boxes.len() == 8,
            "Box clips and disabled instances",
        )?;
        for (d, (source, instance)) in b.disabled_starter_boxes.iter().zip(DISABLED_BOXES) {
            require(
                source_is(&d.source, "level1", *source)
                    && source_is(&d.instance, "level1", *instance)
                    && d.disable_spawn,
                "disabled starter Box",
            )?;
        }
        require(
            b.content_counts.len() == CONTENT_COUNTS.len(),
            "Box content count table",
        )?;
        for (v, (root, source, maximum)) in b.content_counts.iter().zip(CONTENT_COUNTS) {
            prototype(&v.prototype, None, m)?;
            require(
                source_is(&v.prototype.0, "sharedassets1.assets", *root)
                    && source_is(&v.source, "sharedassets1.assets", *source)
                    && v.maximum == *maximum,
                "Box content count source",
            )?;
        }
        require(
            self.packs.len() == 1 && self.startup_packs.len() == 2,
            "selected packs",
        )?;
        let pack = &self.packs[0];
        let p = prototype(&pack.prototype, Some(74), m)?;
        prototype(&pack.cigarette, Some(230), m)?;
        require(
            p.behavior.pack.as_ref() == Some(&pack.source)
                && p.behavior.cigarette_prefab.as_ref() == Some(&pack.cigarette_prefab)
                && pack.wearable_type == 4
                && pack.take_one_order.iter().map(String::as_str).eq([
                    "decrement_integer_count",
                    "instantiate_at_pack_pose",
                    "initialize",
                    "wear_if_slot_empty",
                ]),
            "pack source rules",
        )?;
        validate_ammo(&pack.ammo, "sharedassets1.assets", 141757)?;
        for (v, expected) in self.startup_packs.iter().zip(STARTUP_PACKS) {
            prototype(&v.prototype, Some(74), m)?;
            require(
                [
                    &v.source, &v.save, &v.body, &v.pickup, &v.group, &v.vehicle, &v.pack,
                ]
                .iter()
                .zip(expected)
                .all(|(s, id)| source_is(s, "level1", *id))
                    && m.vehicles.iter().any(|car| car.source == v.vehicle),
                "selected startup pack sources",
            )?;
            validate_ammo(&v.ammo, "level1", expected[7])?;
            require(
                source_is(
                    &v.pose.source,
                    "level1",
                    if expected[0] == 14079 { 33859 } else { 33862 },
                ) && source_is(
                    &v.group_pose.source,
                    "level1",
                    if expected[0] == 14079 { 21655 } else { 21567 },
                ) && valid_pose(&v.pose)
                    && valid_pose(&v.group_pose)
                    && v.pose.parent.as_ref() == Some(&v.group_pose.source)
                    && v.active
                    && !v.group_active
                    && v.manual_start
                    && !v.started
                    && !v.loaded
                    && !v.buried
                    && v.car.is_none()
                    && v.fixed_in_car == 0
                    && v.item_id == 74
                    && v.cigarette_prefab == pack.cigarette_prefab,
                "startup pack initialization",
            )?;
        }
        let h = &self.holiday_startup;
        prototype(&h.prototype, Some(92), m)?;
        require(
            h.source == self.holiday_pumpkin
                && source_is(&h.wearable, "level1", 79857)
                && source_is(&h.terrain, "level1", 64898)
                && source_is(&h.pose.source, "level1", 21449)
                && valid_pose(&h.pose)
                && h.start == [10, 15]
                && h.end == [11, 15]
                && h.inclusive
                && h.local_calendar
                && h.new_game_only
                && !h.multiplayer
                && !h.started
                && !h.loaded
                && !h.buried
                && h.car.is_none()
                && h.fixed_in_car == 0
                && h.item_id == 92
                && source_is(&h.save, "level1", 72331)
                && source_is(&h.body, "level1", 50268)
                && source_is(&h.pickup, "level1", 80624)
                && h.active
                && h.manual_start,
            "holiday startup source",
        )?;
        let s = &self.shovel;
        require(
            s.controller == s.motion.controller
                && s.right_hand_poses.is_empty()
                && s.left_hand_poses.is_empty(),
            "shovel controller or hand pose arrays",
        )?;
        let c = &s.contact;
        require(
            c.force == 20000.
                && c.damage == 40.
                && c.damage_tank == 12.
                && c.hurt_food
                && c.all_particles == 0.01
                && c.blood == 0.5
                && c.sand == 2.
                && c.plant == 1.
                && c.wood == 0.5
                && c.spark == 0.5,
            "shovel contact inputs",
        )?;
        let p = prototype(&s.prototype, Some(216), m)?;
        let behavior = &p.behavior;
        require(
            behavior.melee.as_ref() == Some(&s.source)
                && behavior.attack_target.as_ref() == Some(&s.target)
                && behavior.attack_trigger.as_ref() == Some(&s.trigger)
                && p.pickups
                    .first()
                    .is_some_and(|p| p.pickup.source == s.pickup)
                && s.contact_mask == 8552961
                && s.speed == 0.7
                && s.shovel_speed == 0.5
                && s.recent_recovery_seconds == 1.
                && s.attack_volume == 0.5
                && s.attack_clips == behavior.clips
                && s.contact_clips == behavior.contact_clips,
            "shovel source contact rules",
        )?;
        for clip in b
            .clips
            .iter()
            .chain(&s.attack_clips)
            .chain(s.contact_clips.iter().flatten())
        {
            require(
                m.audio["clips"].as_array().is_some_and(|clips| {
                    clips.iter().any(|c| {
                        c["source"]["file"].as_str() == Some(clip.file.as_str())
                            && c["source"]["path_id"].as_i64() == Some(clip.path_id)
                    })
                }),
                "acquisition audio resource",
            )?;
        }
        s.motion.validate(m)
    }
}
fn validate_ammo(v: &AcquisitionAmmo, file: &str, id: i64) -> Result<(), ReferenceError> {
    require(
        source_is(&v.source, file, id)
            && v.count == 20
            && v.float_count == 0.
            && v.maximum == 20.
            && !v.float_or_int
            && v.r#type == 5
            && v.random_range == [20., 1.],
        "pack count and reversed range",
    )
}

const CATEGORY_IDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
    53, 54, 55, 56, 57, 58, 59, 60, 61, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
    78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
];
const STATION_SPAWNERS: &[&[i64]] = &[
    &[
        155092, 131522, 139976, 155127, 145959, 156024, 152647, 152199, 144347, 156794, 152085,
        152852, 143968, 152401, 136189, 159015, 132525, 157727, 144982, 148080, 154174, 148737,
        137140, 158935, 154247, 141801, 157379, 159136, 145318, 143532, 144808, 142945, 135698,
    ],
    &[
        153352, 135774, 150475, 146976, 144588, 143922, 147378, 145695, 143142, 140022, 142385,
        130795, 131645, 134362, 149222, 143195, 158837, 148932, 150294, 131020, 153823, 137208,
        153740, 153176, 139372, 150978, 134469, 144495, 155447, 159356, 153506, 149796, 152316,
        140091, 140159, 158497, 135875,
    ],
    &[
        133894, 132473, 134881, 152604, 143849, 141037, 134594, 155658, 155735, 131807, 148462,
        133453, 151325, 159388, 151878, 156241, 150554, 152271, 134832, 152756, 149070, 143444,
        144153, 134567, 132539, 131865, 148411, 133089, 153078, 135175, 153070, 151379, 138654,
        156146, 138632, 134371, 131777, 142598, 147640, 136433, 152320, 152354, 149997, 134150,
        141763, 145394, 131269, 149343, 154365, 132040, 143201, 142584, 146500, 140319, 155187,
        138169, 138563, 152202, 132607, 133602, 151471, 139581, 130933, 146077, 152232, 140250,
        136758, 144321, 143910, 158529, 147544, 157221, 137563, 157472, 140306, 133988, 156824,
        149279, 154875, 153360, 134606, 153100, 151479, 141788, 134009, 134004, 151772, 154990,
        139952, 143756, 159395, 152486, 159108, 149708, 143441, 148956, 133810, 137616, 156686,
        151104, 141085, 149001, 156759, 144858, 155522, 151810, 142649, 158222, 135069, 142695,
        135361, 149754, 156893, 142419, 152913, 156419, 130875, 146654, 154127, 135643, 152942,
        158813, 139621, 136713, 156175, 140582, 138065, 135847, 154094, 138584, 146062, 142303,
        132513, 138338, 143718, 135235, 151295, 144008, 153794, 135332, 132925, 136321, 136599,
        149893, 133770, 148179, 134021, 138866, 150213, 137377, 148616, 148291, 150601, 136148,
        132645, 154037, 137393, 141383, 140922, 153038, 149590, 157508, 133298, 157793, 144300,
        140956, 145078, 144514, 148891, 151014, 143615, 136463, 154073, 158683, 153893, 137745,
        151995, 135598, 130805, 155482, 155305, 143582, 156675, 131215, 159227, 140073, 140141,
        151054, 149896, 137792, 153493, 140991, 139566, 131889, 142600, 142749, 153239, 149396,
        138457, 151905, 148855, 156815, 142703, 131517, 131910, 131490, 156814, 136246, 153963,
        154490, 158555, 147609, 139150, 154895, 143647, 139312, 139214, 147519, 153824, 137672,
        153505, 157398, 149671, 136219, 140191, 150410, 136300, 132674, 152360, 159041, 154385,
        148893, 156861, 155859, 145390, 141891,
    ],
];
fn category_name(name: &str) -> Option<u32> {
    match name {
        "car" => Some(0),
        "kaszni" => Some(1),
        "carfull" => Some(2),
        "ldoor" => Some(3),
        "rdoor" => Some(4),
        "door" => Some(5),
        "fbumper" => Some(6),
        "rbumper" => Some(7),
        "bumper" => Some(8),
        "hood" => Some(9),
        "trunk" => Some(10),
        "hoodtrunk" => Some(11),
        "lmirror" => Some(12),
        "rmirror" => Some(13),
        "fmirror" => Some(14),
        "mirror" => Some(15),
        "lnapellenzo" => Some(16),
        "rnapellenzo" => Some(17),
        "napellenzo" => Some(18),
        "rim" => Some(19),
        "tire" => Some(20),
        "hubcap" => Some(21),
        "rimtire" => Some(22),
        "wheel" => Some(23),
        "wheelstuff" => Some(24),
        "attachable" => Some(25),
        "coolant" => Some(26),
        "magazin" => Some(27),
        "kesztyutarto" => Some(28),
        "radio" => Some(29),
        "headlight" => Some(30),
        "kmh" => Some(31),
        "misc1" => Some(32),
        "misc2" => Some(33),
        "misc3" => Some(34),
        "misc4" => Some(35),
        "turd" => Some(36),
        "engine" => Some(37),
        "utanfuto" => Some(38),
        "bus" => Some(39),
        "busengine" => Some(40),
        "buscoolant" => Some(41),
        "bushoodtrunk" => Some(42),
        "buskmh" => Some(43),
        "carkmh" => Some(44),
        "krearseat" => Some(45),
        "bed" => Some(46),
        "beds" => Some(47),
        "compasses" => Some(48),
        "instruments" => Some(49),
        "paints" => Some(50),
        "washtools" => Some(51),
        "miscwashtools" => Some(52),
        "ldoorr" => Some(53),
        "rdoorr" => Some(54),
        "frontframes" => Some(55),
        "bikekmh" => Some(56),
        "biketrunk" => Some(57),
        "bikemisc" => Some(58),
        "bike" => Some(59),
        "bike01" => Some(60),
        "plates" => Some(61),
        "smalldoors" => Some(62),
        "bigdoors" => Some(63),
        "bus02kmh" => Some(64),
        "clocks" => Some(65),
        "toys" => Some(66),
        "melee" => Some(67),
        "ammo" => Some(68),
        "bodypart" => Some(69),
        "food" => Some(70),
        "balls" => Some(71),
        "shotgun" => Some(72),
        "bigfirearm" => Some(73),
        "machineguns" => Some(74),
        "smallfirearms" => Some(75),
        "firearms" => Some(76),
        "humanmonster" => Some(77),
        "flashlights" => Some(78),
        "books" => Some(79),
        "weapon" => Some(80),
        "biginstruments" => Some(81),
        "lengthy" => Some(82),
        "cans" => Some(83),
        "wearables" => Some(84),
        "inbox" => Some(85),
        "bikeTanks" => Some(86),
        "bike03" => Some(87),
        "rodthings" => Some(88),
        "trophies" => Some(89),
        _ => None,
    }
}
fn prefab_root(source: &SourceId) -> Option<i64> {
    if source.file != "sharedassets1.assets" {
        return None;
    }
    match source.path_id {
        32825 => Some(75443),
        9085 => Some(51722),
        18963 => Some(61587),
        39185 => Some(81642),
        39188 => Some(81645),
        43845 => Some(70812),
        24481 => Some(53838),
        24480 => Some(53837),
        20981 => Some(63605),
        20649 => Some(63259),
        11985 => Some(54495),
        _ => None,
    }
}
fn category_contract(c: &CategoryReference) -> bool {
    match c.id {
        0 => {
            c.children == &[1, 2]
                && c.categories == true
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "164553d9ba67111f6a177268eac5ac894e6fd8ab71eeddf35020fcd642a03115"
        }
        1 => {
            c.children
                == &[
                    1, 15, 98, 29, 94, 122, 153, 222, 243, 257, 275, 309, 337, 356, 371, 390,
                ]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "11c16d00d37603d6a41799373f7415ffd4dd80d03c1d3ebadeb04a1b429ab0fa"
        }
        2 => {
            c.children
                == &[
                    2, 16, 99, 30, 95, 123, 154, 223, 244, 272, 276, 314, 338, 357, 372, 388, 391,
                ]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "16dcc7ee8d46326f03f364ce8b91ab97a42756c8f6724a04740352d2f26513c5"
        }
        3 => {
            c.children == &[3, 17, 31, 124, 160, 181, 232, 247, 258, 296, 310, 339, 385]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "99c6a8dfe27c3ab41739cf243c320e50e91ab41cd4515a9b9b621b46f1625e43"
        }
        4 => {
            c.children == &[4, 18, 32, 125, 158, 182, 233, 248, 259, 297, 311, 340, 384]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "4bbeccced313f56772f0c044cdc7c1a50213d879933cb70a36c799a9967d0204"
        }
        5 => {
            c.children == &[3, 4, 53, 54]
                && c.categories == true
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "a54443fab8925ff5734e7502a6072ab758d3087e6b6de9e78696ee6537c977b3"
        }
        6 => {
            c.children == &[7, 21, 33, 128, 161, 188, 236, 262, 298, 351, 378]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "1a6949daaea828ca7986158f4cc913e0801c3395a888e999bc625f3f934dd451"
        }
        7 => {
            c.children == &[8, 22, 34, 129, 162, 237, 246, 263, 299, 352, 379]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "7253b51206eba5fd4ef92df506d4d0196c28d933a17eca214f59d6c4f31dfbb9"
        }
        8 => {
            c.children == &[6, 7, 55]
                && c.categories == true
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "f54dccddcb99d1b361ccdb60c8d4fc529b3a429e0462a5942d73a8b5da25bd30"
        }
        9 => {
            c.children == &[9, 23, 35, 130, 163, 234, 252, 260, 301, 344]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "61d4004ce52b7cb00ac049ea77dfd5d672aaaab75b0a8cc7a67cee84793707f1"
        }
        10 => {
            c.children == &[10, 24, 100, 36, 96, 131, 164, 235, 249, 261, 302, 343, 360]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "b2eed83f96b0b933784ba27dedbd4102ab6c777eca2eb547bf58d60759d0076a"
        }
        11 => {
            c.children == &[9, 10, 42, 57]
                && c.categories == true
                && c.weights == &[100, 100, 50, 50]
                && c.constructor_weights.as_deref() == Some(&[100, 100, 50, 50])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "9e68fe8da2504ca1e3602d40b7c62d9d2132764b931639cb2162d28bcdb38d8f"
        }
        12 => {
            c.children == &[41, 186, 265, 303, 387]
                && c.categories == true
                && c.weights == &[100, 50, 50, 50, 40]
                && c.constructor_weights.as_deref() == Some(&[100, 50, 50, 50, 40])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "8cd08a2b0b4e02513ad4c81ed62d36c0db70730c8b2dc1416d193b750adfed92"
        }
        13 => {
            c.children == &[42, 187, 266, 304, 386]
                && c.categories == true
                && c.weights == &[100, 50, 50, 50, 40]
                && c.constructor_weights.as_deref() == Some(&[100, 50, 50, 50, 40])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "13417bd9167032344c5a238825cdfa1da69d481172c6308dbc2d964b953f28fd"
        }
        14 => {
            c.children == &[40]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "6b71609ce02d931ca59aa36563cb81ef84184c3797dc607eea6ba7daeb84e7f0"
        }
        15 => {
            c.children == &[12, 13, 14]
                && c.categories == true
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "0eb5ba3d51ee4dfd3a5866db955144907165df2ef3b356bdff22a5890ec1509f"
        }
        16 => {
            c.children == &[49]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "a8e7a7c038c6b550602de0c6f63f21e7707403e11be104be049e640d3e9feb6e"
        }
        17 => {
            c.children == &[50]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "6d8a8f7070206ac84c83a24baa954fc350ce13f4b0c43082509ce9fe06fe5a59"
        }
        18 => {
            c.children == &[16, 17]
                && c.categories == true
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "e223f1d8b04dc6330a88a95c53687f5488c7f4ed23f2705713c567bf9ac4f4f1"
        }
        19 => {
            c.children == &[43, 108, 142, 366]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[100, 50, 25, 25])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "d5bb70fa8a563bad4f8c3e68282defbc9c54acb0916e9aab8baed2717f8c1de3"
        }
        20 => {
            c.children == &[44, 109, 143, 268]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[100, 50, 25, 25])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "6b2ccf1a2549ae2814ea0bfd3b811ffde618685aa20e30b95f21b8904788769f"
        }
        22 => {
            c.children == &[47, 274]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "607afb68da62ed2126feba47116b72cbd5efdd2128f381cb58b6959a853ba9db"
        }
        23 => {
            c.children == &[46, 112, 144, 273, 367, 389]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[52, 50, 20, 25, 20, 52])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "e69852615e23c585f785c587c3a3b8f08cb0c2b8a5b7e52a17b96d3c76cf88e5"
        }
        24 => {
            c.children == &[23, 22, 20, 19]
                && c.categories == true
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "ab1c070bf3cbafa07f862a935d623e7ec199de995e670ef57a1127f10db89678"
        }
        25 => {
            c.children == &[54, 55, 56, 57, 59, 93, 102]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "45a36a190a50de54ab2604e715edcabb2e721279f3db8ab614d66c9f95b6b78a"
        }
        26 => {
            c.children == &[12, 26, 133, 156, 242, 256, 346]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "977b372676267dad3c38e171d69f97470e1169dd0be91c225c0bf4974ad137d0"
        }
        27 => {
            c.children == &[79]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "d81495db1020bde5278ac44357f03d5b8f6b833e560a882fca54f1c9e2aad51d"
        }
        28 => {
            c.children == &[14, 28, 135, 167, 185, 238, 264, 300]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "4e47b187f8698d4d05a71176a3f7a844f3e0f7409492462788db33e00397d431"
        }
        29 => {
            c.children == &[39, 267]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "883d94e742699e041f65a67f18dae0f5f3eb6ba705daec2e3840cba9f15dc30c"
        }
        30 => {
            c.children == &[48, 253, 104, 137, 147, 165, 166, 348]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[80, 80, 50, 50, 50, 25, 25, 25])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "1ea527584599fb161f9a0a72b094bfcb1011ffb0d6c405ec2a2e167e6c73560a"
        }
        31 => {
            c.children == &[44, 43, 56, 64]
                && c.categories == true
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "b0edca771eba3c1de81b8483cd164891888c751ec1460fa9456260d5481069a9"
        }
        32 => {
            c.children == &[74, 172, 217, 218, 220, 245, 305, 306]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[10, 20, 70, 30, 10, 10, 10, 10])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "753b7d68a2b4ab54889e5a952c41d5623675f4707b994d01a5dadc18d7087e88"
        }
        33 => {
            c.children == &[15, 18, 30, 31]
                && c.categories == true
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "54922df1f6dacad6271501256768bc5de5a3b87a9f196d828a34afcc9a7f8c54"
        }
        34 => {
            c.children == &[28, 27, 29, 61, 66, 71, 79]
                && c.categories == true
                && c.weights == &[1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "bc5ff56a91d40b40561e03bb8e89eb77e32e8a72d8f32895cde7879419f77a0d"
        }
        35 => {
            c.children == &[32, 33, 34, 49, 50, 51, 58, 68, 70, 78, 89]
                && c.categories == true
                && c.weights == &[20, 20, 20, 5, 35, 25, 30, 15, 50, 20, 3]
                && c.constructor_weights.as_deref()
                    == Some(&[20, 20, 20, 5, 35, 25, 30, 15, 50, 20, 3])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "bc9cfd6cf3f68a3d76114ed70c3a173149e87fd797a223c2ce0af322ea3f0802"
        }
        36 => {
            c.children == &[84]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "778b13cc4877e06e1931cbe5b1ea2a6d8b386e6fee16a17234bea3e86643289a"
        }
        37 => {
            c.children == &[11, 25, 37, 132, 155, 240, 241, 255, 295, 345]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "e4bb3582c6bdd466debb680ced6fb3747a9f601c2da4eb9acb6d074e9cbc1b14"
        }
        38 => {
            c.children == &[52, 53]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "f92599892cb89b318e7201b7c7e410a2296c5b4feae0e2d00af56b160cf8bd7b"
        }
        39 => {
            c.children == &[105, 106, 173, 174, 178, 179, 193, 194, 396, 397]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref()
                    == Some(&[15, 30, 40, 50, 20, 50, 30, 40, 30, 40])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "2c58ebdeb731e648b0377bb74cf4219f7cbe51efcd5034dce492c7e19af7303d"
        }
        41 => {
            c.children == &[110, 176]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "c16028bb33ca63a000052f55d68a4b2cf94b6bee47a15e26ad833ca60e07f790"
        }
        42 => {
            c.children
                == &[
                    107, 183, 184, 195, 191, 380, 382, 377, 376, 375, 381, 393, 394, 395,
                ]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "3aeaa67e7a6e1fd1ce0496d46aef58c973cc084b5a935f669fa15b7a4123a061"
        }
        43 => {
            c.children == &[113, 114, 115]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "f1be909f9b55612eb1df51d3ccf6e16f9e688c12b3408845443334b2afbf97a1"
        }
        44 => {
            c.children
                == &[
                    13, 27, 38, 134, 138, 139, 168, 169, 250, 251, 270, 307, 308, 349, 350, 355,
                    398,
                ]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "141a0a43b7f2950615138bc6462a064c51856d7372d0aebb7752a06935a35242"
        }
        45 => {
            c.children == &[101, 97, 361, 373]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "cece59f1b8d70a9a02e58f115c5fcf514e7da68b74b3ab44b00effc9c86901e1"
        }
        46 => {
            c.children == &[89, 383]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == Some(&[85, 15])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "940349fd995e5e499aeb7d9b65339143412cb7adbb74f88e136a9d83f602cde7"
        }
        47 => {
            c.children == &[46, 45]
                && c.categories == true
                && c.weights == &[75, 25]
                && c.constructor_weights.as_deref() == Some(&[75, 25])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "79cb58636bcb11450011e63df4679b27a745d1ad483a4e813133e9cf6f437af9"
        }
        48 => {
            c.children == &[116]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "54bd2d75c66d453be4307f62a565232549cf5f678cf982700f6eb0da25f194dc"
        }
        49 => {
            c.children == &[48, 65]
                && c.categories == true
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "7a755de69d8b52797843a104f70aab6565c7ffea30655653797ab7ee3df141e5"
        }
        50 => {
            c.children == &[117]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "1d2f13c0a9d999bc8c424341e8c4ef76e3c0b8ff09750f16b9b82c80cb7834d6"
        }
        51 => {
            c.children == &[118, 119, 121]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "fffaeb5b96a5a8cd5146a8d863ca71b1bf45a2669fb648b35c24fffb40f68670"
        }
        53 => {
            c.children == &[5, 19, 126, 159, 342, 359]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "4c3028def489739c010dcd2f027de473fee215d2049b9db8f968bf07baa3a411"
        }
        54 => {
            c.children == &[6, 20, 127, 157, 341, 358]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == true
                && c.constructor_sha256
                    == "fdd7147c24f2c1d59056b624c41cc22dd8d6590fcfb1467ad64a657620344ee7"
        }
        55 => {
            c.children == &[136, 239, 347]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "ab61fe8f11805a1f027f1b7d3a3858a728678136642347a12c2f7ba13f0e9d8a"
        }
        56 => {
            c.children == &[151, 364]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "015f64d6d7da41adaa4d572d88e82c27700299fbdf1a7efe98395fa05850d15a"
        }
        57 => {
            c.children == &[148, 149, 365, 369]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[8, 5, 5, 8])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "cc1ec09a7fb16803642f2e127ca7a26b6faf634e882c481db080f1b413b7a95a"
        }
        58 => {
            c.children == &[145, 146, 148, 362, 363]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "a09dd3fd8c062eb5f53a072369ca84f731f38fe41feef3526ef6cbb9a388b41f"
        }
        59 => {
            c.children == &[60, 87]
                && c.categories == true
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "7b03dd8735fa7399aa431c7f9dc5a8a28d8606b2239f9edc22359f6bb6bc67fd"
        }
        60 => {
            c.children == &[140, 141]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == Some(&[10, 5])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "fed7c46410a421aed14bb094a6d88eef22b09bf0f785fe238a838742fa2f2864"
        }
        61 => {
            c.children == &[177]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "c4480aaaec89e52d6a307f5b689ad4d789e26fdb66fbea25a00c6d207ac3abe0"
        }
        63 => {
            c.children == &[190, 377, 376, 375]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "47bb006658673bfedc24fc3f9273407418ebf158c27e1fcb726f4f89cf80a3d5"
        }
        64 => {
            c.children == &[196, 197, 198, 199]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "c14e453b9ab389d59dc8c7fcda72ec690aebb23cf3766e52779764ce8b636b38"
        }
        65 => {
            c.children == &[200]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "877de4f7eba9e5d0b9a5f74214b9fe48c3e66b4e2571a75fb3564ef6e0bc99e7"
        }
        66 => {
            c.children == &[180, 353]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "c5866be3dae7f5e28679ba493790176101cc461f094d642ca1088d4e22a65355"
        }
        67 => {
            c.children == &[63, 212, 216]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "9800e427cbc34d57088f8e0b056b1ad7f40700f9e60235a847bc8f59d25de6f5"
        }
        68 => {
            c.children == &[76, 213, 215, 279]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[200, 100, 75, 75])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "0503d01a8ec907e8ee564d59ae9c281c296114cdaf6f43b4f5c214ab3179ad0f"
        }
        69 => {
            c.children == &[170, 219]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "148d2e4f9be6b42274ea78ebe48f0d5eb5f28f6a9479bfeeeed163c348bc1b51"
        }
        70 => {
            c.children == &[224, 225, 226, 227, 228, 229]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[20, 60, 60, 40, 30, 100])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "b4078d01b818a6f897a0b3e94d0946daac1d75b22e88ec0db3f3b132467bc49a"
        }
        71 => {
            c.children == &[271]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "ad1a864f4d5267518aff8fe6515dd4355b5bd627a27dc2fe970aaaa535910d01"
        }
        72 => {
            c.children == &[278, 292, 293]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[100, 100, 100])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "626616c39d60cf7ae1cc2869c3e0290e97714478b778b4ebc4f925ffc1e1c7cc"
        }
        73 => {
            c.children == &[72, 74]
                && c.categories == true
                && c.weights == &[100, 100]
                && c.constructor_weights.as_deref() == Some(&[100, 100])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "708498735fa4fe57cdaaa880becde0ba312216cb457944acc5d4f33b6fb46aa1"
        }
        74 => {
            c.children == &[214]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "d645214424350beed7977eed1bf606cea0ad6acd1c0ec0bdb23f1173a6d20642"
        }
        75 => {
            c.children == &[64]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "3aab03171fd40f3789054531e4d1157aa6e01baf125f944f3be9394f1a859e09"
        }
        76 => {
            c.children == &[73, 74, 75]
                && c.categories == true
                && c.weights == &[100, 100, 100]
                && c.constructor_weights.as_deref() == Some(&[100, 100, 100])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "b62239d7c68b2e1595b9fa8ec51c2b7b0c8a131920dca513e27220af1bb1f7cc"
        }
        77 => {
            c.children == &[317]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "53175a530d0bdd73e9437c45df6a3b7faca187bc4874f0c4a78ec4bda82a636d"
        }
        78 => {
            c.children == &[66, 320]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "82b46d1db7f2b4f7ade5e0ae1a1f920b1713e5b0ff39617c5abc28fb7d0b4cdb"
        }
        79 => {
            c.children == &[333]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "ad2b97a19d5456f69e39610d40d09b8ea8cd5a5036d6e2e747f0e395dd787dd7"
        }
        80 => {
            c.children == &[76, 67]
                && c.categories == true
                && c.weights == &[100, 100]
                && c.constructor_weights.as_deref() == Some(&[100, 100])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "dade40c7f4f75d262e5b8a25840da8f7b9778af9afdfcaffc32e7b9100eb0df8"
        }
        81 => {
            c.children == &[354]
                && c.categories == false
                && c.weights == &[1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "1689ef647ae7448eb93d89c9acbf13d77d8bf0e37aa5fdb6bdc1f456ed9ca507"
        }
        82 => {
            c.children == &[80, 81]
                && c.categories == true
                && c.weights == &[100, 33]
                && c.constructor_weights.as_deref() == Some(&[100, 33])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "51276cca1d3d47a2f34a674c1bbacbbb93e31388cd6e6befca229e3f95601ce2"
        }
        83 => {
            c.children == &[72, 60]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "f741ab5a34269a65a9dc57e301f60206bfdc1bb614161556fd06f099955ea797"
        }
        84 => {
            c.children == &[217, 218, 245, 221]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "d92c8f132c88f148637a918abc1d5fd853ffa401dbf1a0dd2bd8e3f4e91c084d"
        }
        85 => {
            c.children
                == &[
                    202, 75, 211, 152, 73, 306, 65, 83, 84, 82, 69, 201, 74, 82, 79,
                ]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "1d9da9ca3bbe380a3d036e5e9bc3cdfbdafd975aa768f3af8d70ffbd1082a0f9"
        }
        86 => {
            c.children == &[362, 363, 370, 392, 399]
                && c.categories == false
                && c.weights == &[1, 1, 1, 1, 1]
                && c.constructor_weights.as_deref() == None
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "f4501ac2be2fb90bfe2df588cff2844e3a47ca8a9100d9c41e77a588bc76cdb4"
        }
        87 => {
            c.children == &[335, 336, 368]
                && c.categories == false
                && c.weights == &[1, 1, 1]
                && c.constructor_weights.as_deref() == Some(&[20, 5, 5])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "bd110a2d0dcf28ae0935809f3c3cd25b0b9f3929e295cc3f61bdad85225e3aec"
        }
        88 => {
            c.children == &[171, 401]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == Some(&[80, 20])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "9c38dae79eb916f794e23129e0362ae8401c2fecf192cd3535f6da43f140cb26"
        }
        89 => {
            c.children == &[91, 400]
                && c.categories == false
                && c.weights == &[1, 1]
                && c.constructor_weights.as_deref() == Some(&[50, 50])
                && c.depend_on_car == false
                && c.constructor_sha256
                    == "4540f733c3021f2abf1d41c9f9e2747da553247f2f714f1a31faf5f68356e808"
        }
        _ => false,
    }
}
const DISABLED_BOXES: &[(i64, i64)] = &[
    (79792, 1904),
    (79793, 1902),
    (79794, 1903),
    (79795, 1909),
    (79796, 1906),
    (79797, 1905),
    (79798, 1908),
    (79799, 1907),
];
const CONTENT_COUNTS: &[(i64, i64, u32)] = &[
    (75443, 132605, 10),
    (51722, 136544, 0),
    (61587, 156405, 0),
    (70812, 134682, 5),
    (53838, 147662, 3),
    (53837, 147661, 3),
    (63605, 134243, 3),
    (63259, 141432, 0),
    (54495, 134663, 0),
];
const STARTUP_PACKS: &[[i64; 8]] = &[
    [14079, 72692, 50432, 75069, 298, 69517, 70704, 70998],
    [14082, 72693, 50434, 75072, 206, 77839, 70705, 70999],
];
const METHODS: &[(&str, &str, &str)] = &[
    (
        "itemdatabase",
        "public class category",
        "b9b61ee3764d32d237916d474e2c6e3d31b3af739542ed2fda19a23a8b8c905e",
    ),
    (
        "itemdatabase",
        "private void Awake()",
        "22061576aceb7e2f81644f4fb933185beed315a8cd96ebbf03500872d4f8707b",
    ),
    (
        "buildingscript",
        "public void CheckDist()",
        "f9a89c3b683110ad99f25f146b50522e30b912d8c4a7e932d5e63db39eaef409",
    ),
    (
        "buildingscript",
        "public void SpawnStuff(int pId)",
        "c852c28b313f23d8e001a4f4567cb97029393689b55dcaca3db73985c73ca020",
    ),
    (
        "buildingscript",
        "private IEnumerator SPAWN()",
        "ea043ea67b565b933b3041fe616552991901ba4a157d8ecedf5bfa5d73620c24",
    ),
    (
        "buildingscript",
        "public static void RandomFlyerSpawn(bool _forceRandomFlyer, bool _randomFlyerOnHalloween, Vector3 _pos, float _offsetty, float _vertical, float _horizontalMin, float _horizontalMax)",
        "b07739fabe3e11eea742682b8ca8bddaecd0127d6f3a049c89f7545a56bd845f",
    ),
    (
        "newRandomStuffSpawnScript",
        "public void Spawn()",
        "cb0ee0cbf20c6477464f14ede286624d783e06b033ed96f36527a16edde311a3",
    ),
    (
        "newRandomStuffSpawnScript",
        "public void InitSpawn()",
        "8b49cde8582fd0df9fd24a186e87dad6572f57ae597c7f85f1b86f5412443d99",
    ),
    (
        "newRandomStuffSpawnScript",
        "public void SelectSpawn()",
        "7a4695625546bb5ca6a9608ea3cca3aa45b2a3e36c743faec940a995acf7601e",
    ),
    (
        "newRandomStuffSpawnScript",
        "public void SpawnItem()",
        "d3b2b7bff0d643afb0bc0e9c46ff0ba4e222c7975700f1d466c926aad04e0c4a",
    ),
    (
        "newRandomStuffSpawnScript",
        "public static int SelectItem(string _categoryName, bool _isOneItem)",
        "30333202a88e361d1a297ecc4260c6a1747e6de2d8c544d6d90198fe910c2108",
    ),
    (
        "newRandomStuffSpawnScript",
        "public static int selectId(int categoryId)",
        "8f20129ab487b00b6a43d00a1d827cc2702c205042503d7b2db822568649e9fd",
    ),
    (
        "undergroundSpawnScript",
        "private void Init()",
        "f55feec80ca147429458381487134c7b960e9915265ff88d98bc335c592cacb7",
    ),
    (
        "undergroundSpawnScript",
        "public void Spawn()",
        "307e136e3b4237c71f14908ca62fedd3f62e8800ccaf4f6e186c72e2658e2a0a",
    ),
    (
        "undergroundSpawnScript",
        "public Vector3 RandomPoint()",
        "02efe82674d916dc21ae4bbc2d942ae2d847069b9ccee8778e62ae87948c8ccf",
    ),
    (
        "spawnInBoxScript",
        "public void FStart()",
        "b4a8f1852338e5819ba02c84fc65832cde84eda60db40cb65acc0eda57ed9be5",
    ),
    (
        "spawnInBoxScript",
        "public void Spawn()",
        "96b8b084e82c28cdf39b5ae1772dc74ac33c5189570a40ae753f35e45e0a70d8",
    ),
    (
        "spawnInBoxScript",
        "public void Opened()",
        "8d52079846a3567f0f75f94a0e86acda2386856bbe7415851f48c23a72b1976b",
    ),
    (
        "breakablescript",
        "private void Start()",
        "9753ebe873cf5ce7c315276df93901da01c38e3078f507f5a9025dd3dd872d51",
    ),
    (
        "breakablescript",
        "private void OnCollisionEnter(Collision c)",
        "bb98ef04b1aaab4aceb4db24a3f37a3d725db7a14317109023abe3a332a99555",
    ),
    (
        "breakablescript",
        "public void TryBreak(float force)",
        "b93980330be86c796b96ee6c133d5566e1c3dad6a7341e9999f478136604340a",
    ),
    (
        "breakablescript",
        "public void Break()",
        "96993e34070a480e03ea9f7078943c885f40f0065be8a643ba300bf71441b04f",
    ),
    (
        "mountStuff",
        "public void Unstore()",
        "90e704fe9745e42b01e2d3d7bc9df9a81bafaa6e1fb054e338417daf07495c6e",
    ),
    (
        "tosaveitemscript",
        "public void Bury()",
        "684fbee4e2b10d87afa0dcc531bd17829dceaa71f3204c91004e66fff6b5996f",
    ),
    (
        "tosaveitemscript",
        "public void UnBury()",
        "d7c4139e456207ed942b6578533ebba65f875870cd3ce2c2ffc758c956c1b5de",
    ),
    (
        "tosaveitemscript",
        "public bool IsRecentlyUnburied()",
        "a8274fc43289477a288c1eca56c86b6c1216809a233df37dda75ca76302fbde1",
    ),
    (
        "tosaveitemscript",
        "public void UpdBuried(float _shovelSpeed)",
        "ea559f724cbc5a3f79b3197e7335060214dbdd2be3ccbfedcfa9d339d54c66fb",
    ),
    (
        "tosaveitemscript",
        "public bool IsBuried()",
        "e8cc70ae7a846f17e558e7f08e2775e433e678343158e1cf8d35403a8aaf8b0b",
    ),
    (
        "tosaveitemscript",
        "public bool IsBuriedRay()",
        "f09fbc891f437bb6581bd15793d7fa6c8360e4cbb1edb2298ad64c35c87589bf",
    ),
    (
        "tosaveitemscript",
        "public void PreDestroy()",
        "e80c8ef220d699926fcbc7ed80d8f7595d0f349c7ec26cf9d76fafc89aadf77b",
    ),
    (
        "tosaveitemscript",
        "public void RandomSpawned(newRandomStuffSpawnScript.item _item)",
        "3aa3df017a3d8c7b732675bbacbf59d8e0ca972c6ff02d0d9a1b780aa6308db3",
    ),
    (
        "tosaveitemscript",
        "public void RandomSpawned()",
        "d50cff85375857709762223bd04a9d9f96ef3d30351a2be5aa28aff0032b561a",
    ),
    (
        "pickupable",
        "public bool CanPickup()",
        "a1f59e9f928f6cb0ccb9a6b333d6537f05e06f5700e7afcb20b2f45c2d566b18",
    ),
    (
        "meleeweaponscript",
        "private void Update()",
        "90a3ff38e366c3ff0be201fd0d483fc20efb339ecf766416031a4d0a3ae53d6c",
    ),
    (
        "weaponhandlingscript",
        "private void Update()",
        "f27677d91a5be38c8410aa4c22d05c4b61e535e44d98cf0f036f617d2f76ccfd",
    ),
    (
        "weaponhandlingscript",
        "public void AttackStart()",
        "ca1e526bb6069b2f3c63d61b7edb9b90f6c7034e4c8a0c5d74c1f919eb5a7070",
    ),
    (
        "weaponhandlingscript",
        "public void AttackEnd()",
        "24243fd2a50609203de6a37894c9245831eb43212b8b1881b63e521fe3ec233f",
    ),
    (
        "weaponhandlingscript",
        "private void StartAttack(meleeweaponscript.attackType tipus)",
        "4994135333ba9ed241262820c5908192051e9bbf002111feb814c4868f300a71",
    ),
    (
        "animationEventScript",
        "public override void OnStateEnter(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)",
        "ac3b3c585431f5212ba04dbf9ceeaacdbc91e679b3391f1cccfcf9d84ce7da10",
    ),
    (
        "animationEventScript",
        "public override void OnStateExit(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)",
        "9a76b8df3823f85846a6b0ec7546539a02ca86fdf22d4868a97ef0ac29af57a9",
    ),
    (
        "fpscontroller",
        "public void usePickedUpStuff(pickupable pP, bool hand)",
        "a06a516862a5adfa6ede969a7f40cab3f35969a280d1526ae22b0165de6880f3",
    ),
    (
        "TerrainGenerationSettings",
        "public void FStart()",
        "024d1ccfbeb54e9a5a1cbfb12ab901e1b8a7462b8b71206eb4dd5afb3a997eb6",
    ),
    (
        "TerrainGenerationSettings",
        "public void DestroyOthers(GameObject selectedCar)",
        "525cbab037ff441ee3d2ad267004f8e813a7d83990a71deb078f500a78a6cec6",
    ),
    (
        "TerrainGenerationSettings",
        "public void CheckCar(GameObject selectedCar, GameObject other)",
        "49c4dbd4c4ae06b00af5bc812f7af4dde8db8b985146c25f44eaa2931feaf78d",
    ),
    (
        "mainscript",
        "public static bool CheckHoliday(holiday _h)",
        "a0d401d36203d5775fc3a69a290fa922759863f7bb0c7456b0720272d27b2f85",
    ),
    (
        "settingsscript",
        "private void RefreshInMain(bool _changeRes)",
        "79c5c2ef69812b60f3426138d2f33f45e52cd8f72b01f8d0fd3cb50423e01f18",
    ),
];

record!(AcquisitionRules {integer_upper_exclusive:bool,float_upper_inclusive:bool,car_type_to_item_index:Vec<u32>,flows:Vec<AcquisitionFlow>});
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionFlow {
    pub owner: String,
    pub signature: String,
    pub steps: Vec<AcquisitionOperation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum IntegerDrawBound {
    Constant(i32),
    Source(String),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FloatDrawBound {
    Constant(f32),
    Source(String),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionOperation {
    Rule {
        action: String,
    },
    IntegerRange {
        lower: IntegerDrawBound,
        upper: IntegerDrawBound,
        guard: String,
        r#use: String,
    },
    FloatRange {
        lower: FloatDrawBound,
        upper: FloatDrawBound,
        guard: String,
        r#use: String,
    },
    UnitSphere {
        guard: String,
        r#use: String,
    },
}
impl AcquisitionRules {
    fn validate(&self, e: &AcquisitionEvidence) -> Result<(), ReferenceError> {
        require(
            self.integer_upper_exclusive
                && self.float_upper_inclusive
                && self.car_type_to_item_index
                    == [
                        1, 1, 15, 98, 29, 94, 105, 122, 140, 153, 173, 178, 222, 243, 257, 275,
                        309, 0, 337, 356, 371, 390, 396,
                    ],
            "acquisition range or vehicle mapping",
        )?;
        require(
            self.flows == checked_flows(),
            "acquisition checked operation type, bounds, order or comparison",
        )?;
        require(
            self.flows.iter().all(|f| {
                e.methods
                    .iter()
                    .any(|m| m.owner == f.owner && m.signature == f.signature)
            }),
            "acquisition operation method link",
        )
    }
}
fn checked_flows() -> Vec<AcquisitionFlow> {
    vec![
AcquisitionFlow{owner:"settingsscript".into(),signature:"private void RefreshInMain(bool _changeRes)".into(),steps:vec![
AcquisitionOperation::Rule{action:"clamped_inverse_lerp_draw_distance_then_lerp_item_distance".into()},
]},
AcquisitionFlow{owner:"buildingscript".into(),signature:"public void CheckDist()".into(),steps:vec![
AcquisitionOperation::Rule{action:"closest_important_object_distance_strictly_less_than_item_distance".into()},
AcquisitionOperation::Rule{action:"set_items_spawned_before_spawn_stuff".into()},
]},
AcquisitionFlow{owner:"buildingscript".into(),signature:"public void SpawnStuff(int pId)".into(),steps:vec![
AcquisitionOperation::Rule{action:"collect_ordinary_in_hierarchy_order_including_inactive".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("minRandomSpawn".into()),upper:FloatDrawBound::Source("maxRandomSpawn".into()),guard:"either_bound_nonzero".into(),r#use:"replace_chance_only_building_affects_this".into()},
AcquisitionOperation::Rule{action:"spawn_all_underground_before_starting_spawn_coroutine".into()},
]},
AcquisitionFlow{owner:"buildingscript".into(),signature:"private IEnumerator SPAWN()".into(),steps:vec![
AcquisitionOperation::Rule{action:"activate_then_fstart_to_start_in_order".into()},
AcquisitionOperation::Rule{action:"painting_fstart_boundary".into()},
AcquisitionOperation::Rule{action:"call_random_flyer_spawn_before_ordinary".into()},
AcquisitionOperation::Rule{action:"ordinary_hierarchy_order_skip_dont_spawn_yield_after_each_call".into()},
]},
AcquisitionFlow{owner:"buildingscript".into(),signature:"public static void RandomFlyerSpawn(bool _forceRandomFlyer, bool _randomFlyerOnHalloween, Vector3 _pos, float _offsetty, float _vertical, float _horizontalMin, float _horizontalMax)".into(),steps:vec![
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Constant(1.0),guard:"not_force_and_halloween_flag_and_holiday".into(),r#use:"spawn_if_greater_than_point35".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(5),upper:IntegerDrawBound::Constant(30),guard:"force_or_successful_holiday_draw".into(),r#use:"flyer_count".into()},
AcquisitionOperation::UnitSphere{guard:"each_flyer".into(),r#use:"scale_y_by_vertical".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("_horizontalMin".into()),upper:FloatDrawBound::Source("_horizontalMax".into()),guard:"each_flyer_after_sphere".into(),r#use:"scale_x_z_then_add_position_and_vertical_offset".into()},
AcquisitionOperation::Rule{action:"unsupported_flyer_instantiate_then_fstart_boundary".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public void Spawn()".into(),steps:vec![
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.001),upper:FloatDrawBound::Constant(1.0),guard:"holiday_none_or_active_and_server_or_no_multi".into(),r#use:"reject_greater_than_chance".into()},
AcquisitionOperation::Rule{action:"init_spawn_only_if_not_spawned_then_select_spawn_then_spawn_item".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public void InitSpawn()".into(),steps:vec![
AcquisitionOperation::Rule{action:"zero_item_and_place_weights_become_one_before_cumulative_sum".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public void SelectSpawn()".into(),steps:vec![
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("ci + 1".into()),guard:"always".into(),r#use:"first_less_or_equal_item_threshold_excluding_peaceful_humanmonster".into()},
AcquisitionOperation::Rule{action:"default_selected_name_error_position_zero_rotation_spawner".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("items[selected].cp + 1".into()),guard:"selected_item".into(),r#use:"first_less_or_equal_place_threshold".into()},
AcquisitionOperation::Rule{action:"null_place_logs_and_keeps_default_pose_without_reroll".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public void SpawnItem()".into(),steps:vec![
AcquisitionOperation::Rule{action:"direct_gameobject_precedes_item_name_otherwise_category_name".into()},
AcquisitionOperation::Rule{action:"selector_exception_logs_prefab_error_and_returns_before_instantiate".into()},
AcquisitionOperation::Rule{action:"null_prefab_instantiate_error_is_outside_selector_catch".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Constant(360.0),guard:"selected_random_rotation_after_instantiate".into(),r#use:"rotate_world_y".into()},
AcquisitionOperation::Rule{action:"root_spawner_spawn_else_child_spawners_in_order".into()},
AcquisitionOperation::Rule{action:"child_unparent_replaces_instance_then_random_spawned_item_unless_disabled".into()},
AcquisitionOperation::Rule{action:"set_body_kinematic_then_fstart_then_set_spawned".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public static int SelectItem(string _categoryName, bool _isOneItem)".into(),steps:vec![
AcquisitionOperation::Rule{action:"item_name_parse_or_category_select_without_catch".into()},
]},
AcquisitionFlow{owner:"newRandomStuffSpawnScript".into(),signature:"public static int selectId(int categoryId)".into(),steps:vec![
AcquisitionOperation::Rule{action:"unchecked_category_lookup_errors_are_native_terminals".into()},
AcquisitionOperation::Rule{action:"null_itemchances_and_categorychances_use_unit_weights".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(1),upper:IntegerDrawBound::Source("item_total + category_total + 1".into()),guard:"always".into(),r#use:"less_or_equal_item_total_selects_item_branch".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Constant(100),guard:"selected_branch_depend_on_car".into(),r#use:"less_than_25_and_mapped_car_index_in_bounds_returns_direct_child_id".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("selected_branch_length".into()),guard:"depend_on_car_without_direct_return".into(),r#use:"repeat_while_mapped_car_index_at_most_ten_draws_then_return_direct_child_id".into()},
AcquisitionOperation::Rule{action:"independent_items_first_less_or_equal_cumulative_threshold_return_item".into()},
AcquisitionOperation::Rule{action:"independent_categories_first_less_or_equal_threshold_recurse_without_subtracting_item_total".into()},
AcquisitionOperation::Rule{action:"no_matching_threshold_returns_item_zero".into()},
]},
AcquisitionFlow{owner:"undergroundSpawnScript".into(),signature:"public void Spawn()".into(),steps:vec![
AcquisitionOperation::Rule{action:"initialize_cumulative_float_weights_once".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Constant(1.0),guard:"always".into(),r#use:"reject_greater_or_equal_chance".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Source("minCount".into()),upper:IntegerDrawBound::Source("maxCount + 1".into()),guard:"successful_chance".into(),r#use:"attempt_count".into()},
AcquisitionOperation::Rule{action:"each_attempt_random_point_before_table_selection".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Source("maxChance".into()),guard:"each_attempt".into(),r#use:"first_strictly_less_table_threshold".into()},
AcquisitionOperation::Rule{action:"select_item_exception_propagates_return_item_null_is_empty_attempt".into()},
AcquisitionOperation::UnitSphere{guard:"nonnull_prefab".into(),r#use:"euler_rotation_times_360".into()},
AcquisitionOperation::Rule{action:"instantiate_then_fstart_then_random_spawned_default_then_bury".into()},
]},
AcquisitionFlow{owner:"undergroundSpawnScript".into(),signature:"public Vector3 RandomPoint()".into(),steps:vec![
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Constant(360.0),guard:"always".into(),r#use:"angle_degrees".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("minRadius".into()),upper:FloatDrawBound::Source("maxRadius".into()),guard:"after_angle".into(),r#use:"radius".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("minOffsetty".into()),upper:FloatDrawBound::Source("maxOffsetty".into()),guard:"after_radius".into(),r#use:"vertical_preserve_reversed_bounds".into()},
AcquisitionOperation::Rule{action:"center_plus_vertical_up_plus_yaw_forward_times_radius".into()},
]},
AcquisitionFlow{owner:"spawnInBoxScript".into(),signature:"public void FStart()".into(),steps:vec![
AcquisitionOperation::Rule{action:"disabled_returns_before_table_initialization".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Source("maxChance".into()),guard:"enabled_after_cumulative_weights".into(),r#use:"first_strictly_less_table_threshold".into()},
AcquisitionOperation::Rule{action:"select_item_exception_propagates_no_threshold_or_null_item_keeps_empty".into()},
AcquisitionOperation::Rule{action:"spawn_before_open_calls_spawn".into()},
]},
AcquisitionFlow{owner:"spawnInBoxScript".into(),signature:"public void Spawn()".into(),steps:vec![
AcquisitionOperation::Rule{action:"disabled_or_spawned_or_null_prefab_returns".into()},
AcquisitionOperation::Rule{action:"null_spawn_position_uses_box_transform".into()},
AcquisitionOperation::Rule{action:"current_count_one_and_spawned_count_zero".into()},
AcquisitionOperation::UnitSphere{guard:"spawned_count_greater_than_zero_before_instantiate".into(),r#use:"offset_spawn_scale_x_times_point15".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(1),upper:IntegerDrawBound::Source("maxCountIfSpawnedInBox + 1".into()),guard:"first_instance_mass_count_positive".into(),r#use:"current_count".into()},
AcquisitionOperation::Rule{action:"cant_pickup_while_mount_then_fstart_then_random_spawned_default".into()},
AcquisitionOperation::UnitSphere{guard:"each_iteration_after_initialization".into(),r#use:"discarded_position_assignment".into()},
AcquisitionOperation::Rule{action:"increment_count_repeat_while_less_than_current_count".into()},
AcquisitionOperation::Rule{action:"mount_fstart_quickfreeze_prescan_disable_then_set_spawned".into()},
]},
AcquisitionFlow{owner:"spawnInBoxScript".into(),signature:"public void Opened()".into(),steps:vec![
AcquisitionOperation::Rule{action:"first_open_calls_spawn_then_sets_opened".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void RandomSpawned(newRandomStuffSpawnScript.item _item)".into(),steps:vec![
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Source("(int)_item.fr1".into()),upper:IntegerDrawBound::Source("(int)_item.fr2".into()),guard:"ammo_exists_and_not_spray".into(),r#use:"set_integer_ammo".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("ammo.ammoRmin".into()),upper:FloatDrawBound::Source("ammo.ammoRmax".into()),guard:"ammo_above_max_or_equal_item_float_bounds".into(),r#use:"set_float_ammo_or_truncate_to_integer".into()},
AcquisitionOperation::Rule{action:"partcondition_random_start_item_integer_bounds_or_default_zero_to_four".into()},
AcquisitionOperation::Rule{action:"apply_fluid_overrides_then_randomfluid_tank_other_tanks_randomize_tanks_boundaries".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.0),upper:FloatDrawBound::Constant(1.0),guard:"food_exists".into(),r#use:"food_hp".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("food.parts.Length - 2".into()),guard:"food_exists_after_hp".into(),r#use:"food_index_clamped_at_zero".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void RandomSpawned()".into(),steps:vec![
AcquisitionOperation::Rule{action:"nonspray_ammo_set_to_max_without_ammo_draw".into()},
AcquisitionOperation::Rule{action:"partcondition_random_start_default_zero_to_one".into()},
AcquisitionOperation::Rule{action:"engine_coolant_empty_chance_one_other_zero_mix_zero_then_fluid_boundaries".into()},
]},
AcquisitionFlow{owner:"breakablescript".into(),signature:"public void TryBreak(float force)".into(),steps:vec![
AcquisitionOperation::Rule{action:"force_greater_or_equal_health_break_else_decrease_health_if_enabled".into()},
]},
AcquisitionFlow{owner:"animationEventScript".into(),signature:"public override void OnStateEnter(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)".into(),steps:vec![
AcquisitionOperation::Rule{action:"state_enter_calls_weapon_attack_start".into()},
]},
AcquisitionFlow{owner:"animationEventScript".into(),signature:"public override void OnStateExit(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)".into(),steps:vec![
AcquisitionOperation::Rule{action:"state_exit_calls_weapon_attack_end".into()},
]},
AcquisitionFlow{owner:"breakablescript".into(),signature:"public void Break()".into(),steps:vec![
AcquisitionOperation::Rule{action:"pickup_drop2_before_mark_destroyed".into()},
AcquisitionOperation::Rule{action:"mark_destroyed_select_reduced_or_full_gib_native_eligibility".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("clips.Length".into()),guard:"clips_nonempty".into(),r#use:"break_clip".into()},
AcquisitionOperation::Rule{action:"set_remove_from_memory_predestroy_then_destroy_when_destroy_allowed".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void PreDestroy()".into(),steps:vec![
AcquisitionOperation::Rule{action:"remove_from_memory_calls_each_physlock_unstore".into()},
]},
AcquisitionFlow{owner:"mountStuff".into(),signature:"public void Unstore()".into(),steps:vec![
AcquisitionOperation::Rule{action:"unphysicslock_then_rebury_if_parent_buried".into()},
]},
AcquisitionFlow{owner:"weaponhandlingscript".into(),signature:"private void Update()".into(),steps:vec![
AcquisitionOperation::Rule{action:"animator_speed_and_attack_trigger_follow_source_input_and_state".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("melee.attackClips.Length".into()),guard:"attacking_and_not_played_and_elapsed_greater_than_point1_divided_by_speed".into(),r#use:"attack_clip".into()},
]},
AcquisitionFlow{owner:"meleeweaponscript".into(),signature:"private void Update()".into(),steps:vec![
AcquisitionOperation::Rule{action:"hand_pose_arrays_guard_attack_index_position_lerp_delta16_rotation_lerp_delta8".into()},
AcquisitionOperation::Rule{action:"overlap_box_trigger_pose_scale_mask_query_collide_excludes_same_root_and_singleplayer_layer_nine".into()},
AcquisitionOperation::Rule{action:"contact_force_from_target_delta_times_half_delta_time_rigidbody_add_unless_recent".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Constant(0.9),upper:FloatDrawBound::Constant(1.1),guard:"accepted_contact".into(),r#use:"spark_force_upper_multiplier".into()},
AcquisitionOperation::FloatRange{lower:FloatDrawBound::Source("allparticle * FcurrentForce".into()),upper:FloatDrawBound::Source("allparticle * FcurrentForce * spark_multiplier".into()),guard:"accepted_contact_after_multiplier".into(),r#use:"spark_count_truncated_integer".into()},
AcquisitionOperation::Rule{action:"particle_and_hit_type_layer17_sand_layer23_glass_then_flesh_plant_wood_else_metal".into()},
AcquisitionOperation::Rule{action:"nonzero_shovel_speed_updates_buried_or_recent_objects_else_native_damage_boundaries".into()},
AcquisitionOperation::IntegerRange{lower:IntegerDrawBound::Constant(0),upper:IntegerDrawBound::Source("meleeHitSounds[hitType].a.Length".into()),guard:"has_to_sound_not_sounded_after_next_time_and_point125_delay".into(),r#use:"contact_clip".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void Bury()".into(),steps:vec![
AcquisitionOperation::Rule{action:"body_kinematic_buried_true_update_outline".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void UnBury()".into(),steps:vec![
AcquisitionOperation::Rule{action:"record_time_only_if_buried_then_body_dynamic_buried_false_outline".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public bool IsRecentlyUnburied()".into(),steps:vec![
AcquisitionOperation::Rule{action:"time_strictly_less_than_unburied_time_plus_one".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public void UpdBuried(float _shovelSpeed)".into(),steps:vec![
AcquisitionOperation::Rule{action:"once_per_delta_time_move_up_delta_times_speed_then_is_buried_ray".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public bool IsBuried()".into(),steps:vec![
AcquisitionOperation::Rule{action:"buried_body_dynamic_unbury_else_is_buried_ray".into()},
]},
AcquisitionFlow{owner:"tosaveitemscript".into(),signature:"public bool IsBuriedRay()".into(),steps:vec![
AcquisitionOperation::Rule{action:"downward_raycast_all_sorted_distance_lowest_enabled_nontrigger_nonwheel_nonlayer9_collider".into()},
AcquisitionOperation::Rule{action:"exclude_self_and_player_hits_unbury_if_below_lowest_and_hit_not_buried".into()},
]},
AcquisitionFlow{owner:"pickupable".into(),signature:"public bool CanPickup()".into(),steps:vec![
AcquisitionOperation::Rule{action:"not_cant_pickup_and_no_save_or_not_is_buried".into()},
]},
AcquisitionFlow{owner:"fpscontroller".into(),signature:"public void usePickedUpStuff(pickupable pP, bool hand)".into(),steps:vec![
AcquisitionOperation::Rule{action:"cursor_locked_use_down_positive_pack_ammo".into()},
AcquisitionOperation::Rule{action:"decrement_integer_count_then_instantiate_cigarette_at_pack_pose_then_fstart".into()},
AcquisitionOperation::Rule{action:"empty_wearable_slot_wears_else_retains_world_cigarette".into()},
]},
AcquisitionFlow{owner:"TerrainGenerationSettings".into(),signature:"public void FStart()".into(),steps:vec![
AcquisitionOperation::Rule{action:"new_game_not_loaded_not_multiplayer_local_calendar_holiday_activation".into()},
AcquisitionOperation::Rule{action:"selected_car_group_check_outside_holiday_new_game_block".into()},
]},
AcquisitionFlow{owner:"TerrainGenerationSettings".into(),signature:"public void CheckCar(GameObject selectedCar, GameObject other)".into(),steps:vec![
AcquisitionOperation::Rule{action:"activate_selected_start_group_destroy_unselected_start_groups".into()},
]},
AcquisitionFlow{owner:"mainscript".into(),signature:"public static bool CheckHoliday(holiday _h)".into(),steps:vec![
AcquisitionOperation::Rule{action:"local_calendar_inclusive_start_and_end_same_year".into()},
]},
]
}

fn prefab_identity(id: i64) -> Option<(&'static str, Option<u32>)> {
    match id {
        32825 => Some(("", Some(74))),
        39986 => Some(("", Some(54))),
        6496 => Some(("", Some(71))),
        42570 => Some(("", Some(60))),
        12981 => Some(("", Some(86))),
        30814 => Some(("", Some(72))),
        35111 => Some(("", Some(76))),
        39989 => Some(("", Some(57))),
        20979 => Some(("", Some(75))),
        20649 => Some(("", Some(92))),
        30548 => Some(("", Some(254))),
        16468 => Some(("", Some(80))),
        28749 => Some(("", Some(66))),
        26645 => Some(("", Some(85))),
        32753 => Some(("", Some(83))),
        41097 => Some(("", None)),
        9085 => Some(("", Some(62))),
        9086 => Some(("", Some(231))),
        24105 => Some(("", Some(87))),
        26970 => Some(("", Some(88))),
        35853 => Some(("", Some(70))),
        36799 => Some(("", Some(81))),
        35272 => Some(("", Some(82))),
        7370 => Some(("", Some(84))),
        _ => None,
    }
}

struct SourceSpawnerBinding {
    source: i64,
    marker: i64,
    chance: f32,
    dont_spawn: bool,
    holiday: u32,
    building_affects: bool,
    entries: &'static [SourceEntryBinding],
}
struct SourceEntryBinding {
    kind: u8,
    identity: i64,
    weight: u32,
    random_rotation: bool,
    disable_random: bool,
    places: &'static [i64],
}
fn spawner_binding(o: &OrdinaryAcquisitionReference) -> bool {
    let Some(b) = SPAWNER_BINDINGS
        .iter()
        .find(|b| b.source == o.source.path_id)
    else {
        return false;
    };
    o.source.file == "sharedassets1.assets"
        && o.marker.source.path_id == b.marker
        && o.chance == b.chance
        && o.dont_spawn == b.dont_spawn
        && o.holiday == b.holiday
        && o.building_affects_this == b.building_affects
        && o.entries.len() == b.entries.len()
        && o.entries.iter().zip(b.entries).all(|(e, b)| {
            let identity = match &e.selection {
                AcquisitionSelection::Category { category_id, .. } => (0, i64::from(*category_id)),
                AcquisitionSelection::Item { item_id, .. } => (1, i64::from(*item_id)),
                AcquisitionSelection::Prefab { source, .. } => (2, source.path_id),
            };
            identity == (b.kind, b.identity)
                && e.weight == b.weight
                && e.random_rotation == b.random_rotation
                && e.disable_random == b.disable_random
                && e.places
                    .iter()
                    .map(|p| p.marker.as_ref().map_or(0, |m| m.source.path_id))
                    .eq(b.places.iter().copied())
        })
}
const SPAWNER_BINDINGS: &[SourceSpawnerBinding] = &[
    SourceSpawnerBinding {
        source: 155092,
        marker: 61833,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 32825,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[66773, 80443, 83523, 78385],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 39986,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[57440, 57399, 72309, 64524, 88948, 71778, 73569, 84839],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 131522,
        marker: 63125,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[66184, 69510, 73635, 69235, 54162, 75437],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[81152, 70778, 73763],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 139976,
        marker: 85141,
        chance: 0.75,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[68475],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[77624],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[60482, 87027],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 155127,
        marker: 80181,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[66422],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[
                    62555, 71332, 72239, 69644, 58028, 74037, 74000, 84886, 54509, 67667,
                ],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 30814,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[83276, 87363],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[54908, 82700, 68049, 51728, 82735],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 145959,
        marker: 54817,
        chance: 1.0,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 30814,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[63463, 52233, 82185, 53903, 71141, 83493, 58654],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[
                    87297, 53076, 66856, 72514, 61254, 50045, 57366, 50927, 58748,
                ],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 156024,
        marker: 86805,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 35111,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[
                76282, 64900, 57465, 70396, 64541, 69026, 64603, 51523, 61320, 88055, 58502, 52348,
                51354,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 152647,
        marker: 59279,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 27,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[
                58040, 60169, 69651, 68837, 57085, 85581, 77506, 68183, 74329, 71577,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 152199,
        marker: 54663,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                60103, 70575, 50946, 54560, 83810, 69772, 61064, 74403, 54671,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 144347,
        marker: 70441,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[61764, 77450, 87478, 84564, 77806, 58255, 57464, 55875],
        }],
    },
    SourceSpawnerBinding {
        source: 156794,
        marker: 79775,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                85514, 88018, 50653, 51873, 80147, 50869, 88192, 75857, 68528,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 152085,
        marker: 84519,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 0,
                identity: 36,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[
                    63707, 88971, 71243, 60068, 63841, 59733, 78034, 69859, 66250, 66887, 58098,
                    53489, 66456, 72562, 71391,
                ],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 39989,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[84511, 82387, 81186],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 20979,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[
                    69100, 68356, 80028, 72149, 84398, 57774, 54247, 58940, 61339, 65564, 63376,
                    66886, 71218,
                ],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[
                    74073, 80843, 81473, 85080, 55877, 63961, 51266, 58706, 81793, 69862, 63899,
                    61284,
                ],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 152852,
        marker: 64283,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143968,
        marker: 59834,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 39,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                58045, 75373, 85937, 63613, 60660, 78868, 68007, 84953, 62723, 67777, 58479, 52707,
                81992, 76363, 86350, 71849, 57313, 85667,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 152401,
        marker: 50102,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136189,
        marker: 77086,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159015,
        marker: 53593,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132525,
        marker: 55063,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157727,
        marker: 64387,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144982,
        marker: 85175,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148080,
        marker: 51257,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154174,
        marker: 58428,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148737,
        marker: 68583,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137140,
        marker: 87476,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158935,
        marker: 55769,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154247,
        marker: 85614,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141801,
        marker: 84613,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157379,
        marker: 83677,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159136,
        marker: 73371,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 145318,
        marker: 89167,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143532,
        marker: 83521,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144808,
        marker: 80501,
        chance: 0.699999988079071,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 59,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[57978, 54795, 77992, 54840, 74013],
        }],
    },
    SourceSpawnerBinding {
        source: 142945,
        marker: 72617,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 1,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 20649,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[70928, 72981, 80546, 51167, 57744, 82135, 60596, 89351],
        }],
    },
    SourceSpawnerBinding {
        source: 135698,
        marker: 83565,
        chance: 0.25,
        dont_spawn: false,
        holiday: 2,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 30548,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[78977, 54691, 82950, 52665, 78542, 84177],
        }],
    },
    SourceSpawnerBinding {
        source: 153352,
        marker: 87594,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 0,
                identity: 47,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[79501, 50082, 68845, 62553],
            },
            SourceEntryBinding {
                kind: 0,
                identity: 47,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[86761],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 135774,
        marker: 86243,
        chance: 1.0,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 30814,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[53406, 84535, 85797, 78786, 73479, 84848, 50655, 89503],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[89067, 69936, 51177],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 150475,
        marker: 76174,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 36,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[54767, 50018, 54379, 83422, 86274],
        }],
    },
    SourceSpawnerBinding {
        source: 146976,
        marker: 85611,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 36,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[78136, 84244, 67066, 67640, 65542],
        }],
    },
    SourceSpawnerBinding {
        source: 144588,
        marker: 63527,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 36,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[73955, 64381, 76682, 63039, 69784],
        }],
    },
    SourceSpawnerBinding {
        source: 143922,
        marker: 75167,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 16468,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[85455],
            },
            SourceEntryBinding {
                kind: 0,
                identity: 0,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[49364],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 147378,
        marker: 50406,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[57159, 69564, 66752, 89146],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[64070, 71881],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[86246, 60089, 49579],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 28749,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[49546],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 26645,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[79846, 61353, 51330],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 145695,
        marker: 64272,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[71438, 62139],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[72879, 71130, 81310],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[77123, 86312, 50021],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 28749,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[79913, 88705, 50369],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 26645,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[67702, 61408],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 143142,
        marker: 87051,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[61471, 61434, 59329],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 6496,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[54675, 59263],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[76229, 51412],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 28749,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[55248, 53517, 62644],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 26645,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[78096, 81613, 84842, 84918],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 140022,
        marker: 77163,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[77505, 70083, 89225, 76044, 55404, 89066, 54702],
        }],
    },
    SourceSpawnerBinding {
        source: 142385,
        marker: 73198,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[57410, 82089, 83215, 65474, 60362],
        }],
    },
    SourceSpawnerBinding {
        source: 130795,
        marker: 82164,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[53199, 74827, 88452, 56923, 56695, 84910],
        }],
    },
    SourceSpawnerBinding {
        source: 131645,
        marker: 81967,
        chance: 0.550000011920929,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 5,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[61517, 69519],
        }],
    },
    SourceSpawnerBinding {
        source: 134362,
        marker: 71600,
        chance: 0.550000011920929,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 5,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[50345, 83297, 60086],
        }],
    },
    SourceSpawnerBinding {
        source: 149222,
        marker: 58231,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 0,
                identity: 5,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[66980],
            },
            SourceEntryBinding {
                kind: 0,
                identity: 5,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[69024],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 143195,
        marker: 74787,
        chance: 0.44999998807907104,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 11,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[66940, 83675, 65523, 66386],
        }],
    },
    SourceSpawnerBinding {
        source: 158837,
        marker: 86638,
        chance: 0.44999998807907104,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 11,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[62350, 52299, 51444, 56105],
        }],
    },
    SourceSpawnerBinding {
        source: 148932,
        marker: 74453,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 0,
                identity: 8,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[63626, 70921],
            },
            SourceEntryBinding {
                kind: 0,
                identity: 8,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[71365, 67867],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 150294,
        marker: 77507,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 0,
                identity: 8,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[85197],
            },
            SourceEntryBinding {
                kind: 0,
                identity: 8,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[52647, 70360],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 131020,
        marker: 81477,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 32753,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[57497, 84335, 54115, 72266],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 41097,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[63060, 67596],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 153823,
        marker: 85490,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[73076, 63497, 51213, 56570, 69014],
        }],
    },
    SourceSpawnerBinding {
        source: 137208,
        marker: 78996,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[80483, 52915, 51137, 80223],
        }],
    },
    SourceSpawnerBinding {
        source: 153740,
        marker: 78124,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[67378, 89272, 86749, 62738, 67265],
        }],
    },
    SourceSpawnerBinding {
        source: 153176,
        marker: 74025,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[67068, 68591, 80314, 70137],
        }],
    },
    SourceSpawnerBinding {
        source: 139372,
        marker: 83536,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 26,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[89355, 63449, 59523],
        }],
    },
    SourceSpawnerBinding {
        source: 150978,
        marker: 56968,
        chance: 0.550000011920929,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[62832],
        }],
    },
    SourceSpawnerBinding {
        source: 134469,
        marker: 57449,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 38,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144495,
        marker: 54265,
        chance: 0.550000011920929,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[71660],
        }],
    },
    SourceSpawnerBinding {
        source: 155447,
        marker: 69352,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[81373, 85375, 84159, 80102, 51725],
        }],
    },
    SourceSpawnerBinding {
        source: 159356,
        marker: 66992,
        chance: 0.6499999761581421,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[81984, 88450, 85839, 72349],
        }],
    },
    SourceSpawnerBinding {
        source: 153506,
        marker: 68009,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 51,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[49830, 77093, 65501, 72970, 73307],
        }],
    },
    SourceSpawnerBinding {
        source: 149796,
        marker: 53036,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 51,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[52114, 80689, 50403, 88356],
        }],
    },
    SourceSpawnerBinding {
        source: 152316,
        marker: 73589,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 59,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[75976, 89165, 80932, 53438, 59437],
        }],
    },
    SourceSpawnerBinding {
        source: 140091,
        marker: 82072,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 1,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 20649,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[56983, 56454, 69360, 73581, 59419, 73868, 54017, 74058],
        }],
    },
    SourceSpawnerBinding {
        source: 140159,
        marker: 74546,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 82,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                61667, 62561, 77415, 77029, 53558, 85076, 69422, 58690, 71311, 74749,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 158497,
        marker: 84486,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9086,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[64910, 55889, 80020, 87059, 49584, 50478, 67305],
        }],
    },
    SourceSpawnerBinding {
        source: 135875,
        marker: 50124,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 77,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[56154, 72292, 52606, 52877, 68776],
        }],
    },
    SourceSpawnerBinding {
        source: 133894,
        marker: 54322,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132473,
        marker: 66636,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134881,
        marker: 81660,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152604,
        marker: 78442,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143849,
        marker: 61905,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141037,
        marker: 66593,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 24105,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134594,
        marker: 89219,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155658,
        marker: 73767,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155735,
        marker: 63600,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 131807,
        marker: 67445,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148462,
        marker: 77129,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133453,
        marker: 52168,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151325,
        marker: 81536,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159388,
        marker: 84402,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151878,
        marker: 63955,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156241,
        marker: 74462,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 150554,
        marker: 68699,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152271,
        marker: 72974,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134832,
        marker: 86363,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152756,
        marker: 66208,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149070,
        marker: 80718,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143444,
        marker: 49808,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144153,
        marker: 71420,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134567,
        marker: 67406,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132539,
        marker: 54515,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 26970,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 131865,
        marker: 61222,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148411,
        marker: 67920,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133089,
        marker: 81119,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153078,
        marker: 76079,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135175,
        marker: 87140,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153070,
        marker: 87565,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151379,
        marker: 79333,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138654,
        marker: 85557,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156146,
        marker: 84433,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138632,
        marker: 81997,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9085,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134371,
        marker: 58782,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 26,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[83903, 64276, 67216, 74355],
        }],
    },
    SourceSpawnerBinding {
        source: 131777,
        marker: 60002,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 37,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[65951, 79034, 61247, 89086],
        }],
    },
    SourceSpawnerBinding {
        source: 142598,
        marker: 82343,
        chance: 1.0,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 42570,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[
                67221, 82639, 58910, 87700, 58956, 53678, 76031, 76465, 66594, 77550, 79880, 72859,
                55631, 75527, 72302, 81842, 53416, 50759, 64828, 67432, 52393, 69132, 86043,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 147640,
        marker: 69084,
        chance: 1.0,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 30814,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[
                    51203, 65817, 86286, 66377, 85186, 53637, 63865, 68787, 83091, 51618, 69713,
                    52766, 66348, 50494, 67885, 68479, 64550, 75086, 76360, 63464, 82913, 55077,
                    68253,
                ],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 42570,
                weight: 0,
                random_rotation: false,
                disable_random: false,
                places: &[87879],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 136433,
        marker: 56935,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[81074, 69791, 88538, 81296, 73424],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 35853,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[58909, 71737],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 152320,
        marker: 54139,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[56756, 80691, 65615, 70112],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 35853,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[51583, 71299],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 152354,
        marker: 55828,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[
            SourceEntryBinding {
                kind: 2,
                identity: 12981,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[62026, 62381, 89188, 89481],
            },
            SourceEntryBinding {
                kind: 2,
                identity: 35853,
                weight: 0,
                random_rotation: true,
                disable_random: false,
                places: &[51411, 58177],
            },
        ],
    },
    SourceSpawnerBinding {
        source: 149997,
        marker: 53221,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134150,
        marker: 72486,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141763,
        marker: 58182,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 145394,
        marker: 67070,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 131269,
        marker: 59897,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149343,
        marker: 88483,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154365,
        marker: 62033,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132040,
        marker: 87561,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143201,
        marker: 89148,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142584,
        marker: 49292,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 146500,
        marker: 87326,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140319,
        marker: 84011,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155187,
        marker: 65978,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138169,
        marker: 62711,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138563,
        marker: 73915,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152202,
        marker: 64652,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132607,
        marker: 62456,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133602,
        marker: 71806,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151471,
        marker: 65724,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139581,
        marker: 59297,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 130933,
        marker: 54193,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 146077,
        marker: 72241,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152232,
        marker: 53323,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140250,
        marker: 88098,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136758,
        marker: 87261,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144321,
        marker: 70033,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143910,
        marker: 55371,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158529,
        marker: 71159,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 147544,
        marker: 80646,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157221,
        marker: 78452,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137563,
        marker: 59163,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157472,
        marker: 73537,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140306,
        marker: 84304,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133988,
        marker: 53191,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156824,
        marker: 61814,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 36799,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149279,
        marker: 78742,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 35272,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[57480, 58289, 80493, 71450, 86786],
        }],
    },
    SourceSpawnerBinding {
        source: 154875,
        marker: 84298,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 32753,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[63134, 52009, 63699, 57354, 58762],
        }],
    },
    SourceSpawnerBinding {
        source: 153360,
        marker: 64498,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 16468,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[75352, 65320, 50483, 49502],
        }],
    },
    SourceSpawnerBinding {
        source: 134606,
        marker: 67662,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153100,
        marker: 62024,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151479,
        marker: 87605,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141788,
        marker: 64506,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134009,
        marker: 82476,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134004,
        marker: 81855,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151772,
        marker: 79307,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154990,
        marker: 56942,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139952,
        marker: 84617,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143756,
        marker: 80149,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159395,
        marker: 85111,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152486,
        marker: 58825,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159108,
        marker: 57327,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149708,
        marker: 52730,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143441,
        marker: 52806,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148956,
        marker: 80363,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133810,
        marker: 64808,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137616,
        marker: 56828,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 24,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156686,
        marker: 85844,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151104,
        marker: 80291,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141085,
        marker: 79554,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149001,
        marker: 82025,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156759,
        marker: 86615,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144858,
        marker: 85897,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155522,
        marker: 55043,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151810,
        marker: 52295,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142649,
        marker: 72785,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158222,
        marker: 61674,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135069,
        marker: 83114,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142695,
        marker: 73811,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135361,
        marker: 86220,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149754,
        marker: 72470,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156893,
        marker: 80407,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142419,
        marker: 66696,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152913,
        marker: 60313,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156419,
        marker: 69523,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 130875,
        marker: 81270,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 146654,
        marker: 87252,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154127,
        marker: 62205,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135643,
        marker: 65779,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 152942,
        marker: 59999,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158813,
        marker: 76413,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139621,
        marker: 66403,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136713,
        marker: 66628,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156175,
        marker: 56506,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140582,
        marker: 51642,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138065,
        marker: 82215,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135847,
        marker: 75331,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 7370,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154094,
        marker: 67717,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138584,
        marker: 60599,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 146062,
        marker: 72644,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142303,
        marker: 72677,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132513,
        marker: 56877,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138338,
        marker: 77451,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143718,
        marker: 74275,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135235,
        marker: 62795,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151295,
        marker: 79028,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144008,
        marker: 59157,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153794,
        marker: 63704,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135332,
        marker: 55367,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132925,
        marker: 49781,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136321,
        marker: 53654,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136599,
        marker: 87618,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149893,
        marker: 59159,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133770,
        marker: 59558,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148179,
        marker: 62767,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 134021,
        marker: 53935,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138866,
        marker: 89458,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 150213,
        marker: 80846,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137377,
        marker: 49490,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148616,
        marker: 61818,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148291,
        marker: 86646,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 150601,
        marker: 71153,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136148,
        marker: 74986,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 132645,
        marker: 57348,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154037,
        marker: 84377,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137393,
        marker: 73369,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 141383,
        marker: 51866,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140922,
        marker: 85998,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153038,
        marker: 71696,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149590,
        marker: 63252,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157508,
        marker: 80839,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 133298,
        marker: 66277,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157793,
        marker: 71706,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144300,
        marker: 57956,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140956,
        marker: 75241,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 145078,
        marker: 73883,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 144514,
        marker: 80850,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 6496,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 148891,
        marker: 68271,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 82,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[88080, 67166, 49538],
        }],
    },
    SourceSpawnerBinding {
        source: 151014,
        marker: 80774,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143615,
        marker: 51719,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136463,
        marker: 57701,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154073,
        marker: 64030,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158683,
        marker: 78990,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153893,
        marker: 85822,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137745,
        marker: 68575,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151995,
        marker: 66210,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 135598,
        marker: 85228,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 130805,
        marker: 66831,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155482,
        marker: 60208,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 155305,
        marker: 89466,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143582,
        marker: 75427,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156675,
        marker: 59359,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 131215,
        marker: 55579,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 159227,
        marker: 63014,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140073,
        marker: 75263,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140141,
        marker: 70361,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151054,
        marker: 75361,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149896,
        marker: 59634,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137792,
        marker: 62817,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153493,
        marker: 61946,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140991,
        marker: 62840,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139566,
        marker: 74970,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 131889,
        marker: 62209,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142600,
        marker: 63850,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 142749,
        marker: 85814,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153239,
        marker: 82117,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149396,
        marker: 82199,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 138457,
        marker: 61628,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 7,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 151905,
        marker: 78743,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 2,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[66306, 73235, 81985, 84545],
        }],
    },
    SourceSpawnerBinding {
        source: 148855,
        marker: 57196,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 2,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[80958, 82827, 68790, 50113],
        }],
    },
    SourceSpawnerBinding {
        source: 156815,
        marker: 76090,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 2,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[74595, 59136, 52503, 58457],
        }],
    },
    SourceSpawnerBinding {
        source: 142703,
        marker: 56313,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 42570,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[66486, 68000, 58068],
        }],
    },
    SourceSpawnerBinding {
        source: 131517,
        marker: 80534,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 42570,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[71709, 65399, 74026],
        }],
    },
    SourceSpawnerBinding {
        source: 131910,
        marker: 72270,
        chance: 0.5,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 42570,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[84251, 72395, 64426],
        }],
    },
    SourceSpawnerBinding {
        source: 131490,
        marker: 60168,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 156814,
        marker: 80382,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136246,
        marker: 56260,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153963,
        marker: 63999,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154490,
        marker: 60231,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 158555,
        marker: 49259,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 147609,
        marker: 85701,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139150,
        marker: 76581,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 35,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 154895,
        marker: 58362,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 143647,
        marker: 65683,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139312,
        marker: 57318,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 139214,
        marker: 60059,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 147519,
        marker: 80624,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153824,
        marker: 54614,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 137672,
        marker: 53505,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 153505,
        marker: 63307,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 157398,
        marker: 86369,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 149671,
        marker: 56517,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 136219,
        marker: 57474,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 140191,
        marker: 62507,
        chance: 0.20000000298023224,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 25,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[],
        }],
    },
    SourceSpawnerBinding {
        source: 150410,
        marker: 50812,
        chance: 0.4000000059604645,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 39,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                70259, 50275, 64108, 88127, 58458, 59573, 52551, 84316, 53573, 86297, 86509, 74802,
                67617, 64829, 54458, 63799, 51950, 53501,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 136300,
        marker: 64795,
        chance: 0.3499999940395355,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 59,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[64354, 69649, 51438, 71593, 84227],
        }],
    },
    SourceSpawnerBinding {
        source: 132674,
        marker: 70248,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 1,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 20649,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[58043, 54476, 86222, 51198],
        }],
    },
    SourceSpawnerBinding {
        source: 152360,
        marker: 73946,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 88,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[54377, 55221, 79864, 64471],
        }],
    },
    SourceSpawnerBinding {
        source: 159041,
        marker: 51632,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 88,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[58030, 79596, 82277, 73612],
        }],
    },
    SourceSpawnerBinding {
        source: 154385,
        marker: 85449,
        chance: 0.30000001192092896,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 88,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[60539, 51983, 85570, 51673],
        }],
    },
    SourceSpawnerBinding {
        source: 148893,
        marker: 78308,
        chance: 0.44999998807907104,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 63,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[
                73357, 76518, 57524, 56293, 84845, 80411, 69391, 60384, 71121, 58972,
            ],
        }],
    },
    SourceSpawnerBinding {
        source: 156861,
        marker: 62528,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9086,
            weight: 0,
            random_rotation: false,
            disable_random: false,
            places: &[56368, 65478, 86235, 66706, 88076],
        }],
    },
    SourceSpawnerBinding {
        source: 155859,
        marker: 74139,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 9086,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[55447, 69514, 64927, 69676, 65384],
        }],
    },
    SourceSpawnerBinding {
        source: 145390,
        marker: 81868,
        chance: 0.25,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 2,
            identity: 30548,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[80794, 81724, 79669, 83853, 54738, 65132],
        }],
    },
    SourceSpawnerBinding {
        source: 141891,
        marker: 67272,
        chance: 0.6000000238418579,
        dont_spawn: false,
        holiday: 0,
        building_affects: false,
        entries: &[SourceEntryBinding {
            kind: 0,
            identity: 77,
            weight: 0,
            random_rotation: true,
            disable_random: false,
            places: &[74340, 84753, 63048, 63737, 77374],
        }],
    },
];

fn table_binding(t: &[AcquisitionTableEntry], b: &[(bool, u32, f32)]) -> bool {
    t.len() == b.len()
        && t.iter()
            .zip(b)
            .all(|(r, (item, id, weight))| match r.selection {
                AcquisitionSelection::Category { category_id, .. } => {
                    !*item && category_id == *id && r.weight == *weight
                }
                AcquisitionSelection::Item { item_id, .. } => {
                    *item && item_id == *id && r.weight == *weight
                }
                _ => false,
            })
}
const BOX_TABLE: &[(bool, u32, f32)] = &[
    (false, 37, 0.5),
    (false, 26, 0.5),
    (false, 41, 0.25),
    (false, 24, 0.5),
    (false, 66, 0.5),
    (false, 71, 0.5),
    (false, 68, 0.5),
    (false, 76, 0.5),
    (false, 70, 0.5),
    (false, 69, 0.5),
    (false, 57, 0.5),
    (false, 83, 1.0),
    (false, 84, 1.0),
    (false, 25, 1.0),
    (false, 85, 2.0),
    (false, 30, 1.0),
    (false, 86, 1.0),
];
const UNDERGROUND_TABLES: &[&[(bool, u32, f32)]] = &[
    &[
        (false, 37, 0.5),
        (false, 26, 0.5),
        (false, 41, 0.25),
        (false, 24, 0.5),
        (false, 68, 0.5),
        (false, 76, 0.5),
        (false, 70, 0.5),
        (false, 71, 0.5),
        (false, 66, 0.5),
        (false, 69, 0.5),
        (false, 57, 0.5),
        (false, 83, 1.0),
        (false, 84, 1.0),
        (false, 25, 1.0),
        (false, 85, 2.0),
        (false, 30, 1.0),
        (false, 86, 1.0),
        (true, 62, 1.0),
    ],
    &[
        (false, 37, 0.5),
        (false, 26, 0.5),
        (false, 41, 0.25),
        (false, 24, 0.5),
        (false, 68, 0.5),
        (false, 76, 0.5),
        (false, 70, 0.5),
        (false, 71, 0.5),
        (false, 66, 0.5),
        (false, 69, 0.5),
        (false, 57, 0.5),
        (false, 83, 1.0),
        (false, 84, 1.0),
        (false, 25, 1.0),
        (false, 85, 2.0),
        (false, 30, 1.0),
        (false, 86, 1.0),
        (true, 62, 1.0),
    ],
    &[
        (false, 37, 0.5),
        (false, 26, 0.5),
        (false, 41, 0.25),
        (false, 24, 0.5),
        (false, 68, 0.5),
        (false, 76, 0.5),
        (false, 70, 0.5),
        (false, 71, 0.5),
        (false, 66, 0.5),
        (false, 69, 0.5),
        (false, 57, 0.5),
        (false, 83, 1.0),
        (false, 84, 1.0),
        (false, 25, 1.0),
        (false, 85, 2.0),
        (false, 30, 1.0),
        (false, 86, 1.0),
        (true, 62, 1.0),
    ],
];

fn checked_pose(p: &AcquisitionPose) -> bool {
    match (p.source.file.as_str(), p.source.path_id) {
        ("sharedassets1.assets", 51722) => {
            p.parent.is_none()
                && p.transform.translation == [0.0, 0.0, -0.0]
                && p.transform.rotation == [-0.0, -0.0, 0.0, 1.0]
                && p.transform.scale == [1.0, 1.0, 1.0]
        }
        ("level1", 21449) => {
            p.parent
                .as_ref()
                .is_some_and(|v| source_is(v, "level1", 21481))
                && p.transform.translation
                    == [1.4539967775344849, 6.00600004196167, 14.700002670288086]
                && p.transform.rotation == [-0.0, -1.0, 0.0, 0.0]
                && p.transform.scale == [1.0, 1.0, 1.0]
        }
        ("level1", 33859) => {
            p.parent
                .as_ref()
                .is_some_and(|v| source_is(v, "level1", 21655))
                && p.transform.translation
                    == [
                        -0.15660084784030914,
                        -0.27971217036247253,
                        -0.5606002807617188,
                    ]
                && p.transform.rotation
                    == [
                        0.7058812379837036,
                        0.7058789730072021,
                        0.04163286089897156,
                        0.04163298010826111,
                    ]
                && p.transform.scale == [100.00006103515625, 100.00003051757812, 100.00003051757812]
        }
        ("level1", 21655) => {
            p.parent
                .as_ref()
                .is_some_and(|v| source_is(v, "level1", 21481))
                && p.transform.translation
                    == [1.4890013933181763, 2.002000093460083, 11.198002815246582]
                && p.transform.rotation == [0.0, 0.0, -0.7071056962013245, 0.7071078419685364]
                && p.transform.scale == [1.0, 1.0, 1.0]
        }
        ("level1", 33862) => {
            p.parent
                .as_ref()
                .is_some_and(|v| source_is(v, "level1", 21567))
                && p.transform.translation
                    == [0.3570023775100708, 0.35050010681152344, -1.0168991088867188]
                && p.transform.rotation
                    == [
                        0.0,
                        0.058877840638160706,
                        -0.998265266418457,
                        9.126959810146218e-08,
                    ]
                && p.transform.scale == [100.00015258789062, 100.0001449584961, 100.0001220703125]
        }
        ("level1", 21567) => {
            p.parent
                .as_ref()
                .is_some_and(|v| source_is(v, "level1", 21481))
                && p.transform.translation
                    == [1.565999984741211, 1.8350000381469727, 9.581000328063965]
                && p.transform.rotation == [0.0, -1.0, -0.0, 0.0]
                && p.transform.scale == [1.0, 1.0, 1.0]
        }
        _ => false,
    }
}
