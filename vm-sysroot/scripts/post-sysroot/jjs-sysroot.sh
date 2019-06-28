#!/usr/bin/env bash
ORIG_CWD="$(pwd)"

sudo mkdir -p "$SYSROOT/var/lib/jjs"
sudo chown "$(whoami):$(whoami)" "$SYSROOT/var/lib/jjs"
cd ../init-jjs-root
cargo run -- "$SYSROOT/var/lib/jjs" ../pkg/ar_data/example-config

sudo mkdir "$SYSROOT/var/lib/jjs/var/problems"
cd ../tt
if [ -d "$ORIG_CWD/problems" ] && ! ls "$ORIG_CWD/problems" | cmp - /dev/null 2>/dev/null
then for i in "$ORIG_CWD"/problems/*
do
    out="$SYSROOT/var/lib/jjs/var/problems/$(basename "$i")"
    mkdir "$out"
    CMAKE_PREFIX_PATH="$ORIG_CWD/../pkg/ar_data/share/cmake"  CPLUS_INCLUDE_PATH="$ORIG_CWD/../pkg/ar_data/include" LIBRARY_PATH="$ORIG_CWD/../pkg/ar_data/lib" JJS_PATH="$ORIG_CWD/../pkg/ar_data" cargo run -- compile --pkg "$i" --out "$out"
    out=
done
fi

sudo chown -R 1:1 "$SYSROOT"/var/lib/jjs/*
sudo chown root:root "$SYSROOT/var/lib/jjs"
sudo chmod -R 0700 "$SYSROOT"/var/lib/jjs/*
sudo chmod 0755 "$SYSROOT"/var/lib/jjs/var{,/submissions}

sudo rm -rf "$SYSROOT/var/lib/jjs/opt"
sudo mkdir "$SYSROOT/var/lib/jjs/opt"
pwsh "$ORIG_CWD/invoker-sysroot.ps1" "$SYSROOT/var/lib/jjs/opt"
