$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$image = 'the-long-drive-wasm-toolchain'
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' $image `
        sh -c 'cargo fmt --all -- --check && cargo test --locked && cargo build --locked --release --target wasm32-unknown-unknown --lib'
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
