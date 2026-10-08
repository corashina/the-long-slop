use bevy::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameReady)]
    fn game_ready();
}

/// Start the single-threaded WebGL2 application on the page canvas.
#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.82)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Desert Drive".into(),
                canvas: Some("#game".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, announce_ready)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.63, 0.47, 0.29))),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.64, 0.20, 0.09))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            ..default()
        },
        Transform::from_xyz(6.0, 9.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
        Msaa::Off,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
    ));
}

fn announce_ready(mut frames: Local<u8>) {
    if *frames < 3 {
        *frames += 1;
        if *frames == 3 {
            game_ready();
        }
    }
}
