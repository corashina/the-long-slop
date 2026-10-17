use my_summer_slop::reference::{
    ReferenceError, ReferenceManifest, TextureMip, fixed_reservation::*,
};
#[test]
fn sequential_png_reserves_base_reader_with_larger_later_input() {
    let mips = vec![
        TextureMip {
            file: "base".into(),
            width: 4096,
            height: 4096,
        },
        TextureMip {
            file: "tail".into(),
            width: 2048,
            height: 2048,
        },
    ];
    let base = 64 * 1024;
    let later = 4 * 1024 * 1024;
    let staging =
        png_chain_staging(&mips, |path| Ok(if path == "base" { base } else { later })).unwrap();
    assert!(staging >= base + png_staging(later, 2048).unwrap());
    let pixels = png_pixels(&mips).unwrap();
    let cpu = png_cpu(pixels, staging).unwrap();
    // Base RGB+RGBA conversion precedes final-chain allocation.
    assert!(cpu >= 4096 * 4096 * 7 + 2 * base);
    // Later conversion overlaps final output and retained encoded base.
    assert!(cpu >= pixels + 2048 * 2048 * 7 + base + png_staging(later, 2048).unwrap());
    assert!(
        png_chain_staging(&mips, |path| if path == "tail" {
            Err(ReferenceError("later receipt missing".into()))
        } else {
            Ok(base)
        })
        .is_err()
    );
}
#[test]
#[ignore = "requires the installed source export"]
fn installed_png_chains_fit_unchanged_single_job_caps() {
    let reference = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut count = 0;
    for texture in reference
        .rendering
        .materials
        .iter()
        .flat_map(|m| &m.properties.textures)
        .chain(&reference.plates.material.textures)
    {
        let staging = png_chain_staging(&texture.mips, |path| {
            Ok(reference.file_catalog.file(path)?.bytes as usize)
        })
        .unwrap();
        let cpu = png_cpu(png_pixels(&texture.mips).unwrap(), staging).unwrap();
        assert!(staging <= 32 * 1024 * 1024, "{}: {staging}", texture.file);
        assert!(cpu <= 256 * 1024 * 1024, "{}: {cpu}", texture.file);
        count += 1;
    }
    assert!(count > 0);
}
