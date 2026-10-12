//! Validate runtime manifests with the same contract as the game.
use my_summer_slop::reference::{AudioReference, ReferenceManifest};
use std::io::{self, Read};

fn validate() -> Result<(), Box<dyn std::error::Error>> {
    let mut content = Vec::new();
    io::stdin()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut content)?;
    let game = ReferenceManifest::from_json(&content)?;
    AudioReference::from_value(&game.audio, &game)?;
    Ok(())
}

fn main() {
    if let Err(error) = validate() {
        eprintln!("Runtime manifest contract: {error}");
        std::process::exit(1);
    }
}
