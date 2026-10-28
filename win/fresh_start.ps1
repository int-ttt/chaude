# Fresh start for one or more projects (Windows): install, extract old transcripts into notes\distill, archive them.
# usage: powershell -ExecutionPolicy Bypass -File .\fresh_start.ps1 <project-dir> [<project-dir> ...] [-Main opus|sonnet] [-Force]
# Close every Claude Code window for these projects first. Then, per project: cd <dir>; claude   and type /distill
param(
    [Parameter(Mandatory = $true, Position = 0, ValueFromRemainingArguments = $true)][string[]]$Projects,
    [ValidateSet('opus', 'sonnet')][string]$Main = '',
    [switch]$Force
)
$ErrorActionPreference = 'Continue'
$Src = $PSScriptRoot
. (Join-Path $Src 'tools\common.ps1')

if (-not $Force -and (Get-Process -Name claude -ErrorAction SilentlyContinue)) {
    Write-Output "실행 중인 Claude가 있습니다. 이 프로젝트들의 Claude 창을 닫은 뒤 다시 실행하세요."
    Write-Output "(다른 프로젝트의 Claude나 Claude 데스크톱 앱만 열려 있다면 끝에 -Force 를 붙이세요)"
    exit 1
}
$Py = Find-Python
if (-not $Py) { Write-Output "Python 3가 필요합니다. https://www.python.org/downloads/ (설치 시 'Add python.exe to PATH' 체크)"; exit 1 }

$Archive = Join-Path $HOME 'claude-archive'
New-Item -ItemType Directory -Force -Path $Archive | Out-Null
$Baseline = Join-Path $HOME 'usage-before.txt'
if (-not (Test-Path -LiteralPath $Baseline)) {
    & $Py.Exe -X utf8 (Join-Path $Src 'scripts\usage_report.py') 2>$null | Out-File -Encoding utf8 -FilePath $Baseline
}

$done = @()
foreach ($arg in $Projects) {
    $r = Resolve-Path -LiteralPath $arg -ErrorAction SilentlyContinue
    if (-not $r) { Write-Output "✗ 폴더 없음: $arg"; continue }
    $P = $r.Path
    $name = Split-Path -Path $P -Leaf
    Write-Output "== $name ($P)"

    $inst = @{ Project = $P }
    if ($Main) { $inst.Main = $Main }
    $out = & (Join-Path $Src 'install.ps1') @inst
    if ($LASTEXITCODE -ne 0) { Write-Output "✗ 설치 실패: $name"; $out | ForEach-Object { "   $_" }; continue }
    $out | Where-Object { $_ -match 'CLAUDE.md|settings.json|백업' } | ForEach-Object { "   $_" }

    $hist = ([string](& $Py.Exe -X utf8 (Join-Path $P 'scripts\claude_history.py') $P)).Trim()
    if (-not $hist -or -not (Test-Path -LiteralPath $hist)) {
        Write-Output "   대화 기록 없음 → 추출·보관 건너뜀. 바로 'cd `"$P`"; claude' 로 시작하세요."
        $done += , @($P, 'none')
        continue
    }

    $ex = (& $Py.Exe -X utf8 (Join-Path $P 'scripts\extract_transcript.py') $hist --out (Join-Path $P 'notes\distill') 2>&1 | Out-String).Trim()
    Write-Output "   $ex"
    $n = 0
    if ($ex -match '메시지 (\d+)개') { $n = [int]$Matches[1] }
    if ($n -eq 0) { Write-Output "✗ 추출된 메시지가 없어 기록을 보관하지 않았습니다. 위 출력을 확인하세요."; continue }

    $date = Get-Date -Format 'yyyyMMdd'
    $dest = Join-Path $Archive "$name-$date"
    $i = 2
    while (Test-Path -LiteralPath $dest) { $dest = Join-Path $Archive "$name-$date-$i"; $i++ }
    try {
        Move-Item -LiteralPath $hist -Destination $dest -ErrorAction Stop
        Write-Output "   기록 보관: $dest"
        & $Py.Exe -X utf8 (Join-Path $P 'scripts\compress_archive.py') $dest
        $done += , @($P, 'distill')
    } catch {
        Write-Output "✗ 기록 보관 실패 (Claude가 열려 있으면 닫고 다시 실행): $($_.Exception.Message)"
    }
}

Write-Output ""
Write-Output "다음 단계:"
foreach ($d in $done) {
    if ($d[1] -eq 'distill') { Write-Output "  cd `"$($d[0])`"; claude   → /distill 입력 → notes\distill\REVIEW.md 질문에 답하기" }
    else { Write-Output "  cd `"$($d[0])`"; claude" }
}
