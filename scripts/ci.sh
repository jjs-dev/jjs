#!/usr/bin/env bash
set -e
cargo clippy -- -D clippy::all -D warnings \
    -A renamed-and-removed-lints #this option is workaround (see https://issues.apache.org/jira/browse/THRIFT-4764)
cd devtool
cargo run -- Pkg
cargo run -- Publish