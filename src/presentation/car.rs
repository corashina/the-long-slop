use crate::vehicle::WHEEL_RADIUS;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;

#[derive(Component)]
pub struct ChassisVisual;

#[derive(Component)]
pub struct WheelVisual(pub usize);

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let paint = materials.add(Color::srgb(0.62, 0.27, 0.12));
    let trim = materials.add(Color::srgb(0.12, 0.10, 0.09));
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.59, 0.62, 0.24),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.3,
        ..default()
    });
    let lamp = materials.add(Color::srgb(0.93, 0.82, 0.55));
    let metal = materials.add(Color::srgb(0.52, 0.48, 0.40));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    commands
        .spawn((ChassisVisual, Transform::default(), Visibility::default()))
        .with_children(|body| {
            for (size, position, material) in [
                ([1.52, 0.46, 3.25], [0.0, 0.0, 0.0], paint.clone()),
                ([1.45, 0.12, 1.05], [0.0, 0.28, -1.06], paint.clone()),
                ([1.44, 0.10, 1.55], [0.0, 0.95, 0.2], paint.clone()),
                ([1.5, 0.14, 0.15], [0.0, -0.14, -1.68], metal.clone()),
                ([1.5, 0.14, 0.15], [0.0, -0.14, 1.68], metal.clone()),
                ([1.35, 0.18, 0.30], [0.0, 0.37, -0.54], trim.clone()),
                ([0.48, 0.52, 0.25], [-0.36, 0.45, 0.52], trim.clone()),
                ([0.48, 0.52, 0.25], [0.36, 0.45, 0.52], trim.clone()),
                ([0.30, 0.14, 0.08], [-0.52, 0.10, -1.66], lamp.clone()),
                ([0.30, 0.14, 0.08], [0.52, 0.10, -1.66], lamp.clone()),
                ([0.04, 0.48, 1.15], [-0.74, 0.64, 0.22], glass.clone()),
                ([0.04, 0.48, 1.15], [0.74, 0.64, 0.22], glass.clone()),
            ] {
                body.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(Vec3::from_array(position))
                        .with_scale(Vec3::from_array(size)),
                ));
            }
            for x in [-0.70, 0.70] {
                for z in [-0.54, 0.92] {
                    body.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(paint.clone()),
                        Transform::from_xyz(x, 0.62, z).with_scale(Vec3::new(0.07, 0.63, 0.07)),
                    ));
                }
            }
            // The driver sees the wheel and dashboard below the windshield.
            body.spawn((
                Mesh3d(meshes.add(Torus::new(0.15, 0.19))),
                MeshMaterial3d(trim.clone()),
                Transform::from_xyz(-0.36, 0.47, -0.30).with_rotation(Quat::from_rotation_x(0.95)),
            ));
        });
    let tire = meshes.add(Cylinder::new(WHEEL_RADIUS, 0.22));
    let hub = meshes.add(Cylinder::new(0.16, 0.23));
    for i in 0..4 {
        commands
            .spawn((WheelVisual(i), Transform::default(), Visibility::default()))
            .with_children(|wheel| {
                wheel.spawn((
                    Mesh3d(tire.clone()),
                    MeshMaterial3d(trim.clone()),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ));
                wheel.spawn((
                    Mesh3d(hub.clone()),
                    MeshMaterial3d(metal.clone()),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ));
            });
    }
}
