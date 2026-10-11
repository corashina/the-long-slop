param([string]$Test)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$image = 'my-summer-slop-toolchain'
$sha = [Security.Cryptography.SHA256]::Create()
try {
    $pathBytes = [Text.Encoding]::UTF8.GetBytes([IO.Path]::GetFullPath($root).ToLowerInvariant())
    $checkout = [BitConverter]::ToString($sha.ComputeHash($pathBytes)).Replace('-', '').Substring(0, 16).ToLowerInvariant()
} finally { $sha.Dispose() }
$target = "/target/$checkout"
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $checks = 'cargo fmt --all -- --check && cargo test --locked && cargo build --locked --release --target wasm32-unknown-unknown --lib'
    if ($Test) {
        if ($Test -notmatch '^[a-z_]+$') { throw 'Use a test target name.' }
        $checks = "cargo test --locked --test $Test"
    }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' `
        --env "CARGO_TARGET_DIR=$target" $image `
        sh -c $checks
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
