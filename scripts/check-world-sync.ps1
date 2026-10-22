# Targeted native tests and WASM compile checks for world sync.
[CmdletBinding()]
param([string]$SourceExport)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"
$cargoTarget = Get-CargoTargetDirectory $root
$image = 'my-summer-slop-toolchain'
$checks = @(
    'cargo fmt --all -- --check'
    'cargo test --locked --lib world_sync'
    'cargo test --locked --lib content_index::tests'
    'cargo test --locked --lib content_lamps::tests'
    'cargo test --locked --lib vehicle::query_bounds'
)
$mounts = @('--mount', "type=bind,source=$root,target=/workspace")
if ($SourceExport) {
    $export = (Resolve-Path -LiteralPath $SourceExport).Path
    if (-not (Test-Path -LiteralPath "$export/manifest.json" -PathType Leaf)) {
        throw 'Source export must contain manifest.json.'
    }
    $mounts += @('--mount', "type=bind,source=$export,target=/workspace/web/assets/reference,readonly")
    $checks += 'cargo test --locked --lib world_sync -- --ignored'
    $checks += 'cargo test --locked --lib content_index_matches_linear_lookups -- --ignored'
    $checks += 'cargo test --locked --lib g10_generated_endpoint_root'
}
$checks += @(
    'cargo check --locked --target wasm32-unknown-unknown --workspace --lib'
    'cargo check --locked --target wasm32-unknown-unknown --lib -p my-summer-slop --no-default-features --features webgpu'
)
& docker build --tag $image --file "$root/tools/Dockerfile" $root
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& docker run --rm @mounts `
    --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
    --mount 'type=volume,source=tld-wasm-target,target=/target' `
    --env "CARGO_TARGET_DIR=$cargoTarget" $image sh -c ($checks -join ' && ')
exit $LASTEXITCODE
