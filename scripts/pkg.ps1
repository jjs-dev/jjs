#!/usr/bin/env powershell
Write-Host "building minion_cli"
cargo build --release --target x86_64-unknown-linux-musl --features human_panic
Set-Location ./pkg
mkdir minion_cli
Copy-Item -Path ../target/x86_64-unknown-linux-musl/release/minion_cli -Destination ./minion_cli/minion_cli
tar -cvzf ./minion_cli.tgz ./minion_cli
