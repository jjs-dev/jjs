#!/bin/bash
$SUBST$
export CARGO_TARGET_DIR=$JJS_BUILD_DIR
cd $JJS_SRC_DIR
cargo run --package deploy --bin make -- "$@"