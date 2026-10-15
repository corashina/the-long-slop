param([string]$Test)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"
$cargoTarget = Get-CargoTargetDirectory $root
$image = 'my-summer-slop-toolchain'
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    if (-not $Test) {
        & node --test "$root/tests/web/audio.test.mjs" "$root/tests/web/menu.test.mjs" "$root/tests/web/loading.test.mjs" "$root/tests/web/input.test.mjs" "$root/tests/web/save-storage.test.mjs" "$root/tests/web/save-session.test.mjs" "$root/tests/web/save-bridge.test.mjs"
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    $checks = 'cargo fmt --all -- --check && cargo test --locked && cargo build --locked --release --target wasm32-unknown-unknown --lib'
    if ($Test) {
        if ($Test -notmatch '^[a-z_]+$') { throw 'Use a test target name.' }
        $checks = "cargo test --locked --test $Test"
    }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' `
        --env "CARGO_TARGET_DIR=$cargoTarget" $image `
        sh -c $checks
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
