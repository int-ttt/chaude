# chaude installer (Windows amd64; arm64 machines get the amd64 build via x64 emulation). Standalone: installs prebuilt release binaries only
# (no source build, no repo checkout, so git and go are not needed). Public repo: no token required.
#   irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
# Installs to %USERPROFILE%\.local\bin\chaude.exe (user dir, no admin rights) and adds that folder to the user PATH (registry,
# once, only when it is missing).
# Env: CHAUDE_VERSION (default latest, e.g. v0.11.11), CHAUDE_REPO. CHAUDE_TEST_ROOT: tests only (prefix for USERPROFILE/LOCALAPPDATA).
# An older %LOCALAPPDATA%\Programs\chaude install is not removed here; the script prints how to remove it.
# Optional token (only for rate limits or private forks): GH_TOKEN, else GITHUB_TOKEN, else `gh auth token`.
# A rejected token is dropped and the request is retried anonymously.
# Runs through iex, so no `exit` or `param` here: errors are caught and printed, the shell stays open.
# Keep this file ASCII-only (irm may decode non-ASCII text incorrectly on Windows PowerShell 5.1).

function Save-ReleaseAsset {
    param([string]$Url, [string]$OutFile, [string]$Token)
    Add-Type -AssemblyName System.Net.Http
    # no auto-redirect: the token is sent to api.github.com only, the storage URL is fetched without it
    $handler = New-Object System.Net.Http.HttpClientHandler
    $handler.AllowAutoRedirect = $false
    $client = New-Object System.Net.Http.HttpClient -ArgumentList $handler
    try {
        $req = New-Object System.Net.Http.HttpRequestMessage -ArgumentList ([System.Net.Http.HttpMethod]::Get), $Url
        $req.Headers.Accept.ParseAdd('application/octet-stream')
        # GitHub API rejects requests without a User-Agent (HTTP 403); .NET HttpClient sends none by default
        $req.Headers.UserAgent.ParseAdd('chaude-installer')
        if ($Token) { $req.Headers.Authorization = New-Object System.Net.Http.Headers.AuthenticationHeaderValue -ArgumentList 'Bearer', $Token }
        $resp = $client.SendAsync($req).GetAwaiter().GetResult()
        $code = [int]$resp.StatusCode
        if ($code -ge 300 -and $code -lt 400) {
            $loc = $resp.Headers.Location
            $resp.Dispose()
            $req = New-Object System.Net.Http.HttpRequestMessage -ArgumentList ([System.Net.Http.HttpMethod]::Get), $loc
            $resp = $client.SendAsync($req).GetAwaiter().GetResult()
        }
        $resp.EnsureSuccessStatusCode() | Out-Null
        [IO.File]::WriteAllBytes($OutFile, $resp.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult())
        $resp.Dispose()
    } finally {
        $client.Dispose()
    }
}

function Install-Chaude {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    if ($PSVersionTable.PSVersion.Major -lt 5) { throw "PowerShell 5.1 or later is required (found $($PSVersionTable.PSVersion)). Install WMF 5.1 or PowerShell 7 and run again." }
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = if ($env:CHAUDE_REPO) { $env:CHAUDE_REPO } else { 'int-ttt/chaude' }
    $version = if ($env:CHAUDE_VERSION) { $env:CHAUDE_VERSION } else { 'latest' }
    # CHAUDE_TEST_ROOT (tests only) replaces LOCALAPPDATA and USERPROFILE with a temp root
    $localApp = if ($env:CHAUDE_TEST_ROOT) { Join-Path $env:CHAUDE_TEST_ROOT 'AppData\Local' } else { $env:LOCALAPPDATA }
    $userHome = if ($env:CHAUDE_TEST_ROOT) { Join-Path $env:CHAUDE_TEST_ROOT 'home' } else { $env:USERPROFILE }
    $dir = Join-Path $userHome '.local\bin'

    $pa = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $arch = switch ($pa) {
        'AMD64' { 'amd64' }
        'ARM64' { 'amd64' }
        default { throw "unsupported architecture: $pa" }
    }
    if ($pa -eq 'ARM64') { Write-Host 'note: no arm64 build is published for Windows; installing the windows/amd64 build (runs through x64 emulation).' }
    if ($version -ne 'latest' -and $version -notlike 'v*') { $version = "v$version" }

    $token = $env:GH_TOKEN
    if (-not $token) { $token = $env:GITHUB_TOKEN }
    if (-not $token -and (Get-Command gh -ErrorAction SilentlyContinue)) {
        $ErrorActionPreference = 'Continue'
        $token = (& gh auth token 2>$null | Out-String).Trim()
        $ErrorActionPreference = 'Stop'
    }
    $headers = @{ Accept = 'application/vnd.github+json' }
    if ($token) { $headers['Authorization'] = "Bearer $token" }

    $relUrl = if ($version -eq 'latest') { "https://api.github.com/repos/$repo/releases/latest" } else { "https://api.github.com/repos/$repo/releases/tags/$version" }
    $rel = $null
    if ($token) {
        try { $rel = Invoke-RestMethod -Uri $relUrl -Headers $headers -UseBasicParsing }
        catch {
            Write-Host 'warning: GitHub rejected the token; retrying without it'
            $token = ''
            $headers = @{ Accept = 'application/vnd.github+json' }
        }
    }
    if (-not $rel) {
        try { $rel = Invoke-RestMethod -Uri $relUrl -Headers $headers -UseBasicParsing }
        catch { throw "cannot read release $version of $repo. Check the version name and your network. If GitHub rate-limited you (HTTP 403), set GH_TOKEN and run again." }
    }
    $tag = $rel.tag_name
    if (-not $tag) { throw "could not resolve release tag of $repo" }

    $name = "chaude_$($tag.TrimStart('v'))_windows_$arch.zip"
    $zipAsset = $rel.assets | Where-Object { $_.name -eq $name } | Select-Object -First 1
    if (-not $zipAsset) { throw "$name not found in release $tag (no prebuilt binary for windows/$arch)" }
    $sumsAsset = $rel.assets | Where-Object { $_.name -eq 'checksums.txt' } | Select-Object -First 1
    if (-not $sumsAsset) { throw "checksums.txt not found in release $tag; refusing to install" }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('chaude-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "downloading $name ($tag)"
        $zip = Join-Path $tmp $name
        $sums = Join-Path $tmp 'checksums.txt'
        Save-ReleaseAsset -Url $zipAsset.url -OutFile $zip -Token $token
        Save-ReleaseAsset -Url $sumsAsset.url -OutFile $sums -Token $token

        # checksums.txt is required: never install an unverified binary
        $line = Get-Content $sums | Where-Object { $_ -match ('\s\*?' + [regex]::Escape($name) + '$') } | Select-Object -First 1
        if (-not $line) { throw "$name is not listed in checksums.txt" }
        $want = ($line.Trim() -split '\s+')[0].ToLower()
        $got = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLower()
        if ($want -ne $got) { throw "checksum mismatch for $name" }

        Expand-Archive -Path $zip -DestinationPath $tmp -Force
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

        # user PATH (registry, HKCU\Environment): add the folder once; an existing entry is left alone
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @()
        if ($userPath) { $entries = @($userPath -split ';' | Where-Object { $_ -ne '' }) }
        if (-not ($entries | Where-Object { $_.TrimEnd('\') -ieq $dir.TrimEnd('\') })) {
            $newPath = if ($userPath) { $userPath.TrimEnd(';') + ';' + $dir } else { $dir }
            [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Write-Host "PATH: added $dir to the user PATH. Open a new terminal to use it."
        }
        if (-not (($env:Path -split ';') -contains $dir)) { $env:Path = "$env:Path;$dir" }

        Write-Host "installed: $dest"
        & $dest --version
        # older installer (%LOCALAPPDATA%\Programs\chaude): not removed here; print how to remove it
        $oldExe = Join-Path $localApp 'Programs\chaude\chaude.exe'
        if (Test-Path $oldExe) {
            Write-Host "note: an older chaude is still at $oldExe. Remove it so the new one is used:" -ForegroundColor Yellow
            Write-Host "  Remove-Item -Force '$oldExe'" -ForegroundColor Yellow
            Write-Host "  then remove '$(Split-Path $oldExe)' from your user PATH (Environment Variables > User variables > Path)." -ForegroundColor Yellow
        }
        if (-not (Get-Command claude -ErrorAction SilentlyContinue)) {
            Write-Host 'warning: claude (Claude Code CLI) not found on PATH. chaude needs it to run. Install Claude Code and open a new terminal.' -ForegroundColor Yellow
        }
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

try {
    Install-Chaude
} catch {
    Write-Host "error: $($_.Exception.Message)" -ForegroundColor Red
}
