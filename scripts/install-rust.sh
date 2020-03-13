#!/usr/bin/env bash

# rustup toolchain install nightly -c clippy -c rustfmt
rustup default nightly-2020-03-12
rm rust-toolchain
rustup component add clippy rustfmt
