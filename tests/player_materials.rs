use my_summer_slop::reference::{
    AssetId, AssetRecord, AssetUsage, MaterialRecord, RenderingRecord,
};

fn player_fixture() -> (RenderingRecord, Vec<AssetRecord>) {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/player_materials.json")).unwrap();
    let materials: Vec<MaterialRecord> =
        serde_json::from_value(fixture["materials"].clone()).unwrap();
    let source: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    let mut asset: AssetRecord = serde_json::from_value(source["assets"][0].clone()).unwrap();
    asset.id = AssetId(21);
    asset.usage = serde_json::from_str("\"player_prototype\"").unwrap();
    asset.material_count = 4;
    asset.dependencies = serde_json::from_value(fixture["dependencies"].clone()).unwrap();
    (RenderingRecord { materials }, vec![asset])
}

#[test]
fn checked_player_material_contract_is_inactive() {
    let (rendering, assets) = player_fixture();
    rendering.validate(&assets, AssetId(999)).unwrap();
    for record in &rendering.materials {
        assert!(record.validate_active_use().is_err());
    }
    let manifest = my_summer_slop::reference::ReferenceManifest::from_json(include_bytes!(
        "common/reference.json"
    ))
    .unwrap();
    for record in &manifest.rendering.materials {
        let player = record.shader_family == my_summer_slop::reference::ShaderFamily::PlayerModel;
        assert_eq!(record.validate_active_use().is_err(), player);
    }
}

#[test]
fn rejects_incomplete_or_misclassified_player_contract() {
    let (original, assets) = player_fixture();
    for mutate in [
        |r: &mut MaterialRecord| {
            r.properties.floats.remove("_Dist");
        },
        |r: &mut MaterialRecord| {
            r.shader_family = my_summer_slop::reference::ShaderFamily::Standard;
        },
        |r: &mut MaterialRecord| {
            r.properties.textures.pop();
        },
        |r: &mut MaterialRecord| {
            r.properties.textures[1].source.path_id = 0;
        },
        |r: &mut MaterialRecord| {
            r.properties.textures[1].source_encoding =
                my_summer_slop::reference::TextureEncoding::Rgba;
        },
        |r: &mut MaterialRecord| {
            r.properties.floats.insert("_UnknownRequired".into(), 1.);
        },
        |r: &mut MaterialRecord| {
            r.state.alpha_mode = my_summer_slop::reference::AlphaMode::Opaque;
        },
    ] {
        let mut rendering = original.clone();
        mutate(&mut rendering.materials[0]);
        assert!(rendering.validate(&assets, AssetId(999)).is_err());
    }
    let mut active = assets;
    active[0].usage = AssetUsage::UniqueScene;
    assert!(original.validate(&active, AssetId(999)).is_err());
}

#[test]
fn rag_is_exclusive_to_the_player_contract() {
    let mut manifest = my_summer_slop::reference::ReferenceManifest::from_json(include_bytes!(
        "common/reference.json"
    ))
    .unwrap();
    manifest.rendering.materials[0].properties.textures[0].source_encoding =
        my_summer_slop::reference::TextureEncoding::Rag;
    assert!(manifest.validate().is_err());
    let count = manifest.expected_instances();
    let (_, assets) = player_fixture();
    manifest.assets.extend(assets);
    assert_eq!(manifest.expected_instances(), count);
}

#[test]
fn selected_player_default_variants_reject_missing_or_foreign_inputs() {
    use my_summer_slop::reference::{ReferenceManifest, TextureDefault};
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for (pid, change) in [(63, 0), (64, 1), (69, 2), (67, 3), (70, 4)] {
        let mut m = original.clone();
        let record = m
            .rendering
            .materials
            .iter_mut()
            .find(|r| r.source.file == "sharedassets0.assets" && r.source.path_id == pid)
            .unwrap();
        match change {
            0 => {
                record.properties.floats.remove("_Dist");
            }
            1 => {
                record
                    .properties
                    .colors
                    .insert("_EmissionColor".into(), [0.; 4]);
            }
            2 => {
                record.properties.floats.remove("_Red");
            }
            3 => {
                record
                    .properties
                    .defaults
                    .insert("_BumpMap".into(), TextureDefault::White);
            }
            _ => {
                record
                    .properties
                    .textures
                    .iter_mut()
                    .find(|t| t.slot == "_BumpMap")
                    .unwrap()
                    .source
                    .path_id = 168;
            }
        }
        assert!(m.validate().is_err(), "material {pid}");
    }
}
