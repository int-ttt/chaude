# Windows: make sure Go is available (installs a project-local copy under .tools\go if missing or too old),
# find Python (hooks are .claude\hooks\*.py), build bin\chaude.exe, write windows .claude\settings.json
# (hooks in exec form: <python> <script>, no Git Bash needed).
# Usage: powershell -ExecutionPolicy Bypass -File platform_setup\install.ps1 [-Cross]
# Env:   GO_VERSION = version downloaded when no usable Go is found; HARNESS_PYTHON = full path to python.exe to use for hooks
param([switch]$Cross)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$GoVersion = if ($env:GO_VERSION) { $env:GO_VERSION } else { '1.27.3' }
$GoMinMinor = 22   # go.mod: go 1.22
$root = Split-Path -Parent $PSScriptRoot
$arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'AMD64' { 'amd64' } 'ARM64' { 'arm64' } default { throw "unsupported arch: $env:PROCESSOR_ARCHITECTURE" } }

function Test-Go([string]$go) {
    if (-not $go -or -not (Test-Path $go)) { return $false }
    try { $v = & $go env GOVERSION 2>$null } catch { return $false }
    if ($v -match '^go1\.(\d+)') { return [int]$Matches[1] -ge $GoMinMinor }
    return $false
}

function Install-Go {
    $pkg = "go$GoVersion.windows-$arch.zip"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    Write-Host "Go not found (or older than 1.$GoMinMinor): downloading $pkg into .tools\go"
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri "https://dl.google.com/go/$pkg" -OutFile (Join-Path $tmp $pkg)
    $want = (Invoke-WebRequest -UseBasicParsing -Uri "https://dl.google.com/go/$pkg.sha256").Content
    if ($want -is [byte[]]) { $want = [Text.Encoding]::ASCII.GetString($want) }
    $got = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $pkg)).Hash
    if ($got.ToLower() -ne $want.Trim().ToLower()) { Remove-Item -Recurse -Force $tmp; throw "checksum mismatch for $pkg" }
    $tools = Join-Path $root '.tools'
    if (Test-Path (Join-Path $tools 'go')) { Remove-Item -Recurse -Force (Join-Path $tools 'go') }
    New-Item -ItemType Directory -Force -Path $tools | Out-Null
    Expand-Archive -Path (Join-Path $tmp $pkg) -DestinationPath $tools
    Remove-Item -Recurse -Force $tmp
}

$localGo = Join-Path $root '.tools\go\bin\go.exe'
$sysGo = (Get-Command go -ErrorAction SilentlyContinue).Source
if (Test-Go $localGo) { $go = $localGo }
elseif (Test-Go $sysGo) { $go = $sysGo }
else { Install-Go; $go = $localGo; if (-not (Test-Go $go)) { throw 'downloaded Go does not run' } }
Write-Host "go: $go ($(& $go env GOVERSION))"
if (-not (Get-Command claude -ErrorAction SilentlyContinue)) { Write-Warning 'claude CLI not on PATH' }
# hook interpreter: first of py -3 / python / python3 that really runs Python >= 3.8
# (the Microsoft Store "python.exe" alias fails this check)
function Test-Python([string[]]$cmd) {
    if (-not (Get-Command $cmd[0] -ErrorAction SilentlyContinue)) { return $false }
    $rest = @($cmd | Select-Object -Skip 1)
    try { & $cmd[0] @rest -c 'import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)' 2>$null | Out-Null; return ($LASTEXITCODE -eq 0) } catch { return $false }
}
$py = $null
$cands = @()
if ($env:HARNESS_PYTHON) { $cands += ,@($env:HARNESS_PYTHON) }
$cands += ,@('py', '-3'); $cands += ,@('python'); $cands += ,@('python3')
foreach ($c in $cands) { if (Test-Python $c) { $py = $c; break } }
if (-not $py) { throw 'Python (>= 3.8) not found: hooks need it. Install from https://www.python.org/downloads/ (check "Add python.exe to PATH" or keep the py launcher) and re-run.' }
$pyCmd = $py -join ' '
Write-Host "python: $pyCmd"

Push-Location $root
try {
    $env:CGO_ENABLED = '0'
    New-Item -ItemType Directory -Force -Path bin | Out-Null
    # Go module lives in src\main\intt; outputs stay under the repo root (bin\ layout: <exe>\..\.claude)
    $mod = Join-Path $root 'src\main\intt'
    & $go -C $mod build -trimpath -o (Join-Path $root 'bin\chaude.exe') .\cmd\chaude
    if ($LASTEXITCODE -ne 0) { throw 'go build failed' }
    if ($Cross) {
        New-Item -ItemType Directory -Force -Path dist | Out-Null
        foreach ($t in 'darwin/arm64', 'darwin/amd64', 'linux/amd64', 'linux/arm64', 'windows/amd64', 'windows/arm64') {
            $os, $a = $t.Split('/')
            $ext = if ($os -eq 'windows') { '.exe' } else { '' }
            $env:GOOS = $os; $env:GOARCH = $a
            & $go -C $mod build -trimpath -o (Join-Path $root "dist\chaude-$os-$a$ext") .\cmd\chaude
            if ($LASTEXITCODE -ne 0) { throw "cross build failed: $t" }
        }
        Remove-Item Env:GOOS, Env:GOARCH
        Write-Host 'cross builds: dist\'
    }
    & .\bin\chaude.exe gen-settings windows --src .claude\settings.json --out .claude\settings.json --python $pyCmd
    if ($LASTEXITCODE -ne 0) { throw 'gen-settings failed' }
} finally { Pop-Location }
Write-Host "state dir: $env:LOCALAPPDATA\claude-harness"
Write-Host "ok (windows/$arch): bin\chaude.exe"
