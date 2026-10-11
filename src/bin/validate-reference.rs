//! Validate runtime manifests with the same contract as the game.
use my_summer_slop::reference::ReferenceManifest;
use std::io::{self, Read};

fn validate() -> Result<(), Box<dyn std::error::Error>> {
    let mut content = Vec::new();
    io::stdin()
        .take(64 * 1024 * 1024 + 1)
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
