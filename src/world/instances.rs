//! Checked prototype collision recipes, shared by source instances.
use super::{Placement, WorldOrigin};
use crate::reference::{
    ReferenceError, SourceId, TriggerRole, WorldPrototypeReference, WorldReference, WorldValue,
};
use crate::vehicle::{ColliderHandle, PhysicsWorld, VehiclePose, collider_builder, to_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{
    Collider, Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder, RigidBodyHandle,
};

struct BodyRecipe {
    builder: RigidBodyBuilder,
    pose: Pose,
}
struct ColliderRecipe {
    source: SourceId,
    body: Option<usize>,
    collider: Collider,
    /// Ladder up is in the collider frame.
    role: Option<TriggerRole>,
}

/// Roles of published trigger colliders. Ladder up is in the collider frame.
#[derive(Default)]
pub struct TriggerRoles(std::collections::HashMap<ColliderHandle, TriggerRole>);
impl TriggerRoles {
    /// Returns the role with ladder up in world space.
    pub fn get(&self, handle: ColliderHandle, collider: &Collider) -> Option<TriggerRole> {
        let mut role = *self.0.get(&handle)?;
        if let TriggerRole::Ladder { up, .. } = &mut role {
            *up = (collider.rotation() * Vector::from_array(*up)).to_array();
        }
        Some(role)
    }
    #[cfg(test)]
    pub(crate) fn insert(&mut self, handle: ColliderHandle, role: TriggerRole) {
        self.0.insert(handle, role);
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub struct StructureRecipe {
    pub cpu_bytes: usize,
    source: SourceId,
    scale: [f32; 3],
    bodies: Vec<BodyRecipe>,
    colliders: Vec<ColliderRecipe>,
}

pub(crate) fn fixed_decoration_source(source: &SourceId) -> bool {
    source.file == "sharedassets1.assets" && matches!(source.path_id, 84448 | 69558 | 66737 | 79572)
}

pub struct StructureInstance {
    pub(crate) fixed_decoration: bool,
    pub cpu_bytes: usize,
    pub placement: Placement,
    pub bodies: Vec<RigidBodyHandle>,
    pub colliders: Vec<(SourceId, ColliderHandle)>,
}
impl StructureRecipe {
    /// Collider construction and retained shape bounds include scaled input copies and topology storage.
    pub fn reservation(prototype: &WorldPrototypeReference) -> usize {
        use crate::reference::ColliderGeometry::*;
        std::mem::size_of::<Self>()
            + prototype.source.file.len()
            + prototype.bodies.len() * std::mem::size_of::<BodyRecipe>()
            + prototype
                .colliders
                .iter()
                .map(|c| {
                    std::mem::size_of::<ColliderRecipe>()
                        + c.source.file.len()
                        + match &c.geometry {
                            Some(Convex { points }) => points.len() * 512,
                            Some(Trimesh { points, triangles }) => {
                                points.len() * 48 + triangles.len() * 256
                            }
                            Some(_) => 512,
                            None => 0,
                        }
                })
                .sum::<usize>()
    }
    pub fn instance_bytes(&self) -> usize {
        std::mem::size_of::<StructureInstance>()
            + self.bodies.len() * std::mem::size_of::<RigidBodyHandle>()
            + self.colliders.len() * std::mem::size_of::<(SourceId, ColliderHandle)>()
            + self.bodies.len() * (std::mem::size_of::<rapier3d::prelude::RigidBody>() + 512)
            + self
                .colliders
                .iter()
                .map(|c| std::mem::size_of::<Collider>() + 512 + c.source.file.capacity())
                .sum::<usize>()
    }

    pub fn new(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
    ) -> Result<Self, ReferenceError> {
        Self::with_scale(reference, prototype, prototype.prefab_pose.scale)
    }
    pub fn with_scale(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
        scale: [f32; 3],
    ) -> Result<Self, ReferenceError> {
        Self::with_appearance(reference, prototype, scale, &[])
    }
    pub(crate) fn with_appearance(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
        scale: [f32; 3],
        appearance: &[super::spawn_appearance::NodeAppearance],
    ) -> Result<Self, ReferenceError> {
        let nonuniform = scale[0] != scale[1] || scale[1] != scale[2];
        let checked_nonuniform = fixed_decoration_source(&prototype.source)
            && prototype
                .bodies
                .iter()
                .all(|b| b.pose.rotation == [0., 0., 0., 1.])
            && prototype
                .colliders
                .iter()
                .filter(|c| c.source_empty_mesh.is_none())
                .all(|c| {
                    matches!(
                        c.geometry,
                        Some(
                            crate::reference::ColliderGeometry::Convex { .. }
                                | crate::reference::ColliderGeometry::Trimesh { .. }
                        )
                    )
                });
        if scale.iter().any(|v| !v.is_finite() || *v <= 0.) || (nonuniform && !checked_nonuniform) {
            return Err(ReferenceError(
                "Unresolved source structure collision scale or rotated body".into(),
            ));
        }
        let active = |mut node: usize| {
            for _ in 0..prototype.nodes.len() {
                let record = &prototype.nodes[node];
                if !record.active {
                    return false;
                }
                let Some(parent) = record.parent else {
                    return true;
                };
                node = parent as usize;
            }
            false
        };
        let mut bodies = Vec::with_capacity(prototype.bodies.len());
        for body in &prototype.bodies {
            let constraints = body.constraints;
            let builder = if body.kinematic {
                RigidBodyBuilder::kinematic_position_based()
            } else {
                RigidBodyBuilder::dynamic().ccd_enabled(true)
            }
            .enabled(active(body.node as usize))
            .additional_mass(body.mass_kg)
            .linear_damping(body.linear_damping)
            .angular_damping(body.angular_damping)
            .gravity_scale(if body.use_gravity { 1.0 } else { 0.0 })
            .enabled_translations(
                constraints & 2 == 0,
                constraints & 4 == 0,
                constraints & 8 == 0,
            )
            .enabled_rotations(
                constraints & 16 == 0,
                constraints & 32 == 0,
                constraints & 64 == 0,
            );
            bodies.push(BodyRecipe {
                builder,
                pose: to_rapier(VehiclePose {
                    position: std::array::from_fn(|i| body.pose.translation[i] * scale[i]),
                    rotation: body.pose.rotation,
                }),
            });
        }
        let mut colliders = Vec::with_capacity(prototype.colliders.len());
        for record in &prototype.colliders {
            if record.source_empty_mesh.is_some() {
                continue;
            }
            let geometry = record.geometry.as_ref().ok_or_else(|| {
                ReferenceError(format!(
                    "Structure collider {}:{} has no checked geometry",
                    record.source.file, record.source.path_id
                ))
            })?;
            let (friction, restitution) = if let Some(source) = &record.material {
                let material = reference
                    .records
                    .iter()
                    .find(|r| r.source == *source)
                    .ok_or_else(|| {
                        ReferenceError(format!(
                            "Missing physical material {}:{}",
                            source.file, source.path_id
                        ))
                    })?;
                let number = |name: &str| match material.fields.get(name) {
                    Some(WorldValue::Number(value)) => Ok(*value as f32),
                    Some(WorldValue::Integer(value)) => Ok(*value as f32),
                    _ => Err(ReferenceError(format!("Physical material field {name}"))),
                };
                (number("m_DynamicFriction")?, number("m_Bounciness")?)
            } else {
                (0.6, 0.0)
            };
            let layer = prototype.nodes[record.node as usize].layer as usize;
            let deformed = node_scaled_geometry(prototype, record, geometry, appearance)?;
            let scaled = scaled_geometry(&deformed, scale);
            let collider = collider_builder(&scaled)?
                .friction(friction)
                .restitution(restitution)
                .density(0.0)
                .enabled(record.enabled && active(record.node as usize))
                .sensor(record.trigger)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(1u32 << layer),
                    Group::from_bits_truncate(reference.collision_rules.layers[layer]),
                    InteractionTestMode::And,
                ))
                .build();
            // Exported ladder up is in the body frame, or in prototype space without a body.
            let role = record.role.map(|mut role| {
                if let TriggerRole::Ladder { up, .. } = &mut role {
                    *up = (collider.rotation().inverse() * Vector::from_array(*up)).to_array();
                }
                role
            });
            colliders.push(ColliderRecipe {
                source: record.source.clone(),
                body: record.body.map(|i| i as usize),
                collider,
                role,
            });
        }
        Ok(Self {
            cpu_bytes: Self::reservation(prototype),
            source: prototype.source.clone(),
            scale,
            bodies,
            colliders,
        })
    }

    /// All source validation finishes before any live physics state changes.
    pub fn publish(
        &self,
        placement: Placement,
        origin: WorldOrigin,
        fixed_decoration: bool,
        physics: &mut PhysicsWorld,
        roles: &mut TriggerRoles,
    ) -> Result<StructureInstance, ReferenceError> {
        if placement.prototype != self.source {
            return Err(ReferenceError(
                "Structure placement does not match its checked recipe".into(),
            ));
        }
        if placement.scale != self.scale {
            return Err(ReferenceError(
                "Source structure placement scale changed".into(),
            ));
        }
        let root = to_rapier(VehiclePose {
            position: origin.local(placement.pose.position),
            rotation: placement.pose.rotation,
        });
        let mut bodies = Vec::with_capacity(self.bodies.len());
        let mut colliders = Vec::with_capacity(self.colliders.len());

        for body in &self.bodies {
            let mut body = body.builder.clone().pose(root * body.pose).build();
            if fixed_decoration {
                body.set_body_type(rapier3d::prelude::RigidBodyType::Fixed, false);
            }
            bodies.push(physics.insert_body(body));
        }
        for source in &self.colliders {
            let mut collider = source.collider.clone();
            let handle = if let Some(body) = source.body {
                physics.insert_collider_with_parent(collider, bodies[body])
            } else {
                collider.set_position(root * *collider.position());
                physics.insert_collider(collider)
            };
            if let Some(role) = source.role {
                roles.0.insert(handle, role);
            }
            colliders.push((source.source.clone(), handle));
        }
        physics.refresh_queries();
        Ok(StructureInstance {
            fixed_decoration,
            cpu_bytes: self.instance_bytes() + super::contents::placement_heap(&placement),
            placement,
            bodies,
            colliders,
        })
    }
}
impl StructureInstance {
    /// The caller removes item and interaction bindings before retiring this instance.
    pub fn retire(self, physics: &mut PhysicsWorld, roles: &mut TriggerRoles) {
        for (_, collider) in self.colliders {
            roles.0.remove(&collider);
            physics.remove_collider(collider);
        }
        for body in self.bodies {
            physics.remove_body(body);
        }
        physics.refresh_queries();
    }
}

fn scaled_geometry(
    source: &crate::reference::ColliderGeometry,
    scale: [f32; 3],
) -> crate::reference::ColliderGeometry {
    use crate::reference::ColliderGeometry::*;
    let point = |p: [f32; 3]| std::array::from_fn(|i| p[i] * scale[i]);
    match source {
        Box {
            center,
            half_extents,
            rotation,
        } => Box {
            center: point(*center),
            half_extents: point(*half_extents),
            rotation: *rotation,
        },
        Sphere { center, radius } => Sphere {
            center: point(*center),
            radius: radius * scale[0],
        },
        Capsule { a, b, radius } => Capsule {
            a: point(*a),
            b: point(*b),
            radius: radius * scale[0],
        },
        Convex { points } => Convex {
            points: points.iter().copied().map(point).collect(),
        },
        Trimesh { points, triangles } => Trimesh {
            points: points.iter().copied().map(point).collect(),
            triangles: triangles.clone(),
        },
    }
}

/// Source mesh points are baked in the owning body frame. Apply child scale there.
fn node_scaled_geometry(
    prototype: &WorldPrototypeReference,
    record: &crate::reference::WorldColliderReference,
    geometry: &crate::reference::ColliderGeometry,
    appearance: &[super::spawn_appearance::NodeAppearance],
) -> Result<crate::reference::ColliderGeometry, ReferenceError> {
    use crate::reference::ColliderGeometry;
    use rapier3d::math::{Rotation, Vector};
    let mut chain = Vec::new();
    let mut node = Some(record.node as usize);
    while let Some(i) = node {
        chain.push(i);
        node = prototype.nodes[i].parent.map(|n| n as usize);
    }
    if !chain.iter().any(|&i| {
        appearance
            .iter()
            .any(|a| a.node == prototype.nodes[i].source && a.scale.is_some())
    }) {
        return Ok(geometry.clone());
    }
    let body = record
        .body
        .map(|b| {
            let t = prototype.bodies[b as usize].pose;
            to_rapier(VehiclePose {
                position: t.translation,
                rotation: t.rotation,
            })
        })
        .unwrap_or(Pose::IDENTITY);
    let point = |p: [f32; 3]| {
        let mut p = body * Vector::from_array(p);
        for &i in chain.iter().rev() {
            let t = prototype.nodes[i].transform;
            p = (Rotation::from_array(t.rotation).inverse()
                * (p - Vector::from_array(t.translation)))
                / Vector::from_array(t.scale);
        }
        for &i in &chain {
            let t = prototype.nodes[i].transform;
            let scale = appearance
                .iter()
                .rev()
                .find(|a| a.node == prototype.nodes[i].source && a.scale.is_some())
                .and_then(|a| a.scale)
                .unwrap_or(t.scale);
            p = Rotation::from_array(t.rotation) * (p * Vector::from_array(scale))
                + Vector::from_array(t.translation);
        }
        (body.inverse() * p).to_array()
    };
    Ok(match geometry {
        ColliderGeometry::Convex { points } => ColliderGeometry::Convex {
            points: points.iter().copied().map(point).collect(),
        },
        ColliderGeometry::Trimesh { points, triangles } => ColliderGeometry::Trimesh {
            points: points.iter().copied().map(point).collect(),
            triangles: triangles.clone(),
        },
        _ => {
            return Err(ReferenceError(
                "Source random scale affects an untraced primitive collider".into(),
            ));
        }
    })
}
