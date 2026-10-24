use super::*;
use crate::reference::{AudioReference, RadioAudioReference};
use crate::simulation::radio::RadioControl;
use bevy::prelude::{Quat, Transform, Vec3};

#[path = "../../../presentation/radio/needle.rs"]
mod needle;

#[test]
fn g10_radio_visual_contract() {
    let (reference, _) = load_garage();
    let audio = AudioReference::from_value(&reference.audio, &reference).unwrap();
    let value = serde_json::to_value(&audio.radio).unwrap();
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("visuals");
    assert!(serde_json::from_value::<RadioAudioReference>(missing).is_err());
    for pointer in [
        "/visuals/screen/node",
        "/visuals/indicator/node",
        "/visuals/needle",
        "/visuals/start",
        "/visuals/end",
        "/visuals/screen/on/asset",
        "/visuals/indicator/off/gltf_material",
    ] {
        let mut raw = reference.audio.clone();
        *raw["radio"].pointer_mut(pointer).unwrap() = serde_json::json!(u32::MAX);
        assert!(
            AudioReference::from_value(&raw, &reference).is_err(),
            "{pointer}"
        );
    }
    for switch in ["screen", "indicator"] {
        let mut raw = reference.audio.clone();
        raw["radio"]["visuals"][switch] = serde_json::json!({"kind":"absent"});
        assert!(AudioReference::from_value(&raw, &reference).is_err());
    }
}

#[test]
fn g10_radio_power_and_tuning() {
    let (reference, mut run) = load_garage();
    let audio = run.audio.as_mut().unwrap();
    let r = &audio.reference.radio;
    for electronics in [
        r.electronics_cutoff - 0.001,
        r.electronics_cutoff,
        r.electronics_cutoff + 0.001,
    ] {
        for on in [false, true] {
            for volume in [0., 0.75] {
                for am in [false, true] {
                    audio.radio.on = on;
                    audio.radio.volume = volume;
                    audio.radio.am = am;
                    for (tuning, key) in [
                        (0., if am { "AMmin" } else { "FMmin" }),
                        (1., if am { "AMmax" } else { "FMmax" }),
                    ] {
                        audio.radio.tuning = tuning;
                        let frame = audio.broadcasts.frame(
                            &audio.radio,
                            0.,
                            electronics,
                            r.electronics_cutoff,
                        );
                        assert_eq!(frame.power, on && electronics > r.electronics_cutoff);
                        assert!((frame.frequency - r.constants[key]).abs() < 0.001);
                        assert_eq!(audio.radio.tuning, tuning);
                    }
                }
            }
        }
    }
    audio.radio.on = false;
    audio.radio.tuning = 0.5;
    audio.radio.control(RadioControl::TuneUp);
    assert!(audio.radio.tuning > 0.5);
    assert!(
        !audio
            .broadcasts
            .frame(&audio.radio, 0., r.electronics_output, r.electronics_cutoff)
            .power
    );
    assert_eq!(reference.build_id, 11140890);
}

#[test]
fn g10_radio_needle_parent_space() {
    let parent = Transform {
        translation: Vec3::new(7., -2., 4.),
        rotation: Quat::from_rotation_y(0.8) * Quat::from_rotation_z(-0.3),
        scale: Vec3::new(2., 0.7, 3.),
    }
    .compute_affine();
    let start = Transform::from_xyz(-4., 3., 2.).compute_affine()
        * Transform::from_rotation(Quat::from_rotation_z(1.2)).compute_affine()
        * Transform::from_xyz(0.3, 0.5, -0.2).compute_affine();
    let end = Transform::from_xyz(8., -4., 1.).compute_affine()
        * Transform::from_scale(Vec3::new(0.5, 2., 1.)).compute_affine()
        * Transform::from_xyz(-1., 0.2, 0.5).compute_affine();
    let rotation = Quat::from_rotation_x(0.3);
    let scale = Vec3::new(0.3, 0.5, 0.8);
    let mut transform = Transform {
        rotation,
        scale,
        ..Transform::default()
    };
    for tuning in [-1_f32, 0., 0.5, 1., 2.] {
        needle::update_needle(&mut transform, start, end, parent, tuning);
        let world: Vec3 = start
            .translation
            .lerp(end.translation, tuning.clamp(0., 1.))
            .into();
        let expected = parent.inverse().transform_point3(world);
        assert!(transform.translation.abs_diff_eq(expected, 0.00001));
        assert_eq!(transform.rotation, rotation);
        assert_eq!(transform.scale, scale);
    }
}
