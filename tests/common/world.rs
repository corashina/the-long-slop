#![allow(dead_code)]
use my_summer_slop::reference::WorldReference;
use std::path::{Path, PathBuf};

/// Synthetic geometry tests do not establish source generation parity.
pub fn synthetic_world() -> WorldReference {
    let manifest: serde_json::Value =
        serde_json::from_slice(include_bytes!("reference.json")).unwrap();
    serde_json::from_value(manifest["world"].clone()).unwrap()
}
pub fn source_samples_path() -> PathBuf {
    std::env::var_os("WORLD_SOURCE_SAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join(".cache/world-source/samples.json")
        })
}
/// Load the exact envelope accepted by the retained-source gate.
pub fn source_samples() -> serde_json::Value {
    let digest = std::process::Command::new("sha256sum")
        .arg(source_samples_path())
        .output()
        .expect("sha256sum for checked source envelope");
    assert!(digest.status.success());
    assert_eq!(
        String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap(),
        "0e7dc65a3e60bd1d5a747665f8ce85fa61ad5803cef4aa106460e292152158f2"
    );
    let data =
        std::fs::read(source_samples_path()).expect("Required checked source samples are missing");
    let samples: serde_json::Value = serde_json::from_slice(&data).unwrap();
    assert_eq!(
        samples["payload_sha256"],
        my_summer_slop::reference::WORLD_SAMPLE_SHA256
    );
    assert_eq!(samples["replay_sha256"], samples["payload_sha256"]);
    samples
}
