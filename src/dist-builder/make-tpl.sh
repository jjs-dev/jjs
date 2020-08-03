#!/usr/bin/env bash

__SUBST__
export CARGO_TARGET_DIR=$JJS_BUILD_DIR
cd "$JJS_SRC_DIR" || exit 1
cargo run --package deploy --bin make -- "$@"
