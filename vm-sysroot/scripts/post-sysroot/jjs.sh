#!/usr/bin/env bash
sudo mkdir -p "$SYSROOT"/usr/{bin,lib}
sudo cp ../pkg/ar_data/bin/* "$SYSROOT/usr/bin"
sudo cp ../pkg/ar_data/lib/* "$SYSROOT/usr/lib"
