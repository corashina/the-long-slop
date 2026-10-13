//! Source default pole placement and conservative light influence admission.
use super::{AbsolutePose, GeneratedId, RoadSegment, WorldBounds, WorldFrame, WorldLayer};
use crate::reference::{
    LightId, ReferenceError, ReferenceManifest, RoadLampReference, WorldReference, WorldValue,
};
use rapier3d::math::Pose;

pub struct WorldLight {
    pub id: GeneratedId,
    pub pole: AbsolutePose,
    pub pose: AbsolutePose,
    pub source_light: LightId,
    pub range: f32,
}
pub(crate) struct SourceRoadLight {
    reference: RoadLampReference,
    pole_node: usize,
    local_light: Pose,
    source_light: LightId,
    range: f32,
    margin: f64,
    seed: i32,
}
impl SourceRoadLight {
    pub fn new(reference: &ReferenceManifest, seed: i32) -> Result<Self, ReferenceError> {
        let road = &reference.lighting.road_lamp;
        let world = &reference.world;
        let component = world
            .records
            .iter()
            .find(|r| r.source == road.source)
            .ok_or_else(|| ReferenceError("Missing default source pole controller".into()))?;
        let Some(WorldValue::Link(owner)) = component.fields.get("m_GameObject") else {
            return Err(ReferenceError("Missing source pole owner".into()));
        };
        let prototype = world
            .prototypes
            .iter()
            .find(|p| p.source == world.road.prototypes[0])
            .unwrap();
        let pole_node = prototype
            .nodes
            .iter()
            .position(|node| {
                world.records.iter().any(|r|r.source==node.source
            && matches!(r.fields.get("m_GameObject"),Some(WorldValue::Link(id)) if id==owner))
            })
            .ok_or_else(|| ReferenceError("Default pole is outside its road prototype".into()))?;
        let light = reference
            .lighting
            .lights
            .iter()
            .find(|l| road.lights.contains(&l.id))
            .ok_or_else(|| ReferenceError("Missing default road light".into()))?;
        let mut local_light = Pose::IDENTITY;
        let mut node = light
            .node
            .ok_or_else(|| ReferenceError("Road light has no node".into()))?;
        for _ in 0..reference.nodes.len() {
            if node == road.rotation_node {
                break;
            }
            let record = reference
                .nodes
                .iter()
                .find(|r| r.id == node)
                .ok_or_else(|| ReferenceError("Missing road light ancestor".into()))?;
            let ancestor = crate::vehicle::to_rapier(crate::vehicle::VehiclePose {
                position: record.transform.translation,
                rotation: record.transform.rotation,
            });
            local_light.translation = ancestor.translation
                + ancestor.rotation
                    * (rapier3d::math::Vector::from_array(record.transform.scale)
                        * local_light.translation);
            local_light.rotation = ancestor.rotation * local_light.rotation;
            node = record
                .parent
                .ok_or_else(|| ReferenceError("Road light has no pole ancestor".into()))?;
        }
        if node != road.rotation_node {
            return Err(ReferenceError("Road light ancestor cycle".into()));
        }
        let mut chain = Vec::new();
        let mut ancestor = Some(pole_node);
        while let Some(index) = ancestor {
            if index != 0 {
                chain.push(index);
            }
            ancestor = prototype.nodes[index].parent.map(|v| v as usize);
        }
        let mut radius = 0.0f64;
        let mut scale = 1.0f64;
        for index in chain.into_iter().rev() {
            let transform = &prototype.nodes[index].transform;
            radius += transform
                .translation
                .iter()
                .map(|v| (*v as f64).powi(2))
                .sum::<f64>()
                .sqrt()
                * scale;
            scale *= transform
                .scale
                .iter()
                .map(|v| v.abs() as f64)
                .fold(0.0, f64::max);
        }
        let margin =
            radius + local_light.translation.length() as f64 * scale + light.range_meters as f64;
        Ok(Self {
            reference: road.clone(),
            pole_node,
            local_light,
            source_light: light.id,
            range: light.range_meters,
            margin,
            seed,
        })
    }
    pub fn influence_margin(&self) -> f64 {
        self.margin
    }
    pub fn place(&self, reference: &WorldReference, segment: &RoadSegment) -> Option<WorldLight> {
        // Task7 supplies the winter and grass controller recipes.
        if segment.biome != 0 {
            return None;
        }
        let prototype = reference
            .prototypes
            .iter()
            .find(|p| p.source == reference.road.prototypes[0])
            .unwrap();
        let frames = super::road::road_frames(prototype, segment);
        let frame = frames[self.pole_node];
        let rotation = super::lamps::flatten_then_tilt(
            frame.rotation,
            super::lamps::source_angles(
                self.seed as u64,
                super::RoadLampId(segment.index as i64),
                &self.reference,
            ),
        );
        let position = std::array::from_fn(|i| segment.position()[i] + frame.position[i] as f64);
        let light_offset = rotation * self.local_light.translation;
        Some(WorldLight {
            id: GeneratedId {
                layer: WorldLayer::Lamp,
                generator: self.reference.generator_source.clone(),
                unit: [segment.index as i64, 0],
                ordinal: 0,
                children: vec![(self.reference.source.clone(), 0)],
            },
            pole: AbsolutePose {
                position,
                rotation: rotation.to_array(),
            },
            pose: AbsolutePose {
                position: std::array::from_fn(|i| position[i] + light_offset[i] as f64),
                rotation: (rotation * self.local_light.rotation).to_array(),
            },
            source_light: self.source_light,
            range: self.range,
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
