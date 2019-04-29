#! /usr/bin/env bash
set -e
cd devtool
cargo run -- pkg
cargo run -- publish