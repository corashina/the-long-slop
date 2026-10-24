//! Checked spirit level, spray bottle, genie lamp, start pack, and mouth smoke inputs.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmallItemsReference {
    pub spirit_level: SpiritLevelReference,
    pub spray_bottle: SprayBottleReference,
    pub genie_lamp: GenieLampReference,
    pub start_packs: Vec<SmallItemStartReference>,
    pub exhale: ExhaleReference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpiritLevelReference {
    pub item: u32,
    pub prototype: SourceId,
    pub source: SourceId,
    pub bubbles: Vec<BubbleReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BubbleReference {
    pub bubble: SourceId,
    pub t1: SourceId,
    pub t2: SourceId,
    pub lerp: f32,
    pub bubble_curve: TorqueCurve,
    pub scale_z_curve: TorqueCurve,
    pub scale_xy_curve: TorqueCurve,
    pub move_xy_curve: TorqueCurve,
    pub min_scale_xy: f32,
    pub max_scale_xy: f32,
    pub min_scale_z: f32,
    pub max_scale_z: f32,
    pub move_xy: f32,
    pub startup_value: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SprayBottleReference {
    pub item: u32,
    pub prototype: SourceId,
    pub source: SourceId,
    pub ammo: AcquisitionAmmo,
    pub cleaning_type: u8,
    pub text: u32,
    pub angle_speed: f32,
    pub trigger_max_angle: f32,
    pub trigger_set_angle: f32,
    pub trigger_reset_angle: f32,
    pub invert: bool,
    pub trigger: SourceId,
    pub hole: SourceId,
    pub set_clip: SourceId,
    pub reset_clip: SourceId,
    pub set_volume: f32,
    pub reset_volume: f32,
    pub mask: u32,
    pub gain: f32,
    pub distance: f32,
    pub wash_radius: f32,
    pub wash_mask: u32,
    pub spray: ParticleReference,
    pub splash: ParticleReference,
    pub startup: Vec<SmallItemStartReference>,
}

/// A StartSpawnItems copy. Its pose comes from the world startup records.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmallItemStartReference {
    pub source: SourceId,
    pub transform: SourceId,
    pub item_id: u32,
    pub ammo: AcquisitionAmmo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenieLampReference {
    pub item: u32,
    pub prototype: SourceId,
    pub source: SourceId,
    pub light: SourceId,
    pub renderer: SourceId,
    pub material: SourceId,
    pub wash_trigger: SourceId,
    pub gain: f32,
    pub lose: f32,
    pub intensity: TorqueCurve,
    pub range: TorqueCurve,
    pub emission: GradientReference,
    pub startup_value: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExhaleReference {
    pub controller: SourceId,
    pub node: NodeId,
    pub max_rate: f32,
    pub charge_speed: f32,
    pub particle: ParticleReference,
}

const SPIRIT_LEVEL_ITEM: u32 = 401;
const SPRAY_BOTTLE_ITEM: u32 = 121;
const GENIE_LAMP_ITEM: u32 = 211;
const PACK_ITEM: u32 = 74;
const START_BOTTLES: usize = 2;
const START_PACKS: usize = 3;

fn invalid(field: &str) -> ReferenceError {
    ReferenceError(format!("Invalid small item field {field}"))
}

fn checked(ok: bool, field: &str) -> Result<(), ReferenceError> {
    if ok { Ok(()) } else { Err(invalid(field)) }
}

fn curve(c: &TorqueCurve) -> bool {
    !c.keys.is_empty() && particles::curve_valid(c)
}

impl SmallItemsReference {
    pub fn validate(&self, m: &ReferenceManifest) -> Result<(), ReferenceError> {
        let records: HashMap<&SourceId, &WorldSourceRecord> =
            m.world.records.iter().map(|r| (&r.source, r)).collect();
        let class = |source: &SourceId, name: &str| {
            records
                .get(source)
                .is_some_and(|r| r.class_name == name && r.fields.len() > 1)
        };
        // Each script lies on a selected catalog prototype; its nodes are prototype nodes.
        let prototype = |item: u32, name: &str, source: &SourceId| {
            let entry = m.world.catalog.items.get(item as usize);
            let owner = entry
                .filter(|i| i.index == item && i.name == name)
                .and_then(|i| i.prototype.as_ref())
                .filter(|p| *p == source)
                .and_then(|p| m.world.prototypes.iter().find(|w| &w.source == p));
            owner.ok_or_else(|| invalid(name))
        };
        let inside = |p: &WorldPrototypeReference, node: &SourceId| {
            p.nodes.iter().any(|n| &n.source == node)
        };

        let l = &self.spirit_level;
        let p = prototype(SPIRIT_LEVEL_ITEM, "waterlevel", &l.prototype)?;
        checked(
            l.item == SPIRIT_LEVEL_ITEM && class(&l.source, "bubbleLevelScript"),
            "spirit level",
        )?;
        checked(l.bubbles.len() == 3, "spirit level bubbles")?;
        for b in &l.bubbles {
            checked(
                [&b.bubble, &b.t1, &b.t2].iter().all(|n| inside(p, n))
                    && b.t1 != b.t2
                    && [
                        &b.bubble_curve,
                        &b.scale_z_curve,
                        &b.scale_xy_curve,
                        &b.move_xy_curve,
                    ]
                    .iter()
                    .all(|c| curve(c))
                    && finite(&[
                        b.lerp,
                        b.min_scale_xy,
                        b.max_scale_xy,
                        b.min_scale_z,
                        b.max_scale_z,
                        b.move_xy,
                        b.startup_value,
                    ])
                    && b.lerp > 0.
                    && b.min_scale_xy > 0.
                    && b.min_scale_xy <= b.max_scale_xy
                    && b.min_scale_z > 0.
                    && b.min_scale_z <= b.max_scale_z
                    && (0. ..=1.).contains(&b.startup_value),
                "spirit level bubble",
            )?;
        }

        let s = &self.spray_bottle;
        let p = prototype(SPRAY_BOTTLE_ITEM, "jizzonit", &s.prototype)?;
        let ammo = |a: &AcquisitionAmmo| {
            class(&a.source, "ammoscript")
                && finite(&[
                    a.float_count,
                    a.maximum,
                    a.random_range[0],
                    a.random_range[1],
                ])
                && a.maximum > 0.
        };
        checked(
            s.item == SPRAY_BOTTLE_ITEM
                && class(&s.source, "spricniscript")
                && ammo(&s.ammo)
                && inside(p, &s.trigger)
                && inside(p, &s.hole)
                && item_prototypes::clip_exists(m, &s.set_clip)
                && item_prototypes::clip_exists(m, &s.reset_clip)
                && finite(&[
                    s.angle_speed,
                    s.trigger_max_angle,
                    s.trigger_set_angle,
                    s.trigger_reset_angle,
                    s.set_volume,
                    s.reset_volume,
                    s.gain,
                    s.distance,
                    s.wash_radius,
                ])
                && s.angle_speed > 0.
                && 0. < s.trigger_reset_angle
                && s.trigger_reset_angle < s.trigger_set_angle
                && s.trigger_set_angle < s.trigger_max_angle
                && (0. ..=1.).contains(&s.set_volume)
                && (0. ..=1.).contains(&s.reset_volume)
                && s.gain > 0.
                && s.distance > 0.
                && s.wash_radius > 0.
                && s.startup.len() == START_BOTTLES,
            "spray bottle",
        )?;
        conditions::validate_particle(&s.source, &s.spray, m)?;
        conditions::validate_particle(&s.source, &s.splash, m)?;
        let start = |v: &SmallItemStartReference, script: &str, item: u32| {
            v.source.file == "level1"
                && class(&v.source, script)
                && records
                    .get(&v.transform)
                    .is_some_and(|r| r.class_name == "Transform")
                && v.item_id == item
                && ammo(&v.ammo)
        };
        for v in &s.startup {
            checked(
                start(v, "spricniscript", SPRAY_BOTTLE_ITEM) && v.ammo.r#type == s.ammo.r#type,
                "start spray bottle",
            )?;
        }

        let g = &self.genie_lamp;
        let p = prototype(GENIE_LAMP_ITEM, "ginlamp", &g.prototype)?;
        let on_prototype = |source: &SourceId, name: &str| {
            records.get(source).is_some_and(|r| {
                r.class_name == name
                    && matches!(r.fields.get("m_GameObject"), Some(WorldValue::Link(go))
                        if records.contains_key(go))
            })
        };
        checked(
            g.item == GENIE_LAMP_ITEM
                && class(&g.source, "ginlampscript")
                && class(&g.wash_trigger, "washtriggerscript")
                && on_prototype(&g.light, "Light")
                && on_prototype(&g.renderer, "MeshRenderer")
                && !p.nodes.is_empty()
                && !g.material.file.is_empty()
                && curve(&g.intensity)
                && curve(&g.range)
                && particles::gradient_valid(&g.emission)
                && finite(&[g.gain, g.lose, g.startup_value])
                && 0. < g.lose
                && g.lose < g.gain
                && (0. ..=1.).contains(&g.startup_value),
            "genie lamp",
        )?;

        checked(self.start_packs.len() == START_PACKS, "start packs")?;
        for v in &self.start_packs {
            checked(
                start(v, "cigiscript", PACK_ITEM) && v.ammo.r#type == 5 && !v.ammo.float_or_int,
                "start pack",
            )?;
        }
        ids(
            s.startup
                .iter()
                .chain(&self.start_packs)
                .map(|v| &v.transform),
            "duplicate small item start",
        )?;

        let e = &self.exhale;
        checked(
            e.node.0 >= 1
                && m.nodes
                    .get(e.node.0 as usize - 1)
                    .is_some_and(|n| n.id == e.node)
                && finite(&[e.max_rate, e.charge_speed])
                && e.max_rate > 0.
                && e.charge_speed > 0.,
            "mouth smoke",
        )?;
        conditions::validate_particle(&e.controller, &e.particle, m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(time: f32, value: f32) -> CurveKey {
        CurveKey {
            time,
            value,
            in_tangent: 0.,
            out_tangent: 0.,
            in_weight: 0.,
            out_weight: 0.,
            weighted_mode: 0,
        }
    }

    #[test]
    fn curves_need_ordered_finite_keys() {
        let mut c = TorqueCurve {
            keys: vec![key(0., 0.), key(1., 5.)],
            pre_wrap_mode: 2,
            post_wrap_mode: 2,
        };
        assert!(curve(&c));
        c.keys.swap(0, 1);
        assert!(!curve(&c));
        c.keys.clear();
        assert!(!curve(&c));
        c.keys.push(key(0., f32::NAN));
        assert!(!curve(&c));
    }

    #[test]
    fn records_reject_unknown_fields() {
        let start = r#"{"source":{"file":"level1","path_id":80187},"transform":{"file":"level1","path_id":21658},
            "item_id":74,"ammo":{"source":{"file":"level1","path_id":77156},"count":20,"float_count":0.0,
            "maximum":20.0,"float_or_int":false,"type":5,"random_range":[20.0,1.0]}}"#;
        let parsed: SmallItemStartReference = serde_json::from_str(start).unwrap();
        assert_eq!(parsed.item_id, 74);
        assert_eq!(parsed.ammo.r#type, 5);
        let extra = start.replacen("\"item_id\"", "\"extra\":1,\"item_id\"", 1);
        assert!(serde_json::from_str::<SmallItemStartReference>(&extra).is_err());
    }
}
