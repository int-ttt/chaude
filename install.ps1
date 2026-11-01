# chaude installer (Windows amd64 + arm64). Standalone: no repo checkout needed.
#   irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
# Env: CHAUDE_VERSION (default latest, e.g. v0.9.11), CHAUDE_INSTALL_DIR (default %USERPROFILE%\.local\bin), CHAUDE_REPO.
# Private repo: if `gh` is installed and logged in, it is used for downloads.
# Runs through iex, so no `exit` or `param` here: errors are caught and printed, the shell stays open.
# Keep this file ASCII-only (irm may decode non-ASCII text incorrectly on Windows PowerShell 5.1).

function Install-Chaude {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = if ($env:CHAUDE_REPO) { $env:CHAUDE_REPO } else { 'int-ttt/chaude' }
    $version = if ($env:CHAUDE_VERSION) { $env:CHAUDE_VERSION } else { 'latest' }
    $dir = if ($env:CHAUDE_INSTALL_DIR) { $env:CHAUDE_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.local\bin' }

    $pa = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $arch = switch ($pa) {
        'AMD64' { 'amd64' }
        'ARM64' { 'arm64' }
        default { throw "unsupported architecture: $pa" }
    }
    if ($version -ne 'latest' -and $version -notlike 'v*') { $version = "v$version" }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('chaude-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $useGh = $false
        if (Get-Command gh -ErrorAction SilentlyContinue) {
            & gh auth status *> $null
            $useGh = ($LASTEXITCODE -eq 0)
        }

        if ($useGh) {
            # works for private repos (uses your gh login)
            Write-Host "downloading via gh from $repo"
            $tagArgs = if ($version -eq 'latest') { @() } else { @($version) }
            & gh release download @tagArgs -R $repo -p "chaude_*_windows_$arch.zip" -p checksums.txt -D $tmp --clobber
            if ($LASTEXITCODE -ne 0) { throw "gh release download failed for $repo" }
        } else {
            if ($version -eq 'latest') {
                try {
                    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -UseBasicParsing
                    $version = $rel.tag_name
                } catch {
                    throw "could not resolve latest release of $repo (private repo? install and log in with gh: gh auth login)"
                }
            }
            $name = "chaude_$($version.TrimStart('v'))_windows_$arch.zip"
            $base = "https://github.com/$repo/releases/download/$version"
            Write-Host "downloading $name"
            Invoke-WebRequest -Uri "$base/$name" -OutFile (Join-Path $tmp $name) -UseBasicParsing
            Invoke-WebRequest -Uri "$base/checksums.txt" -OutFile (Join-Path $tmp 'checksums.txt') -UseBasicParsing
        }

        $zip = Get-ChildItem -Path $tmp -Filter "chaude_*_windows_$arch.zip" | Select-Object -First 1
        if (-not $zip) { throw "download failed: no chaude_*_windows_$arch.zip in $tmp" }
        $sums = Join-Path $tmp 'checksums.txt'
        if (-not (Test-Path $sums)) { throw 'checksums.txt not found in release; refusing to install' }

        # checksums.txt is required: never install an unverified binary
        $line = Get-Content $sums | Where-Object { $_ -match ('\s\*?' + [regex]::Escape($zip.Name) + '$') } | Select-Object -First 1
        if (-not $line) { throw "$($zip.Name) is not listed in checksums.txt" }
        $want = ($line.Trim() -split '\s+')[0].ToLower()
        $got = (Get-FileHash -Algorithm SHA256 -Path $zip.FullName).Hash.ToLower()
        if ($want -ne $got) { throw "checksum mismatch for $($zip.Name)" }

        Expand-Archive -Path $zip.FullName -DestinationPath $tmp -Force
        $src = Join-Path $tmp 'chaude.exe'
        if (-not (Test-Path $src)) { throw 'chaude.exe not found in archive' }

        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $dest = Join-Path $dir 'chaude.exe'
        if (Test-Path $dest) {
            $old = (& $dest --version 2>$null | Select-Object -First 1) -replace '^chaude\s+', ''
            $old = ("$old" -replace '[^A-Za-z0-9._-]', '_').Trim()
            if (-not $old) { $old = 'unknown' }
            $bak = Join-Path $dir "chaude.$old.bak"
            Move-Item -Force $dest $bak
            Write-Host "backed up previous binary to $bak"
        }
        Copy-Item -Force $src $dest

        $onPath = ($env:Path -split ';') -contains $dir
        if (-not $onPath) {
            Write-Host "PATH hint: $dir is not in PATH. Add it for your user account, then open a new terminal:"
            Write-Host "  [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';$dir', 'User')"
        }

        Write-Host "installed: $dest"
        & $dest --version
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

try {
    Install-Chaude
} catch {
    Write-Host "error: $($_.Exception.Message)" -ForegroundColor Red
}
