#[cfg(target_arch = "wasm32")]
fn main() {
    desert_drive::app::run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Build the browser app with scripts/build.ps1.");
}
