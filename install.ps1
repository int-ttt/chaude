# chaude installer (Windows amd64 + arm64). Standalone: installs prebuilt release binaries only
# (no source build, no repo checkout, so git and go are not needed). Public repo: no token required.
#   irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
# Env: CHAUDE_VERSION (default latest, e.g. v0.9.13), CHAUDE_INSTALL_DIR (default %USERPROFILE%\.local\bin), CHAUDE_REPO.
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
    $dir = if ($env:CHAUDE_INSTALL_DIR) { $env:CHAUDE_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.local\bin' }

    $pa = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $arch = switch ($pa) {
        'AMD64' { 'amd64' }
        'ARM64' { 'arm64' }
        default { throw "unsupported architecture: $pa" }
    }
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

        $onPath = ($env:Path -split ';') -contains $dir
        if (-not $onPath) {
            Write-Host "PATH hint: $dir is not in PATH. Add it for your user account, then open a new terminal:"
            Write-Host "  [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';$dir', 'User')"
        }

        Write-Host "installed: $dest"
        & $dest --version
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
