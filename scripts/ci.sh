#!/usr/bin/env bash
rustup component add clippy
cargo check --all
cargo clippy -- -D clippy::all