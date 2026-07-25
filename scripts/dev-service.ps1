#!/usr/bin/env pwsh
# Runs the Network Observatory service (network-observatoryd) from source.
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
cargo run -p service
