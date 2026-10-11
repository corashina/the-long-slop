param(
    [Parameter(Mandatory)][string]$AssemblyPath,
    [Parameter(Mandatory)][string]$OutputPath,
    [ValidateSet('Decompile','Index')][string]$Mode = 'Decompile',
    [string]$DecompilerPath = (Join-Path $PSScriptRoot '../../.cache/ilspy/package/lib/netstandard2.0/ICSharpCode.Decompiler.dll')
)
$ErrorActionPreference = 'Stop'
$OutputPath = [IO.Path]::GetFullPath($OutputPath)
$AssemblyPath = [IO.Path]::GetFullPath($AssemblyPath)
[IO.Directory]::CreateDirectory($OutputPath) | Out-Null
function Write-Json($Name, $Value) {
    $json = ConvertTo-Json -InputObject $Value -Depth 100 -Compress
    [IO.File]::WriteAllText((Join-Path $OutputPath $Name), $json + "`n", [Text.UTF8Encoding]::new($false))
}
function Hash-File($Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
# Compile the metadata tool only. Game assemblies enter PEReader and ILSpy as data.
Add-Type -TypeDefinition @"
using System;
using System.IO;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
public static class ManagedAssetIndex {
    static Dictionary<ushort, OpCode> opcodes = new Dictionary<ushort, OpCode>();
    static ManagedAssetIndex() {
        foreach (var field in typeof(OpCodes).GetFields(BindingFlags.Public | BindingFlags.Static)) {
            if (field.FieldType == typeof(OpCode)) {
                var op = (OpCode)field.GetValue(null); opcodes[(ushort)op.Value] = op;
            }
        }
    }
    static string TypeName(MetadataReader reader, EntityHandle handle) {
        if (handle.Kind == HandleKind.TypeDefinition) {
            var t = reader.GetTypeDefinition((TypeDefinitionHandle)handle);
            var parent = t.GetDeclaringType();
            if (!parent.IsNil) return TypeName(reader, parent) + "+" + reader.GetString(t.Name);
            var ns = reader.GetString(t.Namespace);
            return (ns.Length > 0 ? ns + "." : "") + reader.GetString(t.Name);
        }
        if (handle.Kind == HandleKind.TypeReference) {
            var t = reader.GetTypeReference((TypeReferenceHandle)handle);
            if (t.ResolutionScope.Kind == HandleKind.TypeReference)
                return TypeName(reader, t.ResolutionScope) + "+" + reader.GetString(t.Name);
            var ns = reader.GetString(t.Namespace);
            return (ns.Length > 0 ? ns + "." : "") + reader.GetString(t.Name);
        }
        return "token:" + MetadataTokens.GetToken(handle).ToString("x8");
    }
    static Dictionary<string,object> Target(MetadataReader reader, EntityHandle handle) {
        if (handle.Kind == HandleKind.MethodSpecification)
            return Target(reader, reader.GetMethodSpecification((MethodSpecificationHandle)handle).Method);
        if (handle.Kind == HandleKind.MemberReference) {
            var member = reader.GetMemberReference((MemberReferenceHandle)handle);
            return new Dictionary<string,object>{{"type",TypeName(reader,member.Parent)},
                {"method",reader.GetString(member.Name)}, {"token",MetadataTokens.GetToken(handle)},
                {"signature",Convert.ToHexString(reader.GetBlobBytes(member.Signature)).ToLowerInvariant()}};
        }
        if (handle.Kind == HandleKind.MethodDefinition) {
            var method = reader.GetMethodDefinition((MethodDefinitionHandle)handle);
            return new Dictionary<string,object>{{"type",TypeName(reader,method.GetDeclaringType())},
                {"method",reader.GetString(method.Name)}, {"token",MetadataTokens.GetToken(handle)},
                {"signature",Convert.ToHexString(reader.GetBlobBytes(method.Signature)).ToLowerInvariant()}};
        }
        throw new InvalidDataException("Invalid call token " + MetadataTokens.GetToken(handle).ToString("x8"));
    }
    static bool AssetCall(Dictionary<string,object> target) {
        string type = (string)target["type"], method = (string)target["method"];
        if (!type.StartsWith("UnityEngine.")) return false;
        string[] assets = {"UnityEngine.Object", "UnityEngine.Resources", "UnityEngine.AssetBundle",
            "UnityEngine.Mesh", "UnityEngine.Material", "UnityEngine.Texture", "UnityEngine.Texture2D",
            "UnityEngine.RenderTexture", "UnityEngine.Cubemap", "UnityEngine.Sprite", "UnityEngine.Shader",
            "UnityEngine.ComputeShader", "UnityEngine.AudioClip", "UnityEngine.AnimationClip",
            "UnityEngine.RuntimeAnimatorController", "UnityEngine.AnimatorOverrideController",
            "UnityEngine.ScriptableObject", "UnityEngine.MaterialPropertyBlock", "UnityEngine.ImageConversion",
            "UnityEngine.ComputeBuffer", "UnityEngine.GraphicsBuffer",
            "UnityEngine.Networking.DownloadHandlerTexture", "UnityEngine.Networking.DownloadHandlerAudioClip"};
        foreach (string asset in assets) if (type == asset) return true;
        return method == "set_bones" || method == "set_rootBone" || method == "set_sharedMesh" ||
            method == "set_runtimeAnimatorController" || method == "set_targetTexture" ||
            method == "set_material" || method == "set_sharedMaterial" || method == "set_materials" ||
            method == "set_sharedMaterials" || method == "set_texture" || method == "set_mainTexture" ||
            method == "set_mesh" || method == "set_clip" || method == "SetPropertyBlock";
    }
    public static Dictionary<string,object> Read(string path, string sourceFile) {
        using (var stream = File.OpenRead(path)) using (var pe = new PEReader(stream)) {
            var reader = pe.GetMetadataReader(); var assembly = reader.GetAssemblyDefinition();
            var types = new List<object>(); var methods = new List<object>();
            var calls = new List<object>(); var references = new List<object>(); var errors = new List<object>();
            foreach (var referenceHandle in reader.AssemblyReferences) {
                var reference = reader.GetAssemblyReference(referenceHandle);
                references.Add(new Dictionary<string,object>{{"name",reader.GetString(reference.Name)},
                    {"version",reference.Version.ToString()}, {"culture",reader.GetString(reference.Culture)},
                    {"public_key_or_token",Convert.ToHexString(reader.GetBlobBytes(reference.PublicKeyOrToken)).ToLowerInvariant()}});
            }
            foreach (var typeHandle in reader.TypeDefinitions) {
                var type = reader.GetTypeDefinition(typeHandle); string typeName = TypeName(reader,typeHandle);
                types.Add(new Dictionary<string,object>{{"assembly",sourceFile}, {"full_name",typeName},
                    {"token",MetadataTokens.GetToken(typeHandle)}, {"flags",(int)type.Attributes},
                    {"base_type",type.BaseType.IsNil ? null : TypeName(reader,type.BaseType)}});
                foreach (var methodHandle in type.GetMethods()) {
                    var method = reader.GetMethodDefinition(methodHandle); string name = reader.GetString(method.Name);
                    int token = MetadataTokens.GetToken(methodHandle);
                    var parameters = new List<object>();
                    foreach (var parameterHandle in method.GetParameters()) {
                        var parameter = reader.GetParameter(parameterHandle);
                        parameters.Add(new Dictionary<string,object>{{"sequence",parameter.SequenceNumber},
                            {"name",reader.GetString(parameter.Name)}, {"flags",(int)parameter.Attributes}});
                    }
                    methods.Add(new Dictionary<string,object>{{"assembly",sourceFile},{"type",typeName},
                        {"name",name},{"token",token},{"rva",method.RelativeVirtualAddress},
                        {"flags",(int)method.Attributes},{"implementation_flags",(int)method.ImplAttributes},
                        {"signature",Convert.ToHexString(reader.GetBlobBytes(method.Signature)).ToLowerInvariant()},
                        {"parameters",parameters}});
                    if (method.RelativeVirtualAddress == 0) continue;
                    try {
                        byte[] il = pe.GetMethodBody(method.RelativeVirtualAddress).GetILBytes();
                        var strings = new List<object>();
                        for (int offset=0; offset<il.Length;) {
                            int start=offset; ushort key=il[offset++];
                            if (key==0xfe) key=(ushort)(0xfe00|il[offset++]);
                            OpCode op=opcodes[key]; int operand=offset; int length;
                            switch(op.OperandType) {
                                case OperandType.InlineNone: length=0;break;
                                case OperandType.ShortInlineBrTarget: case OperandType.ShortInlineI: case OperandType.ShortInlineVar: length=1;break;
                                case OperandType.InlineVar:length=2;break;
                                case OperandType.InlineI8:case OperandType.InlineR:length=8;break;
                                case OperandType.InlineSwitch:length=4+4*BitConverter.ToInt32(il,offset);break;
                                default:length=4;break;
                            }
                            if(length<0 || offset+length>il.Length) throw new InvalidDataException("Truncated IL operand");
                            if(op.OperandType==OperandType.InlineString) {
                                int stringToken=BitConverter.ToInt32(il,operand);
                                strings.Add(new Dictionary<string,object>{{"il_offset",start},
                                    {"value",reader.GetUserString(MetadataTokens.UserStringHandle(stringToken & 0xffffff))}});
                            }
                            if(op.OperandType==OperandType.InlineMethod) {
                                var target=Target(reader,MetadataTokens.EntityHandle(BitConverter.ToInt32(il,operand)));
                                if(AssetCall(target)) calls.Add(new Dictionary<string,object>{{"assembly",sourceFile},
                                    {"type",typeName},{"method",name},{"method_token",token},
                                    {"il_offset",start},{"opcode",op.Name},{"target",target},
                                    {"preceding_string_literals",new List<object>(strings)}});
                            }
                            offset+=length;
                        }
                    } catch(Exception error) {
                        errors.Add(new Dictionary<string,object>{{"assembly",sourceFile},{"method_token",token},
                            {"type",typeName},{"method",name},{"error",error.ToString()}});
                    }
                }
            }
            var module=reader.GetModuleDefinition();
            return new Dictionary<string,object>{{"name",reader.GetString(assembly.Name)},
                {"version",assembly.Version.ToString()}, {"culture",reader.GetString(assembly.Culture)},
                {"public_key",Convert.ToHexString(reader.GetBlobBytes(assembly.PublicKey)).ToLowerInvariant()},
                {"mvid",reader.GetGuid(module.Mvid).ToString()}, {"references",references},
                {"types",types},{"methods",methods},{"asset_calls",calls},{"diagnostics",errors}};
        }
    }
}
"@
if ($Mode -eq 'Index') {
    $assemblies = [Collections.Generic.List[object]]::new()
    $types = [Collections.Generic.List[object]]::new()
    $methods = [Collections.Generic.List[object]]::new()
    $calls = [Collections.Generic.List[object]]::new()
    $diagnostics = [Collections.Generic.List[object]]::new()
    foreach ($file in (Get-ChildItem -LiteralPath $AssemblyPath -File -Filter '*.dll' | Sort-Object Name)) {
        $source = 'TheLongDrive_Data/Managed/' + $file.Name
        $item = [ManagedAssetIndex]::Read($file.FullName, $source)
        foreach ($row in $item['types']) { $types.Add($row) }
        foreach ($row in $item['methods']) { $methods.Add($row) }
        foreach ($row in $item['asset_calls']) { $calls.Add($row) }
        foreach ($row in $item['diagnostics']) { $diagnostics.Add($row) }
        $assemblies.Add([ordered]@{file=$source;name=$item['name'];version=$item['version'];culture=$item['culture'];
            public_key=$item['public_key'];mvid=$item['mvid'];references=$item['references'];
            bytes=$file.Length;sha256=(Hash-File $file.FullName);type_count=$item['types'].Count;method_count=$item['methods'].Count})
    }
    Write-Json 'index.json' ([ordered]@{schema_version=1;complete=($diagnostics.Count -eq 0);assemblies=$assemblies;
        types=$types;methods=$methods;asset_calls=$calls;diagnostics=$diagnostics;
        metadata_reader=[ordered]@{version=[Reflection.Metadata.MetadataReader].Assembly.GetName().Version.ToString();
            sha256=(Hash-File ([Reflection.Metadata.MetadataReader].Assembly.Location))}})
    if ($diagnostics.Count) { throw 'Managed IL indexing contains errors. See index.json.' }
    exit 0
}
$receipt = [ordered]@{schema_version=1;complete=$false;mode='whole_module';
    input=[ordered]@{file='TheLongDrive_Data/Managed/Assembly-CSharp.dll';sha256=$null};
    host=[ordered]@{powershell=$PSVersionTable.PSVersion.ToString();dotnet=[Environment]::Version.ToString()};
    diagnostics=@()}
try {
    $inputHash = Hash-File $AssemblyPath
    $receipt['input']['sha256'] = $inputHash
    $DecompilerPath = [IO.Path]::GetFullPath($DecompilerPath)
    $receipt['decompiler'] = [ordered]@{name='ICSharpCode.Decompiler';sha256=(Hash-File $DecompilerPath)}
    $packageRoot = [IO.Directory]::GetParent([IO.Directory]::GetParent([IO.Path]::GetDirectoryName($DecompilerPath)).FullName).FullName
    $nuspec = Join-Path $packageRoot 'ICSharpCode.Decompiler.nuspec'
    $receipt['decompiler']['package_sha256'] = Hash-File $nuspec
    [xml]$package = [IO.File]::ReadAllText($nuspec)
    $receipt['decompiler']['package_version'] = [string]$package.package.metadata.version
    if ($package.package.metadata.version -ne '11.1.0.9782') { throw 'Decompiler package version mismatch.' }
    $decompilerAssembly = [Reflection.Assembly]::LoadFrom($DecompilerPath)
    $receipt['decompiler']['version'] = $decompilerAssembly.GetName().Version.ToString()
    if ($decompilerAssembly.GetName().Version.ToString() -ne '11.1.0.9782') { throw 'Decompiler assembly version mismatch.' }
    $dependencies = [Collections.Generic.List[object]]::new()
    $receipt['decompiler']['dependencies'] = $dependencies
    foreach ($dependency in $package.package.metadata.dependencies.group.dependency) {
        $loaded = [Reflection.Assembly]::Load([string]$dependency.id)
        $required = [Version]([string]$dependency.version)
        $dependencies.Add([ordered]@{name=[string]$dependency.id;required=[string]$dependency.version;
            version=$loaded.GetName().Version.ToString();sha256=(Hash-File $loaded.Location)})
        if ($loaded.GetName().Version -lt $required) { throw ('Decompiler dependency version mismatch: ' + $dependency.id) }
    }
    $settings = [ICSharpCode.Decompiler.DecompilerSettings]::new()
    $decompiler = [ICSharpCode.Decompiler.CSharp.CSharpDecompiler]::new($AssemblyPath, $settings)
    $text = $decompiler.DecompileWholeModuleAsString()
    $codePath = Join-Path $OutputPath 'Assembly-CSharp.cs'
    [IO.File]::WriteAllText($codePath, $text, [Text.UTF8Encoding]::new($false))
    $receipt['output'] = [ordered]@{file='managed/Assembly-CSharp.cs';
        bytes=(Get-Item -LiteralPath $codePath).Length;sha256=(Hash-File $codePath)}
    $diagnostics = @([regex]::Matches($text, '(?m)^\s*//.*(?:Error decompiling|Unknown result type|Expected[ :]|Invalid comparison|ILSpy generated).*') | ForEach-Object { $_.Value.Trim() })
    $receipt['diagnostics'] = $diagnostics
    $errors = @($diagnostics | Where-Object { $_ -match 'Error decompiling' })
    $metadata = [ManagedAssetIndex]::Read($AssemblyPath, 'TheLongDrive_Data/Managed/Assembly-CSharp.dll')
    $receipt['type_count'] = $metadata['types'].Count
    $receipt['method_count'] = $metadata['methods'].Count
    $complete = ($errors.Count -eq 0 -and $metadata['diagnostics'].Count -eq 0)
    $receipt['complete'] = $complete
    Write-Json 'decompilation.json' $receipt
    if ((Hash-File $AssemblyPath) -ne $inputHash) { throw 'Decompilation input changed.' }
    if (-not $complete) { throw 'Decompiler reported incomplete code. See decompilation.json.' }
} catch {
    $receipt['complete'] = $false
    $receipt['diagnostics'] = @($receipt['diagnostics']) + @($_.Exception.ToString())
    Write-Json 'decompilation.json' $receipt
    throw
}
