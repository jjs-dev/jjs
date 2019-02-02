#!/usr/bin/env bash
set -e
rustup default nightly-2019-01-26
rustup component add clippy
rustup target add x86_64-unknown-linux-musl
export RUST_BACKTRACE=1
cargo clippy -- -D clippy::all -D warnings \
    -A renamed-and-removed-lints #this option is workaround (see https://issues.apache.org/jira/browse/THRIFT-4764)
cd devtool
cargo run -- Pkg
cargo run -- Publish