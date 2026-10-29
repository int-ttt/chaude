# chaude installer for Windows (PowerShell 5.1+):
#   irm https://raw.githubusercontent.com/int-ttt/chaude/main/install.ps1 | iex
# Env: CHAUDE_REPO (owner/name), CHAUDE_VERSION (tag, default latest), CHAUDE_INSTALL_DIR
$ErrorActionPreference = 'Stop'
$repo = if ($env:CHAUDE_REPO) { $env:CHAUDE_REPO } else { 'int-ttt/chaude' }
$version = if ($env:CHAUDE_VERSION) { $env:CHAUDE_VERSION } else { 'latest' }
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'amd64' }
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

if ($version -eq 'latest') {
  $version = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest").tag_name
  if (-not $version) { throw "could not resolve the latest release of $repo (private repo? set CHAUDE_VERSION)" }
}
$name = "chaude_$($version.TrimStart('v'))_windows_$arch.zip"
$base = "https://github.com/$repo/releases/download/$version"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("chaude-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Host "downloading $name"
  Invoke-WebRequest "$base/$name" -OutFile (Join-Path $tmp $name) -UseBasicParsing
  try { Invoke-WebRequest "$base/checksums.txt" -OutFile (Join-Path $tmp 'checksums.txt') -UseBasicParsing } catch { }
  $sums = Join-Path $tmp 'checksums.txt'
  if (Test-Path $sums) {
    $line = Select-String -Path $sums -Pattern ("\s" + [regex]::Escape($name) + "$") | Select-Object -First 1
    if ($line) {
      $want = ($line.Line -split '\s+')[0].ToLower()
      $got = (Get-FileHash (Join-Path $tmp $name) -Algorithm SHA256).Hash.ToLower()
      if ($want -ne $got) { throw 'checksum mismatch' }
    }
  }
  Expand-Archive -Path (Join-Path $tmp $name) -DestinationPath $tmp -Force
  $dir = if ($env:CHAUDE_INSTALL_DIR) { $env:CHAUDE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\chaude' }
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  Copy-Item (Join-Path $tmp 'chaude.exe') (Join-Path $dir 'chaude.exe') -Force
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (($userPath -split ';') -contains $dir)) {
  [Environment]::SetEnvironmentVariable('Path', (($userPath.TrimEnd(';') + ';' + $dir).TrimStart(';')), 'User')
  $env:Path = "$env:Path;$dir"
  Write-Host "added $dir to your user PATH (open a new terminal to use 'chaude')"
}
Write-Host "installed: $dir\chaude.exe"
& (Join-Path $dir 'chaude.exe') --version
if (-not (Get-Command py -ErrorAction SilentlyContinue)) {
  Write-Host "[!] Python 3 with the 'py' launcher was not found. The hooks need it:  winget install Python.Python.3.13"
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  Write-Host "[!] Git for Windows was not found. Claude Code runs hooks through Git Bash:  winget install Git.Git"
}
try { & (Join-Path $dir 'chaude.exe') context-menu install | Out-Null; Write-Host "added 'chaude로 열기' to the folder right-click menu (undo: chaude context-menu remove)" } catch { }
Write-Host "next: right-click a project folder -> 'chaude로 열기', or: chaude install <project folder>"
