param(
    [Parameter(Mandatory=$true)][ValidateSet('Setup','Run')][string]$Mode,
    [Parameter(Mandatory=$true)][string]$SourceRoot,
    [Parameter(Mandatory=$true)][string]$ReferenceRoot,
    [Parameter(Mandatory=$true)][string]$ScratchRoot,
    [string]$RequestPath,
    [string]$RunRoot,
    [string]$ProfileRoot
)
$ErrorActionPreference='Stop'
$projectRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$allowedRoot=[IO.Path]::GetFullPath((Join-Path $projectRoot '.cache/g11-driving'))+[IO.Path]::DirectorySeparatorChar
$scratch=[IO.Path]::GetFullPath($ScratchRoot)
if(-not $scratch.StartsWith($allowedRoot,[StringComparison]::OrdinalIgnoreCase)){throw 'Scratch must stay inside this G11 worktree'}
$source=[IO.Path]::GetFullPath($SourceRoot)
$reference=[IO.Path]::GetFullPath($ReferenceRoot)
if($scratch.StartsWith($source+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Scratch overlaps source archive'}
$game=Join-Path $scratch 'game'
$managed=Join-Path $game 'TheLongDrive_Data/Managed'
$sourceManaged=Join-Path $source 'TheLongDrive_Data/Managed'
function Hash($path){(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
if($Mode -eq 'Run'){
    $run=[IO.Path]::GetFullPath($RunRoot)
    $profile=[IO.Path]::GetFullPath($ProfileRoot)
    if(-not $run.StartsWith($allowedRoot,[StringComparison]::OrdinalIgnoreCase) -or $profile -ne (Join-Path $run 'profile')){throw 'Run profile must stay inside its fresh run directory'}
    if(Test-Path -LiteralPath (Join-Path $run 'runtime.json')){throw 'Refusing an existing native run'}
    if(Test-Path -LiteralPath $profile){throw 'Refusing an existing native profile'}
    New-Item -ItemType Directory -Path $profile | Out-Null
    $env:G11_RUN_ROOT=$run
    $env:G11_PROFILE_ROOT=$profile
    $arguments=@('-batchmode','-nographics','-logFile',"`"$run/unity.log`"",'-screen-width','320','-screen-height','240')
    $process=Start-Process -FilePath (Join-Path $game 'TheLongDrive.exe') -ArgumentList $arguments -WorkingDirectory $game -WindowStyle Hidden -PassThru
    @{pid=$process.Id;started_utc=$process.StartTime.ToUniversalTime().ToString('o');executable=(Join-Path $game 'TheLongDrive.exe')} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'process.json') -Encoding utf8
    if(-not $process.WaitForExit(180000)){
        # This is the exact process handle returned by Start-Process. Never search or kill by name or PID.
        $process.Kill();$process.WaitForExit()
        @{completed=$false;reason='180-second native run timeout';exit_code=$process.ExitCode} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'timeout.json') -Encoding utf8
        throw "Native run timed out: $run"
    }
    @{exit_code=$process.ExitCode;exited_utc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $run 'exit.json') -Encoding utf8
    if($process.ExitCode -ne 0){throw "Native run exited $($process.ExitCode): $run"}
    if(-not(Test-Path -LiteralPath (Join-Path $run 'runtime.json'))){throw "Native run has no samples: $run"}
    Write-Output "NATIVE_RUN_EXITED $run"
    exit 0
}
if(Test-Path -LiteralPath $scratch){throw 'Refusing an existing scratch player'}
New-Item -ItemType Directory -Path $scratch | Out-Null
Copy-Item -LiteralPath $source -Destination $game -Recurse
# Copying the entire verified archive avoids selective source assumptions.
$archive=Get-Content -LiteralPath (Join-Path $reference '.cache/reference-library/source-manifest.json') -Raw | ConvertFrom-Json
foreach($file in $archive.files){
    $path=Join-Path $game $file.file
    if(-not(Test-Path -LiteralPath $path) -or (Get-Item -LiteralPath $path).Length -ne $file.bytes -or (Hash $path) -ne $file.sha256){throw "Scratch archive hash differs: $($file.file)"}
}
Add-Type -Path "$PSHOME/Microsoft.CodeAnalysis.dll"
Add-Type -Path "$PSHOME/Microsoft.CodeAnalysis.CSharp.dll"
$references=[Collections.Generic.List[Microsoft.CodeAnalysis.MetadataReference]]::new()
Get-ChildItem -LiteralPath $sourceManaged -Filter '*.dll' | ForEach-Object {$references.Add([Microsoft.CodeAnalysis.MetadataReference]::CreateFromFile($_.FullName))}
$trees=[Collections.Generic.List[Microsoft.CodeAnalysis.SyntaxTree]]::new()
$trees.Add([Microsoft.CodeAnalysis.CSharp.CSharpSyntaxTree]::ParseText([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'driving_capture.cs'))))
$trees.Add([Microsoft.CodeAnalysis.CSharp.CSharpSyntaxTree]::ParseText([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'driving_geometry.cs'))))
$trees.Add([Microsoft.CodeAnalysis.CSharp.CSharpSyntaxTree]::ParseText([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'driving_driver.cs'))))
$options=[Microsoft.CodeAnalysis.CSharp.CSharpCompilationOptions]::new([Microsoft.CodeAnalysis.OutputKind]::DynamicallyLinkedLibrary)
$options=$options.WithDeterministic($true).WithOptimizationLevel([Microsoft.CodeAnalysis.OptimizationLevel]::Release)
$compilation=[Microsoft.CodeAnalysis.CSharp.CSharpCompilation]::Create('G11Capture',$trees,$references,$options)
$observerPath=Join-Path $managed 'G11Capture.dll'
$stream=[IO.File]::Create($observerPath)
try{
    $result=$compilation.Emit($stream)
    $diagnostics=@($result.Diagnostics | ForEach-Object {$_.ToString()});$diagnostics | Set-Content -LiteralPath (Join-Path $scratch 'compiler-diagnostics.txt') -Encoding utf8;$diagnostics | Where-Object {$_ -match 'error CS'} | Write-Output
    if(-not $result.Success){throw 'Original Unity observer compilation failed'}
}finally{$stream.Dispose()}
Write-Output 'OBSERVER_COMPILED against checked shipped assemblies'
Add-Type -Path (Join-Path $sourceManaged 'Unity.Burst.Cecil.dll')
$hook=[Mono.Cecil.AssemblyDefinition]::ReadAssembly($observerPath)
$main=[Mono.Cecil.AssemblyDefinition]::ReadAssembly((Join-Path $sourceManaged 'Assembly-CSharp.dll'))
$core=[Mono.Cecil.AssemblyDefinition]::ReadAssembly((Join-Path $sourceManaged 'UnityEngine.CoreModule.dll'))
$changes=[Collections.Generic.List[string]]::new()
$allowed=[Collections.Generic.HashSet[string]]::new()
$longBranches=@{}
[Mono.Cecil.Cil.OpCodes].GetFields([Reflection.BindingFlags]'Public,Static') | ForEach-Object {$opcode=$_.GetValue($null);$longBranches[$opcode.Name]=$opcode}
function Method($type,$name){
    $matches=@($main.MainModule.GetType($type).Methods | Where-Object Name -ceq $name)
    if($matches.Count -ne 1){throw "Expected one checked method: $type.$name"}
    $matches[0]
}
function HookMethod($name){
    $matches=@($hook.MainModule.GetType('G11Capture').Methods | Where-Object Name -ceq $name)
    if($matches.Count -ne 1){throw "Expected one observer method: $name"}
    $main.MainModule.ImportReference($matches[0])
}
function Identity($method){$method.FullName+"#generic-arity="+$method.GenericParameters.Count}
function Mark($method){[void]$allowed.Add((Identity $method))}
function Expand($method){
    foreach($instruction in $method.Body.Instructions){
        if($instruction.OpCode.OperandType -eq [Mono.Cecil.Cil.OperandType]::ShortInlineBrTarget){
            $instruction.OpCode=$longBranches[$instruction.OpCode.Name.Substring(0,$instruction.OpCode.Name.Length-2)]
        }
    }
}
function Observe($type,$name,$phase,$entry){
    $method=Method $type $name
    if($method.ReturnType.FullName -ne 'System.Void' -or -not $method.HasBody){throw "Unsupported observed signature: $($method.FullName)"}
    Mark $method;Expand $method
    $il=$method.Body.GetILProcessor();$call=HookMethod 'Observe'
    if($entry){$points=@($method.Body.Instructions[0])}else{$points=@($method.Body.Instructions | Where-Object {$_.OpCode.Code -eq 'Ret'})}
    foreach($point in $points){
        $load=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldarg_0)
        $label=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldstr,$phase)
        $invoke=$il.Create([Mono.Cecil.Cil.OpCodes]::Call,$call)
        if($entry){$il.InsertBefore($point,$load);$il.InsertBefore($point,$label);$il.InsertBefore($point,$invoke)}
        else{
            $point.OpCode=[Mono.Cecil.Cil.OpCodes]::Nop
            $il.InsertAfter($point,$load);$il.InsertAfter($load,$label);$il.InsertAfter($label,$invoke)
            $il.InsertAfter($invoke,$il.Create([Mono.Cecil.Cil.OpCodes]::Ret))
        }
    }
    $changes.Add("Observe $type.$name $phase; original instructions and branch targets retained")
}
function AllMethods($type){$type.Methods;foreach($nested in $type.NestedTypes){AllMethods $nested}}
# Retain source component intent while the observer holds every native source wheel.
# GameObject.SetActive calls remain original, including carscript.Lock and enableObjs.
foreach($type in $main.MainModule.Types){foreach($method in (AllMethods $type)){
    if(-not $method.HasBody){continue}
    foreach($instruction in @($method.Body.Instructions)){
        $target=$instruction.Operand
        if($target -is [Mono.Cecil.MethodReference] -and $target.Name -ceq 'SetActive' -and $target.DeclaringType.FullName -ceq 'UnityEngine.GameObject'){
            Mark $method;Expand $method
            $instruction.OpCode=[Mono.Cecil.Cil.OpCodes]::Ldstr;$instruction.Operand=(Identity $method)
            $method.Body.GetILProcessor().InsertAfter($instruction,$method.Body.GetILProcessor().Create([Mono.Cecil.Cil.OpCodes]::Call,(HookMethod 'SetSourceActive')))
            $changes.Add("Observe original GameObject.SetActive input in $(Identity $method); original activation retained")
        }
        elseif($target -is [Mono.Cecil.MethodReference] -and $target.Name -ceq 'set_enabled' -and $target.DeclaringType.FullName -ceq 'UnityEngine.Collider'){
            Mark $method;Expand $method
            $instruction.OpCode=[Mono.Cecil.Cil.OpCodes]::Call;$instruction.Operand=HookMethod 'SetColliderEnabled'
            $changes.Add("Retain original Collider.enabled input in $(Identity $method); temporary native wheel hold only")
        }
    }
}}
$lock=Method 'mountStuff' 'Lock';Mark $lock;Expand $lock
$il=$lock.Body.GetILProcessor();$call=HookMethod 'ObserveLock'
foreach($entry in @($true,$false)){
    if($entry){$points=@($lock.Body.Instructions[0]);$phase='storage.lock.entry'}else{$points=@($lock.Body.Instructions | Where-Object {$_.OpCode.Code -eq 'Ret'});$phase='storage.lock.exit'}
    foreach($point in $points){
        $load=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldarg_0);$cargo=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldarg_1)
        $label=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldstr,$phase);$invoke=$il.Create([Mono.Cecil.Cil.OpCodes]::Call,$call)
        if($entry){$il.InsertBefore($point,$load);$il.InsertBefore($point,$cargo);$il.InsertBefore($point,$label);$il.InsertBefore($point,$invoke)}
        else{$point.OpCode=[Mono.Cecil.Cil.OpCodes]::Nop;$il.InsertAfter($point,$load);$il.InsertAfter($load,$cargo);$il.InsertAfter($cargo,$label);$il.InsertAfter($label,$invoke);$il.InsertAfter($invoke,$il.Create([Mono.Cecil.Cil.OpCodes]::Ret))}
    }
}
$changes.Add('Observe original mountStuff.Lock entry/exit with original cargo argument; retain deferred Destroy and source scheduling')

Observe 'mainmenuscript' 'Start' 'menu.exit' $false
Observe 'carscript' 'Start' 'car.ready' $false
Observe 'partslotscript' 'SpawnPart' 'slot.ready' $false
# Preserve the original iterator result. Observe only its actual terminal false return.
$storageIterator=@($main.MainModule.GetType('mountStuff').NestedTypes | Where-Object Name -Match '^<StartFreeze>d__')
if($storageIterator.Count -ne 1){throw 'Expected one checked source storage StartFreeze iterator'}
$storageOwner=@($storageIterator[0].Fields | Where-Object {$_.Name -ceq '<>4__this' -and $_.FieldType.FullName -ceq 'mountStuff'})
$storageMove=@($storageIterator[0].Methods | Where-Object Name -ceq 'MoveNext')
if($storageOwner.Count -ne 1 -or $storageMove.Count -ne 1 -or $storageMove[0].ReturnType.FullName -cne 'System.Boolean'){throw 'Unsupported source storage iterator identity'}
$method=$storageMove[0];Mark $method;Expand $method
$il=$method.Body.GetILProcessor();$call=HookMethod 'Observe'
foreach($point in @($method.Body.Instructions | Where-Object {$_.OpCode.Code -eq 'Ret'})){
    $point.OpCode=[Mono.Cecil.Cil.OpCodes]::Dup
    $ret=$il.Create([Mono.Cecil.Cil.OpCodes]::Ret)
    $branch=$il.Create([Mono.Cecil.Cil.OpCodes]::Brtrue,$ret)
    $load=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldarg_0)
    $owner=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldfld,$storageOwner[0])
    $label=$il.Create([Mono.Cecil.Cil.OpCodes]::Ldstr,'storage.ready')
    $invoke=$il.Create([Mono.Cecil.Cil.OpCodes]::Call,$call)
    $il.InsertAfter($point,$branch);$il.InsertAfter($branch,$load);$il.InsertAfter($load,$owner)
    $il.InsertAfter($owner,$label);$il.InsertAfter($label,$invoke);$il.InsertAfter($invoke,$ret)
}
$changes.Add('Observe mountStuff.StartFreeze actual terminal MoveNext false; original yields, QuickFreeze calls, return value, and scheduling retained')

Observe 'fpscontroller' 'GetIn' 'driver.get-in.exit' $false
Observe 'fpscontroller' 'Update' 'driver.update.entry' $true
Observe 'fpscontroller' 'Update' 'driver.update.exit' $false
Observe 'mainscript' 'FStart' 'world.entry' $true
Observe 'mainscript' 'FStart' 'world.exit' $false
Observe 'mainscript' 'SettingsRefreshed' 'settings.entry' $true
foreach($spec in @(@('inputscript','DriveStuff','input'),@('carscript','Update','car'),@('enginescript','Update','engine'),@('carscript','AutoShift','autoshift'),@('carscript','Stuff','rpm'),@('carscript','GetPlayerInput','control'))){
    Observe $spec[0] $spec[1] ($spec[2]+'.entry') $true
    Observe $spec[0] $spec[1] ($spec[2]+'.exit') $false
}
foreach($name in @('DriveStuff','DriveStuff2')){
    $method=Method 'inputscript' $name
    Mark $method;Expand $method
    $inputCalls=0;$axisCalls=0
    foreach($instruction in $method.Body.Instructions){
        if($instruction.Operand -isnot [Mono.Cecil.MethodReference]){continue}
        $target=$instruction.Operand
        if($target.DeclaringType.FullName -eq 'inputscript' -and $target.Name -in @('GetAxis','GetKey','GetKeyDown','GetKeyUp')){
            $replacement=@{GetAxis='Axis';GetKey='Held';GetKeyDown='Down';GetKeyUp='Up'}[$target.Name]
            $instruction.OpCode=[Mono.Cecil.Cil.OpCodes]::Call;$instruction.Operand=HookMethod $replacement;$inputCalls++
        }
        if($target.DeclaringType.FullName -eq 'UnityEngine.Input' -and $target.Name -eq 'GetAxis'){
            $instruction.OpCode=[Mono.Cecil.Cil.OpCodes]::Call;$instruction.Operand=HookMethod 'AnalogAxis';$axisCalls++
        }
    }
    if($inputCalls -eq 0){throw "Checked source input boundary missing: $name"}
    $changes.Add("$name source input calls -> declared controls; smoothing instructions unchanged; digital=$inputCalls analog=$axisCalls")
}
function NewBody($method){
    $method.ImplAttributes=[Mono.Cecil.MethodImplAttributes]::IL;$method.IsPInvokeImpl=$false
    $method.Body=[Mono.Cecil.Cil.MethodBody]::new($method);$method.Body.GetILProcessor()
}
$path=Method 'pathscript' 'path';Mark $path
$il=NewBody $path;$il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Call,(HookMethod 'ProfilePath')));$il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Ret))
$changes.Add('pathscript.path -> explicit scratch profile; normal saves remain outside scratch')
$steam=Method 'SteamManager' 'Awake';Mark $steam
$il=NewBody $steam;$il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Ret))
$changes.Add('SteamManager.Awake -> return; suppress Steam initialization only in scratch')
foreach($method in $core.MainModule.GetType('UnityEngine.PlayerPrefs').Methods){
    if($method.Name -notin @('SetInt','SetFloat','SetString','GetInt','GetFloat','GetString','HasKey','DeleteKey','DeleteAll','Save')){continue}
    Mark $method
    $signature=($method.Parameters | ForEach-Object {$_.ParameterType.FullName}) -join ','
    $matches=@($hook.MainModule.GetType('G11Prefs').Methods | Where-Object {$_.Name -eq $method.Name -and (($_.Parameters | ForEach-Object {$_.ParameterType.FullName}) -join ',') -eq $signature})
    if($matches.Count -ne 1){throw "Missing exact isolated PlayerPrefs adapter: $($method.FullName)"}
    $il=NewBody $method
    foreach($parameter in $method.Parameters){$il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Ldarg,$parameter))}
    $il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Call,$core.MainModule.ImportReference($matches[0])))
    $il.Append($il.Create([Mono.Cecil.Cil.OpCodes]::Ret))
    $changes.Add("PlayerPrefs.$($method.Name)($signature) -> isolated in-memory profile")
}
$main.Write((Join-Path $managed 'Assembly-CSharp.dll'))
$core.Write((Join-Path $managed 'UnityEngine.CoreModule.dll'))
function Fingerprint($method){
    if(-not $method.HasBody){return [string]$method.ImplAttributes}
    ($method.Body.Instructions | ForEach-Object {$_.ToString()}) -join "`n"
}
$audited=0
foreach($name in @('Assembly-CSharp.dll','UnityEngine.CoreModule.dll')){
    $before=[Mono.Cecil.AssemblyDefinition]::ReadAssembly((Join-Path $sourceManaged $name))
    $after=[Mono.Cecil.AssemblyDefinition]::ReadAssembly((Join-Path $managed $name))
    $original=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal);foreach($type in $before.MainModule.Types){foreach($method in (AllMethods $type)){$original[(Identity $method)]=Fingerprint $method}}
    foreach($type in $after.MainModule.Types){foreach($method in (AllMethods $type)){
        if(-not $allowed.Contains((Identity $method)) -and $original[(Identity $method)] -cne (Fingerprint $method)){throw "Unauthorized source method change: $($method.FullName)"}
        $audited++
    }}
}
$scripting=Join-Path $game 'TheLongDrive_Data/ScriptingAssemblies.json'
$assemblies=Get-Content -LiteralPath $scripting -Raw | ConvertFrom-Json
$assemblies.names+='G11Capture.dll';$assemblies.types+=16
$assemblies | ConvertTo-Json -Compress -Depth 4 | Set-Content -LiteralPath $scripting -Encoding utf8
$files=@('TheLongDrive_Data/Managed/Assembly-CSharp.dll','TheLongDrive_Data/Managed/UnityEngine.CoreModule.dll','TheLongDrive_Data/ScriptingAssemblies.json')
$patches=@($files | ForEach-Object {@{file=$_;original_sha256=(Hash (Join-Path $source $_));patched_sha256=(Hash (Join-Path $game $_))}})
$request=Get-Content -LiteralPath $RequestPath -Raw | ConvertFrom-Json
@{schema_version=1;observer_revision=$request.observer_revision;observer_assembly_sha256=(Hash $observerPath);
    changed_methods=@($allowed);changes=@($changes);patched_files=$patches;audited_methods=$audited;
    archive_file_count=$archive.files.Count;source_physics_module_sha256=(Hash (Join-Path $managed 'UnityEngine.PhysicsModule.dll'));
    native_engine_sha256=(Hash (Join-Path $game 'UnityPlayer.dll'))} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $scratch 'patch-receipt.json') -Encoding utf8
Write-Output "SCRATCH_VERIFIED source_files=$($archive.files.Count) audited_methods=$audited"
