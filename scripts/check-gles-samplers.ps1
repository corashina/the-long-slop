$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"
$target = Get-CargoTargetDirectory $root
& docker build --tag my-summer-slop-toolchain --file "$root/tools/Dockerfile" $root
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& docker run --rm --mount "type=bind,source=$root,target=/workspace" `
    --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
    --mount 'type=volume,source=tld-wasm-target,target=/target' `
    --env "CARGO_TARGET_DIR=$target" my-summer-slop-toolchain `
    cargo test --locked --manifest-path vendor/wgpu-hal/Cargo.toml --no-default-features --features gles --lib sampler_cache_
exit $LASTEXITCODE
