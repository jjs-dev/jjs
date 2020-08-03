#!/usr/bin/env bash

ldlinux="$(ldd /bin/bash | tail -1 | sed 's/^\s*//g' | sed 's/ (0x[0-9a-f]*)$//g')"
sudo mkdir -p "$SYSROOT/$ldlinux"
sudo rmdir "$SYSROOT/$ldlinux"
sudo cp "$ldlinux" "$SYSROOT/$ldlinux"
