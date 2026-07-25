#!/usr/bin/env pwsh
# Runs the Network Observatory web dashboard in dev mode.
$root = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $root "web")
npm install
npm run dev
