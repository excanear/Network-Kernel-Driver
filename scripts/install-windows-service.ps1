#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Installs network-observatoryd as a real Windows Service (Phase I).
.DESCRIPTION
  Requires an elevated (Administrator) PowerShell session. Builds a release
  binary if one doesn't exist yet, then registers it with the Service Control
  Manager via sc.exe, configured to auto-start.
#>
param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root "target\release\network-observatoryd.exe"

if ($Uninstall) {
    Write-Host "Stopping and removing service NetworkObservatoryService..."
    sc.exe stop NetworkObservatoryService | Out-Null
    sc.exe delete NetworkObservatoryService
    exit 0
}

if (-not (Test-Path $exe)) {
    Write-Host "Release binary not found, building..."
    Push-Location $root
    cargo build --release -p service
    Pop-Location
}

Write-Host "Installing NetworkObservatoryService -> $exe --service"
sc.exe create NetworkObservatoryService binPath= "`"$exe`" --service" start= auto DisplayName= "Network Observatory"
sc.exe description NetworkObservatoryService "Network interface observability platform (see docs/architecture.md)"
sc.exe start NetworkObservatoryService

Write-Host "Done. Check status with: sc.exe query NetworkObservatoryService"
