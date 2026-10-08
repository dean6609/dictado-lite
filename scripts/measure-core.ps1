param([Parameter(Mandatory)][string]$Model, [Parameter(Mandatory)][string]$Wav)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$localRoot = Join-Path $projectRoot 'local'
New-Item -ItemType Directory -Force $localRoot | Out-Null
$env:VK_LOADER_LAYERS_DISABLE = '~implicit~'
$arguments = @((Resolve-Path -LiteralPath $Model).Path, (Resolve-Path -LiteralPath $Wav).Path, '--idle-after=40')
$quotedArguments = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
$stopwatch = [Diagnostics.Stopwatch]::StartNew()
$measureProcess = Start-Process -FilePath (Join-Path $projectRoot 'target/release/dictado-lite.exe') -ArgumentList $quotedArguments -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $localRoot 'core-measure-transcript.jsonl') -RedirectStandardError (Join-Path $localRoot 'core-measure-engine.log')
$samples = [Collections.Generic.List[object]]::new()
while (!$measureProcess.HasExited) {
    $measureProcess.Refresh()
    $dedicated = $null
    $shared = $null
    try {
        $gpu = Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUProcessMemory -Filter "Name LIKE 'pid_$($measureProcess.Id)_%'" -ErrorAction Stop
        if ($gpu) {
            $dedicated = [double](($gpu | Measure-Object DedicatedUsage -Sum).Sum) / 1MB
            $shared = [double](($gpu | Measure-Object SharedUsage -Sum).Sum) / 1MB
        }
    } catch { }
    if ($measureProcess.HasExited) { break }
    $samples.Add([ordered]@{
        elapsed_s = [math]::Round($stopwatch.Elapsed.TotalSeconds, 3)
        resident_mib = [math]::Round($measureProcess.WorkingSet64 / 1MB, 3)
        private_mib = [math]::Round($measureProcess.PrivateMemorySize64 / 1MB, 3)
        cpu_seconds = [math]::Round($measureProcess.TotalProcessorTime.TotalSeconds, 3)
        gpu_dedicated_mib = $dedicated
        gpu_shared_mib = $shared
    })
    Start-Sleep -Milliseconds 1000
}
$measureProcess.WaitForExit()
if ($measureProcess.ExitCode -ne 0) { throw "Measurement process failed: $($measureProcess.ExitCode)" }
[ordered]@{
    scope = 'File CLI: inference then forty seconds idle; not tray or release-to-paste latency'
    gpu_source = 'Windows GPU Process Memory counters; null means unavailable, not zero'
    samples = $samples
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $localRoot 'core-resources.json')
$samples | Select-Object -First 3
$samples | Select-Object -Last 3
