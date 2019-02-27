#!/usr/bin/env bash
set -e

export RUST_BACKTRACE=1
cargo fmt --verbose --all -- --check
cargo clippy --all -- -D clippy::all -D warnings