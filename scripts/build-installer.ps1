param([Parameter(Mandatory)][string]$Model, [string]$Output = 'artifacts/Dictado-Lite-0.1.0-Setup.exe')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
    $pin = Get-Content models/manifest.json -Raw | ConvertFrom-Json
    if ((Get-Item -LiteralPath $Model).Length -ne $pin.bytes -or (Get-FileHash -LiteralPath $Model -Algorithm SHA256).Hash.ToLowerInvariant() -ne $pin.sha256) { throw 'Pinned model size/hash mismatch' }
    $stage = Join-Path $projectRoot ('artifacts/payload-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage | Out-Null
    foreach ($name in 'dictado-lite.exe','ggml-base.dll','ggml-cpu.dll','ggml.dll','libtranscribe.dll') { Copy-Item -LiteralPath (Join-Path 'target/release' $name) $stage }
    if (Test-Path target/release/ggml-vulkan.dll) {Copy-Item target/release/ggml-vulkan.dll $stage}
    foreach ($name in 'libc++.dll','libunwind.dll') { if (Test-Path (Join-Path 'target/release' $name)) {Copy-Item -LiteralPath (Join-Path 'target/release' $name) $stage} }
    Copy-Item -LiteralPath 'target/release/dictado-setup.exe' (Join-Path $stage 'dictado-uninstall.exe')
    New-Item -ItemType Directory -Path (Join-Path $stage 'models'),(Join-Path $stage 'licenses/rust') | Out-Null
    Copy-Item -LiteralPath $Model (Join-Path $stage ('models/' + $pin.filename))
    Copy-Item models/manifest.json (Join-Path $stage 'models/manifest.json')
    Copy-Item LICENSE,THIRD_PARTY_NOTICES.md $stage
    Copy-Item licenses/* (Join-Path $stage 'licenses') -Recurse -Force
    $triple = rustc -vV | Select-String '^host: ' | ForEach-Object {$_.Line.Substring(6).Trim()}
    if ($triple -ne 'x86_64-pc-windows-gnu') {throw 'Offline packaging currently supports the verified GNU x64 release; MSVC builds remain development/CI checks'}
    $metadata = cargo metadata --locked --format-version 1 --filter-platform $triple | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $notices = [System.Collections.Generic.List[string]]::new()
    $notices.Add('Resolved Rust dependencies (including build-time crates). Original copyright/license texts follow. Sources: crates.io, Cargo.lock. The Rust standard library is MIT/Apache-2.0; its notices are included separately.')
    foreach ($package in $metadata.packages | Where-Object source | Sort-Object name,version) {
        $crate = Split-Path $package.manifest_path -Parent
        $texts = @(Get-ChildItem -LiteralPath $crate -File | Where-Object { $_.Name -match '^(LICENSE|COPYING|NOTICE|UNLICENSE)' })
        if ($texts.Count -eq 0) {
            $override = Join-Path $projectRoot ('licenses/overrides/' + $package.name + '-' + $package.version)
            if (Test-Path -LiteralPath $override) {$texts = @(Get-ChildItem -LiteralPath $override -File)}
            if ($texts.Count -eq 0) {throw "Missing license text: $($package.name) $($package.version)"}
        }
        $folder = Join-Path $stage ('licenses/rust/' + $package.name + '-' + $package.version)
        New-Item -ItemType Directory -Path $folder | Out-Null
        foreach ($text in $texts) {Copy-Item -LiteralPath $text.FullName $folder}
        $notices.Add("$($package.name) $($package.version) — $($package.license) — $($package.repository)")
    }
    $notices | Set-Content (Join-Path $stage 'licenses/RUST-DEPENDENCIES.txt') -Encoding utf8
    $files = @(Get-ChildItem -LiteralPath $stage -File -Recurse | Sort-Object FullName)
    $target = [System.IO.Path]::GetFullPath((Join-Path $projectRoot $Output))
    $artifactPrefix = (Join-Path $projectRoot 'artifacts') + [System.IO.Path]::DirectorySeparatorChar
    if (!$target.StartsWith($artifactPrefix,[StringComparison]::OrdinalIgnoreCase)) {throw 'Output must stay inside project artifacts'}
    New-Item -ItemType Directory -Force (Split-Path $target -Parent) | Out-Null
    $out = [System.IO.File]::Open($target,[System.IO.FileMode]::Create,[System.IO.FileAccess]::Write,[System.IO.FileShare]::None)
    $manifest = [ordered]@{product='dean6609/dictado-lite';version='0.1.0';files=@()}
    try {
        $stub = [System.IO.File]::OpenRead((Join-Path $projectRoot 'target/release/dictado-setup.exe'))
        try {$stub.CopyTo($out)} finally {$stub.Dispose()}
        foreach ($file in $files) {
            $entry = [ordered]@{name=$file.FullName.Substring($stage.Length+1).Replace('\','/');bytes=$file.Length;sha256=(Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant();offset=$out.Position}
            $manifest.files += $entry
            $inputFile = [System.IO.File]::OpenRead($file.FullName)
            try {$inputFile.CopyTo($out)} finally {$inputFile.Dispose()}
        }
        $json = [System.Text.Encoding]::UTF8.GetBytes(($manifest | ConvertTo-Json -Depth 5 -Compress))
        $out.Write($json,0,$json.Length)
        $length = [BitConverter]::GetBytes([uint64]$json.Length);$out.Write($length,0,$length.Length)
        $magic = [System.Text.Encoding]::ASCII.GetBytes('DICTADO-PACK-v1!');$out.Write($magic,0,$magic.Length)
        $out.Flush($true)
    } finally {$out.Dispose()}
    $manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path (Split-Path $target -Parent) 'payload-manifest.json') -Encoding utf8
    $hash=(Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $([System.IO.Path]::GetFileName($target))" | Set-Content ($target+'.sha256') -Encoding ascii
    Write-Output "$target ($((Get-Item -LiteralPath $target).Length) bytes; SHA256 $hash)"
} finally {Pop-Location}
