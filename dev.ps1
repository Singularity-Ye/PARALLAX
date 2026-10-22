param([ValidateSet('run', 'test', 'check', 'build')][string]$Task = 'run')
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$cargoPath = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (!(Test-Path -LiteralPath $cargoPath)) { throw '未找到用户 Rust 工具链，请先安装 rustup。' }
$env:PATH = (Split-Path -Parent $cargoPath) + ';' + $env:PATH
switch ($Task) {
    'check' {
        & $cargoPath fmt --check
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        & $cargoPath clippy --locked --all-targets -- -D warnings
    }
    'build' { & $cargoPath build --locked --release }
    'test' { & $cargoPath test --locked }
    'run' { & $cargoPath run --locked }
}
exit $LASTEXITCODE
