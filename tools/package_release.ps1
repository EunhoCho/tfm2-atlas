param(
    [string]$CandidateName = "TFM2.Atlas.1.0.33"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$releaseRoot = Join-Path $root "release"
$stage = Join-Path $releaseRoot $CandidateName
$zip = Join-Path $releaseRoot "$CandidateName.zip"

if (Test-Path -LiteralPath $stage) { throw "Candidate directory already exists: $stage" }
if (Test-Path -LiteralPath $zip) { throw "Candidate ZIP already exists: $zip" }

$dashboardExe = Join-Path $root "desktop\dashboard\dist\TFM2.Atlas.Dashboard.1.0.33.exe"
$editorExe = Join-Path $root "desktop\editor\dist\TFM2.Atlas.Editor.1.0.33.exe"
$coreDll = Join-Path $root "atlas-core\target\release\tfm2_atlas_core.dll"
$clientDll = Join-Path $root "atlas-client-055\tfm2_atlas_client_055.dll"
$editorDll = Join-Path $root "atlas-editor\target\release\tfm2_atlas_editor.dll"

foreach ($file in @($dashboardExe, $editorExe, $coreDll, $clientDll, $editorDll)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required release artifact is missing: $file" }
}

function New-ModFolder([string]$Parent, [string]$Id, [string]$Dll, [string]$Manifest) {
    $folder = New-Item -ItemType Directory -Force -Path (Join-Path $Parent "mods\$Id")
    Copy-Item -LiteralPath $Dll -Destination (Join-Path $folder "$Id.dll")
    Copy-Item -LiteralPath $Manifest -Destination (Join-Path $folder "mod.mod_info")
}

$dashboard = New-Item -ItemType Directory -Force -Path (Join-Path $stage "Dashboard")
$editor = New-Item -ItemType Directory -Force -Path (Join-Path $stage "Editor")
New-Item -ItemType Directory -Force -Path (Join-Path $dashboard "mods") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $editor "mods") | Out-Null

Copy-Item -LiteralPath $dashboardExe -Destination (Join-Path $dashboard "TFM2.Atlas.Dashboard.exe")
Copy-Item -LiteralPath $editorExe -Destination (Join-Path $editor "TFM2.Atlas.Editor.exe")

New-ModFolder $dashboard "tfm2_atlas_core" $coreDll (Join-Path $root "atlas-core\mod.mod_info")
New-ModFolder $dashboard "tfm2_atlas_client_055" $clientDll (Join-Path $root "atlas-client-055\mod.mod_info")
New-ModFolder $editor "tfm2_atlas_core" $coreDll (Join-Path $root "atlas-core\mod.mod_info")
New-ModFolder $editor "tfm2_atlas_client_055" $clientDll (Join-Path $root "atlas-client-055\mod.mod_info")
New-ModFolder $editor "tfm2_atlas_editor" $editorDll (Join-Path $root "atlas-editor\mod.mod_info")

Copy-Item -LiteralPath (Join-Path $root "desktop\dashboard\README.md") -Destination $dashboard
Copy-Item -LiteralPath (Join-Path $root "desktop\dashboard\THIRD_PARTY_NOTICES.md") -Destination $dashboard
Copy-Item -LiteralPath (Join-Path $root "desktop\dashboard\UPSTREAM_NOTICE.md") -Destination $dashboard
Copy-Item -LiteralPath (Join-Path $root "desktop\dashboard\assets\OFL-Noto-Sans-KR.txt") -Destination $dashboard
Copy-Item -LiteralPath (Join-Path $root "desktop\editor\README.md") -Destination $editor
Copy-Item -LiteralPath (Join-Path $root "desktop\editor\THIRD_PARTY_NOTICES.md") -Destination $editor
Copy-Item -LiteralPath (Join-Path $root "desktop\editor\UPSTREAM_NOTICE.md") -Destination $editor
Copy-Item -LiteralPath (Join-Path $root "desktop\editor\assets\OFL-Noto-Sans-KR.txt") -Destination $editor
Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination (Join-Path $dashboard "LICENSE-ATLAS.txt")
Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination (Join-Path $editor "LICENSE-ATLAS.txt")

foreach ($product in @("dashboard", "editor")) {
    $destination = if ($product -eq "dashboard") { $dashboard } else { $editor }
    $unpacked = Join-Path $root "desktop\$product\dist\win-unpacked"
    Copy-Item -LiteralPath (Join-Path $unpacked "LICENSE.electron.txt") -Destination $destination
    Copy-Item -LiteralPath (Join-Path $unpacked "LICENSES.chromium.html") -Destination $destination
}

Copy-Item -LiteralPath (Join-Path $root "INSTALL.ko.md") -Destination $stage
Copy-Item -LiteralPath (Join-Path $root "INSTALL.en.md") -Destination $stage

$coreDashboardHash = (Get-FileHash -LiteralPath (Join-Path $dashboard "mods\tfm2_atlas_core\tfm2_atlas_core.dll") -Algorithm SHA256).Hash
$coreEditorHash = (Get-FileHash -LiteralPath (Join-Path $editor "mods\tfm2_atlas_core\tfm2_atlas_core.dll") -Algorithm SHA256).Hash
$clientDashboardHash = (Get-FileHash -LiteralPath (Join-Path $dashboard "mods\tfm2_atlas_client_055\tfm2_atlas_client_055.dll") -Algorithm SHA256).Hash
$clientEditorHash = (Get-FileHash -LiteralPath (Join-Path $editor "mods\tfm2_atlas_client_055\tfm2_atlas_client_055.dll") -Algorithm SHA256).Hash
if ($coreDashboardHash -ne $coreEditorHash -or $clientDashboardHash -ne $clientEditorHash) {
    throw "Common mod hashes differ between Dashboard and Editor."
}

$manifest = Get-ChildItem -LiteralPath $stage -Recurse -File |
    Sort-Object FullName |
    ForEach-Object {
        $relative = $_.FullName.Substring($stage.Length).TrimStart('\').Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $relative"
    }
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllLines((Join-Path $stage "MANIFEST.sha256"), $manifest, $utf8NoBom)

Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -CompressionLevel Optimal
Write-Output $stage
Write-Output $zip
