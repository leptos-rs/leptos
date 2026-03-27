# Autonomous test for leptos-webview2-repro (leptos#4610)
# Run with: powershell -NoProfile -Command "& { . Z:\src\leptos-webview2-repro\run-test.ps1 }"
# Or from repo root: powershell -NoProfile -Command "Set-Location Z:\src\leptos-webview2-repro; . .\run-test.ps1"

$ErrorActionPreference = "Continue"
$env:NO_COLOR = $null
$env:RUSTFLAGS = ""

$root = if ($PSScriptRoot) { $PSScriptRoot } else { Get-Location }
Set-Location $root
$logFile = Join-Path $root "test_run.log"

"" | Out-File $logFile -Encoding utf8
function Log { param($msg) Add-Content $logFile $msg; Write-Host $msg }

Log "=== leptos-webview2-repro autonomous test $(Get-Date -Format 'o') ==="

# 1. Build frontend
Log "`n--- Building frontend ---"
Push-Location (Join-Path $root "frontend")
cargo build --release --target wasm32-unknown-unknown --config 'target.wasm32-unknown-unknown.rustflags=[]' 2>&1 | ForEach-Object { Log $_ }
if ($LASTEXITCODE -ne 0) { Log "CARGO BUILD FAILED"; Pop-Location; exit 1 }
trunk build --release 2>&1 | ForEach-Object { Log $_ }
if ($LASTEXITCODE -ne 0) { Log "TRUNK BUILD FAILED"; Pop-Location; exit 1 }
Pop-Location

# 2. Start trunk serve
Log "`n--- Starting trunk serve ---"
$trunkJob = Start-Job -ScriptBlock {
    $env:NO_COLOR = $null; $env:RUSTFLAGS = ""
    Set-Location (Join-Path $using:root "frontend")
    trunk serve 2>&1
}
Start-Sleep -Seconds 12

# 3. Run Tauri app (60s)
Log "`n--- Running Tauri app (60s) ---"
$tauriJob = Start-Job -ScriptBlock {
    $env:NO_COLOR = $null; $env:RUSTFLAGS = ""
    Set-Location $using:root
    cargo tauri dev 2>&1
}
Start-Sleep -Seconds 60

Stop-Job $tauriJob,$trunkJob -ErrorAction SilentlyContinue
$tauriOut = Receive-Job $tauriJob
Remove-Job $tauriJob,$trunkJob -ErrorAction SilentlyContinue

Log "`n--- Tauri output ---"
$tauriOut | ForEach-Object { Log $_ }

$hasPanic = ($tauriOut | Out-String) -match "panic|callback removed|thread.*panicked"
Log "`n=== Result: $(if ($hasPanic) { 'PANIC DETECTED' } else { 'No panic in terminal output (panic may appear in WebView DevTools - open with Ctrl+Shift+I)' }) ==="
Log "Log: $logFile"
