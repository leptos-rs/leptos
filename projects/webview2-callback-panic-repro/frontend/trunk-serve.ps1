$env:NO_COLOR = $null
$env:RUSTFLAGS = ''
Set-Location $PSScriptRoot
trunk serve
