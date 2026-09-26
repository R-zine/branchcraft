param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\.."))
)

$ErrorActionPreference = "Stop"
$bundleDirectory = Join-Path $RepositoryRoot "target\release\bundle\deb"
$packages = @(Get-ChildItem -LiteralPath $bundleDirectory -Filter "*.deb" -File)

if ($packages.Count -ne 1) {
    throw "Expected exactly one Debian package in $bundleDirectory. Build it before preparing Flatpak."
}

Copy-Item -LiteralPath $packages[0].FullName -Destination (Join-Path $PSScriptRoot "branchcraft.deb") -Force
Write-Output "Prepared packaging/flatpak/branchcraft.deb"
