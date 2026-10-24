param([string]$Commit = 'HEAD')
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$root = Split-Path $PSScriptRoot -Parent
. "$PSScriptRoot/cargo-target.ps1"

function Invoke-Checked([string]$File, [string[]]$Arguments) {
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File failed with exit code $LASTEXITCODE" }
}

$source = & git -C $root rev-parse --verify --end-of-options "$Commit^{commit}"
if ($LASTEXITCODE -ne 0) { throw 'The source commit is unavailable.' }
$source = $source.Trim()
$output = Join-Path $root ".cache/reflection-comparison/$($source.Substring(0,12))-$([Guid]::NewGuid().ToString('N').Substring(0,8))"
New-Item -ItemType Directory -Path $output | Out-Null
$archive = Join-Path $output 'source.tar'
Invoke-Checked -File git -Arguments @('-C', $root, 'archive', '--format=tar', "--output=$archive", $source)
$image = 'the-long-slop-toolchain'
$imageId = & docker image inspect --format '{{.Id}}' $image
if ($LASTEXITCODE -ne 0) { throw 'Build tools/Dockerfile before creating the comparison.' }
$manifest = Join-Path $root 'web/assets/reference/manifest.json'
$receipt = [ordered]@{
    source_commit = $source
    source_build = 11140890
    manifest_sha256 = (Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash
    cargo_lock_sha256 = $null
    toolchain_image = $imageId.Trim()
    profile = 'locked release; opt-level=3; thin LTO; codegen-units=1; panic=abort; strip=true'
    baseline_change = 'Prepare all six valid probe faces for every job, including mip jobs.'
    counters = 'Identical opt-in counters; disabled by default.'
    acceptance = 'Awaiting user tests. No benchmark samples collected.'
    packages = [ordered]@{}
}
$before = @'
pub fn prepare_probe_face(job: Option<ProbeJob>, face: usize, warmup: bool) -> bool {
    face < 6 && (warmup || job == Some(ProbeJob::Face(face as u8)))
}
'@
$after = @'
pub fn prepare_probe_face(_job: Option<ProbeJob>, face: usize, _warmup: bool) -> bool {
    face < 6
}
'@
$before = $before.Replace("`r`n", "`n")
$after = $after.Replace("`r`n", "`n")
$target = Get-CargoTargetDirectory $root
foreach ($variant in @('candidate', 'baseline')) {
    $directory = Join-Path $output $variant
    New-Item -ItemType Directory -Path $directory | Out-Null
    Invoke-Checked -File tar -Arguments @('-xf', $archive, '-C', $directory)
    foreach ($asset in @('reference', 'runtime', 'library')) {
        New-Item -ItemType Junction -Path (Join-Path $directory "web/assets/$asset") `
            -Target (Join-Path $root "web/assets/$asset") | Out-Null
    }
    if ($variant -eq 'baseline') {
        $path = Join-Path $directory 'src/reference/reflections.rs'
        $text = [IO.File]::ReadAllText($path).Replace("`r`n", "`n")
        $first = $text.IndexOf($before, [StringComparison]::Ordinal)
        if ($first -lt 0 -or $text.IndexOf($before, $first + $before.Length, [StringComparison]::Ordinal) -ge 0) {
            throw 'The face selector changed. Review the baseline patch before building.'
        }
        [IO.File]::WriteAllText($path, $text.Replace($before, $after), [Text.UTF8Encoding]::new($false))
    }
    Write-Output "Building $variant from $source"
    $arguments = @('run', '--rm', '--mount', "type=bind,source=$directory,target=/workspace",
        '--mount', 'type=volume,source=tld-wasm-cargo,target=/cargo',
        '--mount', 'type=volume,source=tld-wasm-target,target=/target',
        '--env', "CARGO_TARGET_DIR=$target", '--env', 'CARGO_BUILD_JOBS=2', $imageId.Trim(), 'sh', '-c',
        'cargo clean --package the-long-slop --release --target wasm32-unknown-unknown && cargo build --locked --release --target wasm32-unknown-unknown --lib && wasm-bindgen --target web --out-dir web/pkg --out-name the_long_slop "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/the_long_slop.wasm"')
    Invoke-Checked -File docker -Arguments $arguments
    $wasm = Join-Path $directory 'web/pkg/the_long_slop_bg.wasm'
    $receipt.packages[$variant] = [ordered]@{
        root = $directory
        wasm_sha256 = (Get-FileHash -LiteralPath $wasm -Algorithm SHA256).Hash
        javascript_sha256 = (Get-FileHash -LiteralPath (Join-Path $directory 'web/pkg/the_long_slop.js') -Algorithm SHA256).Hash
        bytes = (Get-Item -LiteralPath $wasm).Length
    }
    $receipt.cargo_lock_sha256 = (Get-FileHash -LiteralPath (Join-Path $directory 'Cargo.lock') -Algorithm SHA256).Hash
    $receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'comparison.json') -Encoding utf8
}
if ((Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash -ne $receipt.manifest_sha256) {
    throw 'The asset manifest changed during the build. These packages are not a matched pair.'
}
Write-Output "Comparison packages: $output"
Write-Output "Candidate: `$env:PORT='4198'; node '$output/candidate/scripts/serve.mjs'"
Write-Output "Baseline: `$env:PORT='4199'; node '$output/baseline/scripts/serve.mjs'"
