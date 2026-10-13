//! Source pole recipes and conservative light influence admission.
use super::spawn_rules::*;
use super::{AbsolutePose, GeneratedId, RoadSegment, WorldBounds, WorldFrame, WorldLayer};
use crate::reference::{
    LightId, LightKind, LightReference, ReferenceError, ReferenceManifest, RoadLampReference,
    ShadowReference, SourceId, Transform, WorldReference, WorldValue,
};
use rapier3d::math::Pose;

#[derive(Clone)]
pub struct WorldLight {
    pub id: GeneratedId,
    pub pole: AbsolutePose,
    pub pose: AbsolutePose,
    pub record: LightReference,
    pub biome: Option<u8>,
    pub prototype: SourceId,
    pub scale: [f32; 3],
    pub range: f32,
    pub enabled: bool,
    pub intensity: f32,
}
impl WorldLight {
    pub(crate) fn heap_bytes(&self) -> usize {
        super::contents::identity_heap(&self.id)
            + self.record.source.file.capacity()
            + self.prototype.file.capacity()
    }
}
struct RoadLightRecipe {
    reference: RoadLampReference,
    pole_node: usize,
    local_light: Pose,
    light: LightReference,
    margin: f64,
}
pub(crate) struct SourceRoadLight {
    recipes: Vec<RoadLightRecipe>,
    seed: i32,
}
fn record<'a>(
    world: &'a WorldReference,
    id: &SourceId,
) -> Result<&'a crate::reference::WorldSourceRecord, ReferenceError> {
    world
        .records
        .iter()
        .find(|r| &r.source == id)
        .ok_or_else(|| ReferenceError("Missing source light record".into()))
}
pub(crate) fn light_record(
    world: &WorldReference,
    id: &SourceId,
) -> Result<LightReference, ReferenceError> {
    let f = &record(world, id)?.fields;
    if optional(f, "m_Cookie")?.is_some() {
        return Err(ReferenceError(
            "World lamp cookie requires its checked texture binding".into(),
        ));
    }
    let c = fields(value(f, "m_Color")?)?;
    let shadows = fields(value(f, "m_Shadows")?)?;
    let mask = match value(fields(value(f, "m_CullingMask")?)?, "m_Bits")? {
        WorldValue::Integer(n) => *n as u32,
        _ => return Err(ReferenceError("Invalid source lamp mask".into())),
    };
    Ok(LightReference {
        id: LightId(0),
        source: id.clone(),
        node: None,
        pose: Transform {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        },
        kind: match integer(f, "m_Type")? {
            0 => LightKind::Spot,
            2 => LightKind::Point,
            _ => return Err(ReferenceError("Unresolved world lamp type".into())),
        },
        color: [
            number(c, "r")?,
            number(c, "g")?,
            number(c, "b")?,
            number(c, "a")?,
        ],
        serialized_intensity: number(f, "m_Intensity")?,
        startup_intensity: number(f, "m_Intensity")?,
        range_meters: number(f, "m_Range")?,
        full_cone_degrees: number(f, "m_SpotAngle")?,
        inner_cone_degrees: number(f, "m_InnerSpotAngle")?,
        enabled: flag(f, "m_Enabled")?,
        culling_mask: mask,
        shadows: ShadowReference {
            kind: integer(shadows, "m_Type")? as u8,
            strength: number(shadows, "m_Strength")?,
            bias: number(shadows, "m_Bias")?,
            normal_bias: number(shadows, "m_NormalBias")?,
            near_plane: number(shadows, "m_NearPlane")?,
            resolution: integer(shadows, "m_Resolution")?,
            custom_resolution: integer(shadows, "m_CustomResolution")?,
        },
        cookie: None,
        cookie_size: number(f, "m_CookieSize")?,
    })
}
impl SourceRoadLight {
    pub fn new(reference: &ReferenceManifest, seed: i32) -> Result<Self, ReferenceError> {
        let world = &reference.world;
        let mut recipes = Vec::with_capacity(3);
        for source in &world.road.prototypes {
            let prototype = world
                .prototypes
                .iter()
                .find(|p| &p.source == source)
                .unwrap();
            let controller=world.records.iter().find(|r| r.class_name=="roadscript" && matches!(r.fields.get("m_GameObject"),Some(WorldValue::Link(go)) if record(world,source).is_ok_and(|t|matches!(t.fields.get("m_GameObject"),Some(WorldValue::Link(owner)) if owner==go)))).ok_or_else(||ReferenceError("Missing biome road controller".into()))?;
            let f = &controller.fields;
            let pole = link(f, "randomPoleRotTransform")?;
            let pole_node = prototype
                .nodes
                .iter()
                .position(|n| &n.source == pole)
                .ok_or_else(|| ReferenceError("Missing biome pole node".into()))?;
            let koz = record(world, link(f, "koz")?)?;
            let light = light_record(world, link(&koz.fields, "LLight")?)?;
            let owner = link(&record(world, &light.source)?.fields, "m_GameObject")?;
            let mut node=prototype.nodes.iter().position(|n| record(world,&n.source).is_ok_and(|r|matches!(r.fields.get("m_GameObject"),Some(WorldValue::Link(go)) if go==owner))).ok_or_else(||ReferenceError("Missing biome light node".into()))?;
            let mut local_light = Pose::IDENTITY;
            for _ in 0..prototype.nodes.len() {
                if node == pole_node {
                    break;
                }
                let n = &prototype.nodes[node];
                let t = &n.transform;
                let ancestor = crate::vehicle::to_rapier(crate::vehicle::VehiclePose {
                    position: t.translation,
                    rotation: t.rotation,
                });
                local_light.translation = ancestor.translation
                    + ancestor.rotation
                        * (rapier3d::math::Vector::from_array(t.scale) * local_light.translation);
                local_light.rotation = ancestor.rotation * local_light.rotation;
                node = n
                    .parent
                    .ok_or_else(|| ReferenceError("Biome lamp is outside its pole".into()))?
                    as usize;
            }
            if node != pole_node {
                return Err(ReferenceError("Biome lamp ancestor cycle".into()));
            }
            let mut road = reference.lighting.road_lamp.clone();
            road.source = koz.source.clone();
            road.road_source = controller.source.clone();
            road.asset = prototype.asset;
            road.rotation_chances = [
                number(f, "randomPoleRotChance1")?,
                number(f, "randomPoleRotChance2")?,
            ];
            road.rotation_angles = [
                [
                    integer(f, "randomPoleRotAngleX1")?,
                    integer(f, "randomPoleRotAngleY1")?,
                    integer(f, "randomPoleRotAngleZ1")?,
                ],
                [
                    integer(f, "randomPoleRotAngleX2")?,
                    integer(f, "randomPoleRotAngleY2")?,
                    integer(f, "randomPoleRotAngleZ2")?,
                ],
            ];
            road.road_pose.scale = prototype.nodes[pole_node].transform.scale;
            let pole_pose = super::placement::node_pose(prototype, pole_node);
            let radius = pole_pose
                .translation
                .iter()
                .map(|v| (*v as f64).powi(2))
                .sum::<f64>()
                .sqrt();
            local_light.translation *= rapier3d::math::Vector::from_array(pole_pose.scale);
            road.road_pose.scale = pole_pose.scale;
            let margin =
                radius + local_light.translation.length() as f64 + light.range_meters as f64;
            recipes.push(RoadLightRecipe {
                reference: road,
                pole_node,
                local_light,
                light,
                margin,
            });
        }
        Ok(Self { recipes, seed })
    }
    pub fn influence_margin(&self) -> f64 {
        self.recipes.iter().map(|r| r.margin).fold(0., f64::max)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.recipes.capacity() * std::mem::size_of::<RoadLightRecipe>()
            + self
                .recipes
                .iter()
                .map(|r| {
                    r.reference.source.file.capacity()
                        + r.reference.road_source.file.capacity()
                        + r.reference.generator_source.file.capacity()
                        + r.reference.lights.capacity() * std::mem::size_of::<LightId>()
                        + r.light.source.file.capacity()
                })
                .sum::<usize>()
    }
    pub fn place(&self, reference: &WorldReference, segment: &RoadSegment) -> Option<WorldLight> {
        let recipe = &self.recipes[segment.biome as usize];
        let prototype = reference
            .prototypes
            .iter()
            .find(|p| p.source == reference.road.prototypes[segment.biome as usize])
            .unwrap();
        let frames = super::road::road_frames(prototype, segment);
        let frame = frames[recipe.pole_node];
        let rotation = super::lamps::flatten_then_tilt(
            frame.rotation,
            super::lamps::source_angles(
                self.seed as u64,
                super::RoadLampId(segment.index as i64),
                &recipe.reference,
            ),
        );
        let position = std::array::from_fn(|i| segment.position()[i] + frame.position[i] as f64);
        let offset = rotation * recipe.local_light.translation;
        Some(WorldLight {
            id: GeneratedId {
                layer: WorldLayer::Lamp,
                generator: recipe.reference.generator_source.clone(),
                unit: [segment.index as i64, 0],
                ordinal: 0,
                children: vec![(recipe.reference.source.clone(), 0)],
            },
            pole: AbsolutePose {
                position,
                rotation: rotation.to_array(),
            },
            pose: AbsolutePose {
                position: std::array::from_fn(|i| position[i] + offset[i] as f64),
                rotation: (rotation * recipe.local_light.rotation).to_array(),
            },
            range: recipe.light.range_meters,
            record: recipe.light.clone(),
            biome: Some(segment.biome),
            prototype: prototype.nodes[recipe.pole_node].source.clone(),
            scale: recipe.reference.road_pose.scale,
            enabled: true,
            intensity: recipe.light.startup_intensity,
        })
    }
}
/// Select all influencing source lights. Visibility and pole mesh distance do not reject light influence.
pub fn required_world_lights(
    frame: &WorldFrame<'_>,
    receiver_bounds: &[WorldBounds],
    output: &mut Vec<GeneratedId>,
) {
    let mut count = 0;
    for light in frame.lights {
        if receiver_bounds.iter().any(|bounds| {
            let squared = (0..3)
                .map(|i| {
                    let p = light.pose.position[i];
                    let d = if p < bounds.min[i] {
                        bounds.min[i] - p
                    } else if p > bounds.max[i] {
                        p - bounds.max[i]
                    } else {
                        0.
                    };
                    d * d
                })
                .sum::<f64>();
            squared <= (light.range as f64).powi(2)
        }) && !output[..count].contains(&light.id)
        {
            if count == output.len() {
                output.push(light.id.clone());
            } else if output[count] != light.id {
                output[count] = light.id.clone();
            }
            count += 1;
        }
    }
    output.truncate(count);
}
