#!/usr/bin/env bash
cargo check --all
cargo clippy -- -D clippy::all