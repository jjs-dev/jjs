#!/usr/bin/env powershell
param(
    [switch]$Touch
)
$env:RUST_BACKTRACE="1"
cargo fmt --verbose --all -- --check
if ($Touch) {
    cargo run -p devtool -- touch --verbose
}
cargo clippy --all -- -D clippy::all -D warnings
New-Item -Path ./minion-ffi/example-c/cmake-build-debug -ItemType Directory -ErrorAction SilentlyContinue
Set-Location ./minion-ffi/example-c/cmake-build-debug
cmake ..
cmake --build .