#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Installs the test-signed NetworkObservatory NDIS filter driver for local
  development. MUST be run in an elevated (Administrator) PowerShell.

.DESCRIPTION
  1. Imports the dev test-signing certificate into LocalMachine\Root and
     LocalMachine\TrustedPublisher (required for Windows to accept a
     test-signed driver's signature at all).
  2. Enables test-signing mode via bcdedit (required for Windows to load
     ANY non-production-signed driver, even a trusted one) — this requires
     a REBOOT to take effect, which this script does not do automatically.
  3. Installs the driver via pnputil.

  Run build.ps1 first (unprivileged) to produce NetObsFilter.sys + the pfx.
#>
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    throw "This script must be run as Administrator. Re-launch PowerShell elevated and run it again."
}

$pfxPath = Join-Path $root "netobs-test-cert.pfx"
if (-not (Test-Path $pfxPath)) {
    throw "netobs-test-cert.pfx not found — run build.ps1 first."
}

Write-Host "Importing test-signing certificate into LocalMachine trust stores..."
$pwd = ConvertTo-SecureString -String "netobs-dev" -Force -AsPlainText
Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\LocalMachine\Root -Password $pwd | Out-Null
Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\LocalMachine\TrustedPublisher -Password $pwd | Out-Null

$testSigningState = (bcdedit /enum | Select-String "testsigning\s+Yes")
if (-not $testSigningState) {
    Write-Host "Enabling test-signing mode (bcdedit)..."
    bcdedit /set testsigning on | Out-Null
    Write-Warning "Test-signing just enabled. A REBOOT is required before the driver can load. Reboot, then re-run this script to finish installation, or run the 'pnputil' step manually afterward."
    exit 0
}

Write-Host "Installing driver via pnputil..."
pnputil /add-driver "$root\NetworkObservatory.inf" /install

Write-Host "Done. Check with: pnputil /enum-drivers | Select-String NetObs"
Write-Host "To attach it to an adapter, bind it via an adapter's Properties > Network dialog, or 'netcfg' scripting."
