param([switch]$Profiling)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"
$cargoTarget = Get-CargoTargetDirectory $root
$image = 'my-summer-slop-toolchain'
# Profiling keeps function names so the browser profiler shows Rust frames.
$profile = if ($Profiling) { 'profiling' } else { 'release' }
$bindgen = if ($Profiling) { '--keep-debug' } else { '' }
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' `
        --env "CARGO_TARGET_DIR=$cargoTarget" --env "PROFILE=$profile" --env "BINDGEN_FLAGS=$bindgen" $image `
        sh -c 'cargo build --locked --profile "$PROFILE" --target wasm32-unknown-unknown --lib && mkdir -p web/pkg && wasm-bindgen $BINDGEN_FLAGS --target web --out-dir web/pkg --out-name my_summer_slop "$CARGO_TARGET_DIR/wasm32-unknown-unknown/$PROFILE/my_summer_slop.wasm"'
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
