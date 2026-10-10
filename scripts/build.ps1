$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$image = 'my-summer-slop-toolchain'
try {
    & docker build --tag $image --file "$root/tools/Dockerfile" $root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    & docker run --rm --mount "type=bind,source=$root,target=/workspace" `
        --mount 'type=volume,source=tld-wasm-cargo,target=/cargo' `
        --mount 'type=volume,source=tld-wasm-target,target=/target' $image `
        sh -c 'cargo build --locked --release --target wasm32-unknown-unknown --lib && mkdir -p web/pkg && wasm-bindgen --target web --out-dir web/pkg --out-name my_summer_slop /target/wasm32-unknown-unknown/release/my_summer_slop.wasm'
    exit $LASTEXITCODE
} catch {
    Write-Error $_
    exit 1
}
