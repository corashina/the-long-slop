//! Checked generated LODGroup membership. Runtime coefficients come from the source culling path.
use crate::reference::{
    ReferenceError, SourceId, WorldPrototypeReference, WorldReference, WorldValue,
};
use bevy::math::{Mat4, Vec3};
#[derive(Clone, Copy)]
pub struct LodView {
    pub position: Vec3,
    pub fov_degrees: f32,
    pub ortho_size: Option<f32>,
}
#[derive(Clone, Copy)]
pub struct LodSelection {
    pub first: usize,
    pub fade: f32,
    pub transition: bool,
}
pub struct LodLevel {
    pub height: f32,
    pub width: f32,
    pub nodes: Vec<usize>,
}
pub struct LodGroup {
    pub source: SourceId,
    pub node: usize,
    pub point: Vec3,
    pub size: f32,
    pub levels: Vec<LodLevel>,
}
fn link(value: Option<&WorldValue>) -> Option<&SourceId> {
    match value {
        Some(WorldValue::Link(value)) => Some(value),
        _ => None,
    }
}
fn number(value: Option<&WorldValue>) -> Result<f32, ReferenceError> {
    match value {
        Some(WorldValue::Number(value)) => Ok(*value as f32),
        Some(WorldValue::Integer(value)) => Ok(*value as f32),
        _ => Err(ReferenceError("Missing checked LOD number".into())),
    }
}
impl LodGroup {
    pub fn for_prototype<'a>(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
        owner: impl Fn(&SourceId) -> Option<&'a SourceId>,
    ) -> Result<Vec<Self>, ReferenceError> {
        let node_for = |node_owner: &SourceId| {
            prototype
                .nodes
                .iter()
                .position(|n| owner(&n.source) == Some(node_owner))
        };
        let mut result = Vec::new();
        for record in reference
            .records
            .iter()
            .filter(|r| r.class_name == "LODGroup")
        {
            let Some(node) = owner(&record.source).and_then(node_for) else {
                continue;
            };
            if matches!(
                record.fields.get("m_Enabled"),
                Some(WorldValue::Bool(false))
            ) {
                continue;
            }
            if !matches!(
                record.fields.get("m_FadeMode"),
                Some(WorldValue::Integer(1))
            ) || !matches!(
                record.fields.get("m_AnimateCrossFading"),
                Some(WorldValue::Bool(false))
            ) {
                return Err(ReferenceError(
                    "Unsupported source LOD transition mode".into(),
                ));
            }
            let Some(WorldValue::Record(point)) = record.fields.get("m_LocalReferencePoint") else {
                return Err(ReferenceError("Missing source LOD reference point".into()));
            };
            let point = Vec3::new(
                number(point.get("x"))?,
                number(point.get("y"))?,
                -number(point.get("z"))?,
            );
            let Some(WorldValue::Array(entries)) = record.fields.get("m_LODs") else {
                return Err(ReferenceError("Missing source LOD levels".into()));
            };
            let mut levels = Vec::with_capacity(entries.len());
            for entry in entries {
                let WorldValue::Record(fields) = entry else {
                    return Err(ReferenceError("Invalid source LOD level".into()));
                };
                let Some(WorldValue::Array(renderers)) = fields.get("renderers") else {
                    return Err(ReferenceError("Missing LOD renderer membership".into()));
                };
                let mut nodes = Vec::with_capacity(renderers.len());
                for renderer in renderers {
                    let WorldValue::Record(fields) = renderer else {
                        return Err(ReferenceError("Invalid LOD renderer entry".into()));
                    };
                    if let Some(source) = link(fields.get("renderer")) {
                        let index = owner(source).and_then(node_for).ok_or_else(|| {
                            ReferenceError("LOD renderer lies outside its prototype".into())
                        })?;
                        nodes.push(index);
                    }
                }
                levels.push(LodLevel {
                    height: number(fields.get("screenRelativeHeight"))?,
                    width: number(fields.get("fadeTransitionWidth"))?,
                    nodes,
                });
            }
            result.push(Self {
                source: record.source.clone(),
                node,
                point,
                size: number(record.fields.get("m_Size"))?,
                levels,
            });
        }
        Ok(result)
    }
    /// UnityPlayer 3f99f0 selection and checked camera factor, and checked static width coefficients.
    pub fn select(
        &self,
        world: Mat4,
        view: LodView,
        bias: f32,
        maximum: usize,
    ) -> Option<LodSelection> {
        let scale = Vec3::new(
            world.x_axis.truncate().length(),
            world.y_axis.truncate().length(),
            world.z_axis.truncate().length(),
        );
        let size = scale.max_element() * self.size;
        let center = world.transform_point3(self.point);
        let distance = if let Some(half_height) = view.ortho_size {
            2.0 * half_height / bias
        } else {
            let angle = view.fov_degrees / 360.0;
            let angle = angle + angle;
            let angle = angle * std::f32::consts::PI;
            let angle = angle * 0.5;
            let factor = ((angle.tan() as f64) * 2.0) as f32 / bias;
            ((view.position - center).length_squared() * (factor * factor)).sqrt()
        };
        for (index, level) in self.levels.iter().enumerate().skip(maximum) {
            let threshold = size / level.height;
            if distance >= threshold {
                continue;
            }
            let previous = if index == 0 {
                1.0
            } else {
                self.levels[index - 1].height
            };
            let width = if size.abs() <= 1.0e-6 || level.width <= 1.0e-5 {
                0.0
            } else {
                (threshold - size / (level.height + (previous - level.height) * level.width))
                    .max(0.0)
            };
            let remaining = threshold - distance;
            let transition = width > 0.0 && remaining < width;
            return Some(LodSelection {
                first: index,
                fade: if transition { remaining / width } else { 1.0 },
                transition,
            });
        }
        None
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source.file.capacity()
            + self.levels.capacity() * std::mem::size_of::<LodLevel>()
            + self
                .levels
                .iter()
                .map(|l| l.nodes.capacity() * std::mem::size_of::<usize>())
                .sum::<usize>()
    }
}
