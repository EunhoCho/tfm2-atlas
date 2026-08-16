param(
    [Parameter(Mandatory = $true)][string]$DashboardExecutable,
    [Parameter(Mandatory = $true)][string]$EditorExecutable
)

$ErrorActionPreference = "Stop"
$probeRoot = Join-Path $env:TEMP ("tfm2-atlas-portable-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $probeRoot | Out-Null

try {
    $cases = @(
        @{ Name = "DASHBOARD"; Source = $DashboardExecutable; File = "TFM2.Atlas.Dashboard.exe"; Title = "TFM2 Atlas Dashboard" },
        @{ Name = "EDITOR"; Source = $EditorExecutable; File = "TFM2.Atlas.Editor.exe"; Title = "TFM2 Atlas Editor" }
    )
    foreach ($case in $cases) {
        $directory = New-Item -ItemType Directory -Force -Path (Join-Path $probeRoot $case.Name)
        $executable = Join-Path $directory $case.File
        Copy-Item -LiteralPath $case.Source -Destination $executable
        $startedAt = Get-Date
        $launcher = Start-Process -FilePath $executable -ArgumentList "--user-data-dir=$(Join-Path $directory 'UserData')" -PassThru
        try {
            $window = $null
            for ($attempt = 0; $attempt -lt 40; $attempt++) {
                Start-Sleep -Milliseconds 500
                $window = Get-Process | Where-Object { $_.StartTime -ge $startedAt -and $_.MainWindowTitle -eq $case.Title } | Select-Object -First 1
                if ($window) { break }
            }
            if (-not $window) { throw "Portable $($case.Name) window did not appear." }
            Write-Output "PORTABLE_$($case.Name)_OK|$($window.Id)|$($window.MainWindowTitle)|$executable"
        }
        finally {
            Get-Process | Where-Object { $_.StartTime -ge $startedAt -and ($_.MainWindowTitle -eq $case.Title -or $_.Id -eq $launcher.Id) } |
                Stop-Process -Force -ErrorAction SilentlyContinue
        }
    }
}
finally {
    if ($probeRoot.StartsWith($env:TEMP, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $probeRoot).StartsWith("tfm2-atlas-portable-")) {
        Remove-Item -LiteralPath $probeRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
