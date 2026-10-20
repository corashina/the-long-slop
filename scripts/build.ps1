$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"
$cargoTarget = Get-CargoTargetDirectory $root
$image = 'my-summer-slop-toolchain'
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' `
        --env "CARGO_TARGET_DIR=$cargoTarget" $image `
        sh -c 'cargo build --locked --release --target wasm32-unknown-unknown --lib -p my-summer-slop -p terrain-worker && cargo rustc --locked --release --target wasm32-unknown-unknown --lib -p road-worker -- -C link-arg=--initial-memory=2097152 && cargo rustc --locked --release --target wasm32-unknown-unknown --lib -p grass-worker -- -C link-arg=--initial-memory=2097152 && mkdir -p web/pkg && wasm-bindgen --target web --out-dir web/pkg --out-name my_summer_slop "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/my_summer_slop.wasm" && wasm-bindgen --target web --out-dir web/pkg --out-name terrain_worker "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/terrain_worker.wasm" && wasm-bindgen --target web --out-dir web/pkg --out-name road_worker "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/road_worker.wasm" && wasm-bindgen --target web --out-dir web/pkg --out-name grass_worker "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/grass_worker.wasm"'
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & node "$PSScriptRoot/prepare-http-compression.mjs"
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
