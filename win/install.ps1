# Install the Opus-lean setup into a project (Windows).
# usage: powershell -ExecutionPolicy Bypass -File .\install.ps1 <project-dir> [-Main opus|sonnet]
#        (no project-dir: opens the settings window to pick folders and options)
# Safe to re-run: existing CLAUDE.md / settings.json are backed up and merged, not overwritten.
param(
    [Parameter(Position = 0)][string]$Project = '',
    [ValidateSet('opus', 'sonnet')][string]$Main = ''
)
$ErrorActionPreference = 'Stop'
$Src = $PSScriptRoot
. (Join-Path $Src 'tools\common.ps1')

if (-not $Project) {
    $Py = Find-Python
    if (-not $Py) { Write-Output "Python 3가 필요합니다. https://www.python.org/downloads/ (설치 시 'Add python.exe to PATH' 체크)"; exit 1 }
    & $Py.Exe -X utf8 (Join-Path $Src 'tools\configure.py')
    exit $LASTEXITCODE
}

$resolved = Resolve-Path -LiteralPath $Project -ErrorAction SilentlyContinue
if (-not $resolved) { Write-Output "✗ 폴더 없음: $Project"; exit 1 }
$Dst = $resolved.Path

$Py = Find-Python
if (-not $Py) {
    Write-Output "Python 3가 필요합니다. https://www.python.org/downloads/ 에서 설치하세요 (설치 화면에서 'Add python.exe to PATH' 체크)."
    exit 1
}
$BashPy = Get-BashPython

# Backup
$ts = Get-Date -Format 'yyyyMMdd-HHmmss'
$Bk = Join-Path $Dst ".claude-backup-$ts-$PID"
foreach ($f in @('CLAUDE.md', '.claude\settings.json')) {
    $p = Join-Path $Dst $f
    if (Test-Path -LiteralPath $p) {
        $t = Join-Path $Bk $f
        New-Item -ItemType Directory -Force -Path (Split-Path $t) | Out-Null
        Copy-Item -LiteralPath $p -Destination $t
    }
}

# Copy files
foreach ($d in @('.claude\agents', '.claude\hooks', '.claude\skills', 'scripts', 'templates',
                 'notes\research', 'notes\runs', 'notes\pastes', 'logs', 'docs\claude-lean')) {
    New-Item -ItemType Directory -Force -Path (Join-Path $Dst $d) | Out-Null
}
Copy-Item -Path (Join-Path $Src '.claude\agents\*.md') -Destination (Join-Path $Dst '.claude\agents') -Force
Copy-Item -Path (Join-Path $Src '.claude\hooks\*.py') -Destination (Join-Path $Dst '.claude\hooks') -Force
Copy-Item -Path (Join-Path $Src '.claude\skills\*') -Destination (Join-Path $Dst '.claude\skills') -Recurse -Force
Copy-Item -Path (Join-Path $Src 'scripts\*.py') -Destination (Join-Path $Dst 'scripts') -Force
Copy-Item -Path (Join-Path $Src 'templates\*.md') -Destination (Join-Path $Dst 'templates') -Force
foreach ($doc in @('README.md', 'APPLY_GUIDE.md', 'FRESH_START.md', 'MIGRATE_NOW_PROMPT.md', 'CHANGELOG.md')) {
    Copy-Item -LiteralPath (Join-Path $Src $doc) -Destination (Join-Path $Dst 'docs\claude-lean') -Force
}
if (-not (Test-Path -LiteralPath (Join-Path $Dst 'notes\FACTS.md'))) {
    Copy-Item -LiteralPath (Join-Path $Src 'templates\FACTS.template.md') -Destination (Join-Path $Dst 'notes\FACTS.md')
}
if (-not (Test-Path -LiteralPath (Join-Path $Dst 'notes\DECISIONS.md'))) {
    Copy-Item -LiteralPath (Join-Path $Src 'templates\DECISIONS.template.md') -Destination (Join-Path $Dst 'notes\DECISIONS.md')
}

# Merge CLAUDE.md and settings.json, fill in the Python path
& $Py.Exe -X utf8 (Join-Path $Src 'tools\merge_settings.py') $Src $Dst $Py.Exe $Main $BashPy $Bk
if ($LASTEXITCODE -ne 0) { Write-Output "✗ 설정 병합 실패"; exit 1 }

if (Test-Path -LiteralPath $Bk) { Write-Output "백업: $Bk" }
Write-Output "설치 완료: $Dst"
Write-Output "다음: cd `"$Dst`"; claude   (새 세션에서 /agents, /hooks 확인 — docs\claude-lean\APPLY_GUIDE.md 3단계)"
