#!/usr/bin/env bash
ORIG_CWD="$(pwd)"

sudo mkdir -p "$SYSROOT/var/jjs"
sudo chown "$(whoami):$(whoami)" "$SYSROOT/var/jjs"
cargo run --offline -p setup -- --data-dir "$SYSROOT/var/jjs" --install-dir ../pkg/ar_data/ --setup-config

sudo mkdir "$SYSROOT/var/jjs/var/problems"
# shellcheck disable=SC2012
if [ -d "$ORIG_CWD/problems" ] && ! ls "$ORIG_CWD/problems" | cmp - /dev/null 2>/dev/null
then for i in "$ORIG_CWD"/problems/*
do
    out="$SYSROOT/var/jjs/var/problems/$(basename "$i")"
    mkdir "$out"
    CMAKE_PREFIX_PATH="$ORIG_CWD/../pkg/ar_data/share/cmake"  CPLUS_INCLUDE_PATH="$ORIG_CWD/../pkg/ar_data/include" LIBRARY_PATH="$ORIG_CWD/../pkg/ar_data/lib" JJS_PATH="$ORIG_CWD/../pkg/ar_data" cargo run --offline -p ppc -- compile --pkg "$i" --out "$out"
    out=
done
fi

sudo rm -rf "$SYSROOT/var/jjs/opt"
#rm -rf tmp
cargo run --offline -p soft ../toolchains "$SYSROOT/var/jjs"
echo 'sandbox:x:179:179:sandbox:/:/bin/sh' > "$SYSROOT/var/jjs/opt/etc/passwd"
echo 'sandbox:x:179:' > "$SYSROOT/var/jjs/opt/etc/group"
#sudo mv tmp "$SYSROOT/var/jjs/opt"

sudo chown -R 1:1 "$SYSROOT"/var/jjs/*
sudo chown root:root "$SYSROOT/var/jjs"
sudo chmod -R 0700 "$SYSROOT"/var/jjs/*
sudo chmod 0755 "$SYSROOT"/var/jjs/var{,/submissions}
sudo chown -R root:root "$SYSROOT/var/jjs/opt"
sudo chmod -R 755 "$SYSROOT/var/jjs/opt"
