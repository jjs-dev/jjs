#!/usr/bin/env powershell
Write-Host "----> building minion-cli" -ForegroundColor Green
cargo build --bin minion_cli --release --target x86_64-unknown-linux-musl --features dist
Write-Host "----> building jjs-cleanup" -ForegroundColor Green
cargo build --bin cleanup --release --target x86_64-unknown-linux-musl
Write-Host "----> packaging" -ForegroundColor Green
Set-Location ./pkg
mkdir ar_data
Copy-Item -Path ../target/x86_64-unknown-linux-musl/release/minion_cli -Destination ./ar_data/minion_cli
mkdir cleanup
Copy-Item -Path ../target/x86_64-unknown-linux-musl/release/cleanup -Destination ./ar_data/cleanup
tar cvzf ./jjs.tgz ./ar_data/