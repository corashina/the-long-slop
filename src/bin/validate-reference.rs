//! Validate runtime manifests with the same contract as the game.
use my_summer_slop::reference::{MAX_MANIFEST_BYTES, ReferenceManifest};
use std::io::{self, Read};

fn validate() -> Result<(), Box<dyn std::error::Error>> {
    let mut content = Vec::new();
    io::stdin()
        .take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut content)?;
    ReferenceManifest::from_json(&content)?;
    Ok(())
}

fn main() {
    if let Err(error) = validate() {
        eprintln!("Runtime manifest contract: {error}");
        std::process::exit(1);
    }
}
