//! Existing generated world inputs use the same scene lighting without invented source identities.
use super::*;
use bevy::camera::primitives::MeshAabb;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeneratedWorldMaterial {
    Terrain,
    Stripe,
    Rock,
    Cactus,
    Post,
    Pump,
}
impl GeneratedWorldMaterial {
    pub fn material(self) -> SourceMaterial {
        let color = match self {
            Self::Terrain => Color::WHITE,
            Self::Stripe => Color::srgb(0.86, 0.75, 0.48),
            Self::Rock => Color::srgb(0.44, 0.32, 0.22),
            Self::Cactus => Color::srgb(0.25, 0.34, 0.19),
            Self::Post => Color::srgb(0.80, 0.75, 0.63),
            Self::Pump => Color::srgb(0.67, 0.18, 0.10),
        };
        // Preserve the former StandardMaterial inputs, including its dependency-owned default roughness.
        let base = StandardMaterial {
            base_color: color,
            perceptual_roughness: if matches!(self, Self::Terrain) {
                1.
            } else {
                StandardMaterial::default().perceptual_roughness
            },
            ..default()
        };
        from_base(base, 1, false)
    }
}
/// Called only during mesh generation. Reuse the resulting attributes for every draw and origin shift.
pub fn generated_mesh(mut mesh: Mesh) -> Mesh {
    let bounds = mesh
        .compute_aabb()
        .expect("generated world mesh has positions");
    let count = mesh.count_vertices();
    mesh.insert_attribute(
        crate::presentation::renderers::SOURCE_BOUNDS_CENTER,
        vec![[bounds.center.x, bounds.center.y, bounds.center.z, 1.]; count],
    );
    // Generated world casts and receives shadows and uses object motion, matching its existing behavior.
    mesh.insert_attribute(
        crate::presentation::renderers::SOURCE_BOUNDS_EXTENTS,
        vec![
            [
                bounds.half_extents.x,
                bounds.half_extents.y,
                bounds.half_extents.z,
                7.
            ];
            count
        ],
    );
    mesh
}

pub(super) fn from_base(base: StandardMaterial, mask: u32, reflected: bool) -> SourceMaterial {
    let mut surface = SourceSurface::default();
    surface.data.color = Vec4::from_array(base.base_color.to_linear().to_f32_array());
    surface.data.emission = Vec4::from_array(base.emissive.to_f32_array());
    surface.data.factors = Vec4::new(base.metallic, 1. - base.perceptual_roughness, 1., 0.5);
    surface.data.scales = Vec4::new(1., 1., 1., 0.);
    surface.data.flags.x = 1 << 11;
    surface.data.uv = [Vec4::new(1., 1., 0., 0.); 9];
    surface.state.reflected = reflected;
    surface.state.unlit = base.unlit;
    surface.state.standard = !base.unlit;
    surface.data.flags.z = u32::from(base.unlit);
    SourceMaterial {
        retained_color: None,
        mirror: None,
        base,
        extension: SourceExtension {
            receiver_mask: mask,
            surface,
        },
    }
}

#[derive(Component)]
pub(super) struct GeneratedLocalCaster;
#[derive(Resource, Default)]
pub(super) struct GeneratedCasterMaterials(
    HashMap<GeneratedWorldMaterial, Handle<StandardMaterial>>,
);

/// New streamed meshes acquire one local-lamp caster child during setup.
pub(super) fn prepare_local_casters(
    mut commands: Commands,
    mut cache: ResMut<GeneratedCasterMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<
        (Entity, &GeneratedWorldMaterial, &Mesh3d),
        (Without<GeneratedLocalCaster>, Without<SourceMaterialWarmup>),
    >,
) {
    for (entity, identity, mesh) in &query {
        let handle = cache
            .0
            .entry(*identity)
            .or_insert_with(|| materials.add(identity.material().base))
            .clone();
        commands.spawn((
            SourceCaster,
            mesh.clone(),
            MeshMaterial3d(handle),
            bevy::camera::visibility::RenderLayers::layer(32),
            Transform::default(),
            Visibility::Inherited,
            ChildOf(entity),
        ));
        commands.entity(entity).insert(GeneratedLocalCaster);
    }
}
