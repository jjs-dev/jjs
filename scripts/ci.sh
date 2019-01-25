#!/usr/bin/env bash
rustup component add clippy
#RUSTFLAGS="-D warnings" cargo check --all
cargo clippy -- -D clippy::all -D warnings