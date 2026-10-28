# Shared helpers for install.ps1 / fresh_start.ps1 (Windows PowerShell 5.1+ and PowerShell 7).
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

function Test-PythonCommand([string]$Cmd) {
    # Returns the full path of python.exe behind $Cmd, or $null (skips the Microsoft Store stub).
    $parts = $Cmd -split ' '
    $exe = $parts[0]
    $rest = @()
    if ($parts.Count -gt 1) { $rest = @($parts[1..($parts.Count - 1)]) }
    try {
        $out = & $exe @rest -c "import sys; print(sys.executable)" 2>$null
        if ($LASTEXITCODE -eq 0 -and $out) {
            $path = ([string]($out | Select-Object -Last 1)).Trim()
            if ($path -and (Test-Path -LiteralPath $path)) { return $path }
        }
    } catch { }
    return $null
}

function Find-Python {
    foreach ($c in @('py -3', 'python', 'python3')) {
        $p = Test-PythonCommand $c
        if ($p) { return [pscustomobject]@{ Exe = $p; Cmd = $c } }
    }
    return $null
}

function Get-BashPython {
    # The command agents type in the Bash/PowerShell tool to run scripts.
    foreach ($c in @('python', 'py -3', 'python3')) {
        if (Test-PythonCommand $c) { return $c }
    }
    return 'python'
}
