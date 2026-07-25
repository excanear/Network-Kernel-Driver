#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Compiles, links, and test-signs the NetworkObservatory NDIS filter driver
  directly with cl.exe/link.exe/signtool.exe from the WDK — no Visual Studio
  driver-project integration required (see docs/phase2-kernel-driver-design.md
  for why: the WDK's VS extension failed to install against this VS 17.14
  instance, so the build talks to the WDK headers/libs/signtool directly).
  Does NOT require elevation and does NOT touch test-signing/BCD/reboot.
#>
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

$vsWhere = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath = & $vsWhere -latest -products * -property installationPath
$vcVersionFile = Join-Path $vsPath "VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt"
$vcVersion = (Get-Content $vcVersionFile).Trim()
$vc = Join-Path $vsPath "VC\Tools\MSVC\$vcVersion"

$kitRoot = "C:\Program Files (x86)\Windows Kits\10"
$kitVersion = (Get-ChildItem "$kitRoot\Include" -Directory | Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' -and (Test-Path (Join-Path $_.FullName "km\ndis.h")) } | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1).Name
$kitInc = "$kitRoot\Include\$kitVersion"
$kitLib = "$kitRoot\Lib\$kitVersion"
$kitBin = "$kitRoot\bin\$kitVersion\x64"

$env:INCLUDE = "$kitInc\km\crt;$kitInc\km;$kitInc\shared;$vc\include"
$env:LIB = "$vc\lib\x64;$kitLib\km\x64"
$env:PATH = "$vc\bin\Hostx64\x64;$kitBin;$env:PATH"

Set-Location $root
New-Item -ItemType Directory -Force -Path "$root\build" | Out-Null

Write-Host "Compiling..."
$sources = "src\driver.c", "src\filter.c", "src\telemetry.c", "src\ioctl.c"
& cl.exe /kernel /c /W3 /WX- /Od /D_WIN64 /D_AMD64_ /DAMD64 /D_UNICODE /DUNICODE `
    /DNDIS630=1 /DNDIS_SUPPORT_NDIS630=1 /DNDIS_WDM=1 /Gz @sources
if ($LASTEXITCODE -ne 0) { throw "Compile failed" }

Write-Host "Linking..."
& link.exe /OUT:NetObsFilter.sys /DRIVER /SUBSYSTEM:NATIVE,10.0 /ENTRY:DriverEntry `
    /MACHINE:X64 /NODEFAULTLIB /OSVERSION:10.0 /VERSION:10.0 /DEBUG /PDB:NetObsFilter.pdb `
    driver.obj filter.obj telemetry.obj ioctl.obj ntoskrnl.lib ndis.lib bufferoverflowfastfailk.lib
if ($LASTEXITCODE -ne 0) { throw "Link failed" }

$certSubject = "CN=NetworkObservatoryTestCert"
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1
if (-not $cert) {
    Write-Host "Creating test-signing certificate..."
    $cert = New-SelfSignedCertificate -Type Custom -Subject $certSubject -KeyUsage DigitalSignature `
        -FriendlyName "NetworkObservatory Test Cert" -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
}

$pfxPath = Join-Path $root "netobs-test-cert.pfx"
if (-not (Test-Path $pfxPath)) {
    $pwd = ConvertTo-SecureString -String "netobs-dev" -Force -AsPlainText
    Export-PfxCertificate -Cert "Cert:\CurrentUser\My\$($cert.Thumbprint)" -FilePath $pfxPath -Password $pwd | Out-Null
}

Write-Host "Signing..."
& signtool.exe sign /sha1 $cert.Thumbprint /fd SHA256 NetObsFilter.sys
if ($LASTEXITCODE -ne 0) { throw "Signing failed" }

Write-Host "Done: $root\NetObsFilter.sys (test-signed, cert thumbprint $($cert.Thumbprint))"
Write-Host "Next: run install-test-driver.ps1 as Administrator to trust the cert, enable test-signing, and install."
