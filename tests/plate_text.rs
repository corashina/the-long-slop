use my_summer_slop::{
    plates::{PlateState, layout::layout_text},
    reference::ReferenceManifest,
};
fn record() -> my_summer_slop::reference::PlateRecord {
    ReferenceManifest::from_json(include_bytes!("common/reference.json"))
        .unwrap()
        .plates
}
#[test]
fn plate_generation_uses_source_ranges() {
    let r = record();
    for upper in [false, true] {
        let mut bounds = Vec::new();
        let s = PlateState::new(&r, &mut |a, b| {
            bounds.push((a, b));
            if upper { b - 1 } else { a }
        })
        .unwrap();
        assert_eq!(
            s.text(r.plates[1].item),
            Some(if upper { "ZZZ-999" } else { "AAA-000" })
        );
        assert_eq!(
            bounds,
            [(65, 91), (65, 91), (65, 91), (48, 58), (48, 58), (48, 58)].repeat(2)
        );
    }
    assert!(PlateState::new(&r, &mut |_, b| b).is_err());
}
#[test]
fn plate_start_order_preserves_shared_text() {
    for reverse in [false, true] {
        let mut r = record();
        let front = r.plates[0].item;
        let rear = r.plates[1].item;
        if reverse {
            r.plates.reverse();
        }
        let mut count = 0;
        let mut s = PlateState::new(&r, &mut |a, _| {
            let group = count / 6;
            count += 1;
            a + group
        })
        .unwrap();
        assert_eq!(count, if reverse { 6 } else { 12 });
        assert_eq!(s.text(front), s.text(rear));
        assert_eq!(
            s.text(front),
            Some(if reverse { "AAA-000" } else { "BBB-111" })
        );
        let before = s.revision(front).unwrap();
        s.set(rear, "ABC-123").unwrap();
        assert_eq!(s.text(front), Some("ABC-123"));
        assert!(s.revision(front).unwrap() > before);
        let revision = s.revision(front);
        s.set(rear, "ABC-123").unwrap();
        assert_eq!(s.revision(front), revision);
    }
}
#[test]
fn plate_layout_uses_source_metrics() {
    let mut r = record();
    let l = &mut r.plates[0].layout;
    l.font_size = 0.8;
    l.orthographic = false;
    r.font.face.insert("PointSize".into(), 10.0);
    r.font.face.insert("Scale".into(), 1.0);
    r.font.bold_style = 0.75;
    r.font.bold_spacing = 7.0;
    r.font.normal_spacing = 0.0;
    let a = r
        .font
        .characters
        .iter()
        .find(|c| c.unicode == 65)
        .unwrap()
        .glyph;
    for c in &mut r.font.characters {
        c.scale = 1.0;
    }
    let g = r.font.glyphs.iter_mut().find(|g| g.index == a).unwrap();
    g.metrics = [6.0, 8.0, 1.0, 8.0, 10.0];
    g.scale = 1.0;
    g.rect = [20, 30, 6, 8];
    r.font.pair_adjustments = vec![my_summer_slop::reference::PlatePairAdjustment {
        first: my_summer_slop::reference::PlateGlyphAdjustment {
            glyph: a,
            values: [0.0, 0.0, -2.0, 0.0],
        },
        second: my_summer_slop::reference::PlateGlyphAdjustment {
            glyph: a,
            values: [0.0; 4],
        },
        flags: 0,
    }];
    r.font.face.insert("AscentLine".into(), 8.0);
    r.font.face.insert("DescentLine".into(), -2.0);
    r.font.face.insert("Baseline".into(), 1.0);
    let m = layout_text(&r.font, l, &r.material, "AA").unwrap();
    assert_eq!(m.vertices.len(), 8);
    assert_eq!(m.indices.len(), 12);
    assert!((m.vertices[1].position[0] - m.vertices[0].position[0] - 0.1526).abs() < 0.00001);
    assert!((m.vertices[4].position[0] - m.vertices[0].position[0] - 0.0696).abs() < 0.00001);
    assert!((m.vertices[0].position[0] - (-0.1191)).abs() < 0.00001);
    assert!((m.vertices[0].position[1] - (-0.0548)).abs() < 0.00001);
    assert!((m.vertices[0].uv0[0] - (20.0 - 6.5375) / 1024.0).abs() < 0.000001);
    assert!((m.vertices[2].uv0[1] - (38.0 + 6.5375) / 1024.0).abs() < 0.000001);
    assert_eq!(m.vertices[0].position[2], 0.0);
    assert_eq!(m.vertices[0].normal, [0.0, 0.0, 1.0]);
    assert_eq!(m.indices[..6], [0, 1, 2, 0, 2, 3]);
    let mut shifted = l.clone();
    shifted.pivot = [0.0, 0.0];
    shifted.bounds = [0.8, 0.2];
    shifted.margin = [0.02, 0.03, 0.04, 0.05];
    shifted.transform.translation = [10.0; 3];
    let other = layout_text(&r.font, &shifted, &r.material, "AA").unwrap();
    for (a, b) in m.vertices.iter().zip(&other.vertices) {
        assert!((b.position[0] - a.position[0] - 0.39).abs() < 0.00001);
        assert!((b.position[1] - a.position[1] - 0.11).abs() < 0.00001);
        assert_eq!(b.position[2], 0.0);
    }
    assert!(
        m.vertices
            .iter()
            .all(|v| v.position.iter().all(|x| x.is_finite()) && v.uv1[1] < 0.0)
    );
    assert!(layout_text(&r.font, l, &r.material, "☃").is_err());
    l.font_size = f32::NAN;
    assert!(layout_text(&r.font, l, &r.material, "A").is_err());
}
#[test]
#[ignore = "Requires installed source export"]
fn installed_generated_glyphs_resolve() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap()
            .plates;
    let mut draw = my_summer_slop::plates::StartupDraw::new(my_summer_slop::world::DEFAULT_SEED);
    let state = PlateState::new(&r, &mut |a, b| draw.draw(a, b)).unwrap();
    for p in &r.plates {
        let mesh =
            layout_text(&r.font, &p.layout, &r.material, state.text(p.item).unwrap()).unwrap();
        assert_eq!(mesh.vertices.len(), 28);
        assert!(mesh.vertices.iter().all(|v| v.position[2] == 0.0));
    }
}
#[test]
fn plate_text_uniforms_keep_source_surface_values() {
    let r = record();
    let p = my_summer_slop::plates::layout::sdf_parameters(&r.material, &r.plates[0].layout);
    assert_eq!(
        p.face_color,
        [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 1.0]
    );
    assert_eq!(p.face_dilate, 0.1);
    assert_eq!(p.outline_width, 0.1);
    assert_eq!(p.outline_softness, 0.2);
    assert_eq!(p.gradient_scale, 10.0);
    assert_eq!(p.weight, 0.75);
    assert_eq!(p.scale_ratio_a, 0.9);
    assert_eq!(p.perspective_filter, 0.875);
}
