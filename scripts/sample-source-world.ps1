param(
    [Parameter(Mandatory=$true)][string]$ReferenceRoot,
    [string]$OutputRoot='.cache/world-source',
    [switch]$Verify
)
$ErrorActionPreference='Stop'
$repoRoot=Split-Path $PSScriptRoot -Parent
$reference=(Resolve-Path -LiteralPath $ReferenceRoot).Path
$referencePython=Join-Path $reference 'tools/reference/.venv/Scripts/python.exe'
if(-not(Test-Path -LiteralPath $referencePython)){throw 'Accepted reference Python environment is missing'}
Push-Location $repoRoot
try {
    $arguments=@('-B','-m','tools.reference.world_probe','--reference-root',$reference,'--output-root',$OutputRoot)
    if($Verify){$arguments+='--verify'}
    & $referencePython @arguments
    if($LASTEXITCODE -ne 0){throw 'Source world capture or validation failed'}
} finally {Pop-Location}
