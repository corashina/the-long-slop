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
    let paint = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.25, 0.13),
        perceptual_roughness: 0.85,
        metallic: 0.12,
        ..default()
    });
    let faded = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.41, 0.25),
        perceptual_roughness: 0.95,
        ..default()
    });
    let rust = materials.add(StandardMaterial {
        base_color: Color::srgb(0.27, 0.14, 0.08),
        perceptual_roughness: 1.0,
        ..default()
    });
    let trim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.09, 0.08, 0.07),
        perceptual_roughness: 0.95,
        ..default()
    });
    let glass = materials.add(StandardMaterial {
        base_color: Color::srgba(0.36, 0.47, 0.45, 0.22),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.45,
        ..default()
    });
    let lamp = materials.add(Color::srgb(0.93, 0.82, 0.55));
    let tail_lamp = materials.add(Color::srgb(0.55, 0.09, 0.04));
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.48, 0.40),
        metallic: 0.65,
        perceptual_roughness: 0.65,
        ..default()
    });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    commands
        .spawn((ChassisVisual, Transform::default(), Visibility::default()))
        .with_children(|body| {
            for (size, position, material) in [
                ([1.52, 0.46, 3.25], [0.0, 0.0, 0.0], paint.clone()),
                ([1.45, 0.12, 1.05], [0.0, 0.28, -1.06], paint.clone()),
                ([1.44, 0.10, 1.55], [0.0, 0.95, 0.2], paint.clone()),
                ([1.32, 0.018, 1.32], [0.0, 1.008, 0.18], faded.clone()),
                ([0.61, 0.016, 0.42], [-0.18, 0.348, -1.12], faded.clone()),
                ([0.21, 0.018, 0.16], [0.48, 0.35, -0.86], rust.clone()),
                ([1.5, 0.14, 0.15], [0.0, -0.14, -1.68], metal.clone()),
                ([1.5, 0.14, 0.15], [0.0, -0.14, 1.68], metal.clone()),
                ([1.35, 0.18, 0.30], [0.0, 0.37, -0.54], trim.clone()),
                ([0.48, 0.52, 0.25], [-0.36, 0.45, 0.52], trim.clone()),
                ([0.48, 0.52, 0.25], [0.36, 0.45, 0.52], trim.clone()),
                ([0.30, 0.14, 0.08], [-0.52, 0.10, -1.66], lamp.clone()),
                ([0.30, 0.14, 0.08], [0.52, 0.10, -1.66], lamp.clone()),
                ([0.28, 0.12, 0.035], [-0.55, 0.10, 1.64], tail_lamp.clone()),
                ([0.28, 0.12, 0.035], [0.55, 0.10, 1.64], tail_lamp.clone()),
                ([0.36, 0.10, 0.025], [0.0, -0.06, 1.64], lamp.clone()),
                ([1.36, 0.035, 0.035], [0.0, 0.37, 0.96], trim.clone()),
                ([1.36, 0.48, 0.025], [0.0, 0.64, 0.96], glass.clone()),
                ([1.36, 0.48, 0.025], [0.0, 0.64, -0.57], glass.clone()),
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
            for side in [-1.0, 1.0] {
                for (size, position, material) in [
                    (
                        [0.015, 0.23, 0.75],
                        [side * 0.766, 0.02, 0.20],
                        faded.clone(),
                    ),
                    (
                        [0.018, 0.03, 1.42],
                        [side * 0.765, -0.17, 0.2],
                        rust.clone(),
                    ),
                    (
                        [0.022, 0.027, 0.16],
                        [side * 0.778, 0.23, 0.55],
                        metal.clone(),
                    ),
                    (
                        [0.025, 0.035, 1.45],
                        [side * 0.742, 0.40, 0.2],
                        trim.clone(),
                    ),
                    (
                        [0.05, 0.48, 0.035],
                        [side * 0.74, 0.64, 0.24],
                        paint.clone(),
                    ),
                    (
                        [0.16, 0.11, 0.08],
                        [side * 0.86, 0.51, -0.48],
                        metal.clone(),
                    ),
                ] {
                    body.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(Vec3::from_array(position))
                            .with_scale(Vec3::from_array(size)),
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
    let hub = meshes.add(Cylinder::new(0.17, 0.235));
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
