#!/usr/bin/env powershell
param(
    [switch]$Touch
)
Set-StrictMode -Version Latest
$env:RUST_BACKTRACE="1"
cargo fmt --verbose --all -- --check
if ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
}
if ($Touch) {
    cargo run -p devtool -- touch --verbose
}
cargo clippy --all -- -D clippy::all -D warnings
if ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
}
New-Item -Path ./minion-ffi/example-c/cmake-build-debug -ItemType Directory -ErrorAction SilentlyContinue
Set-Location ./minion-ffi/example-c/cmake-build-debug
cmake ..
if ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
} 
cmake --build .
if ($LASTEXITCODE -ne 0) {
    Exit $LASTEXITCODE
}