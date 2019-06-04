#!/bin/bash

if [ "x$2" != x ] || [ "x$1" == "x--help" ]
then cat >&2 << EOF
usage: ./build.sh [sysroot_path]

Build a jjs sysroot at \$sysroot_path (default = ./sysroot)
This script assumes that you have working sudo command. Don't run directly as root!
EOF
exit 1
fi

bash -c 'cd ../soft; cargo build'

SYSROOT="$1"

if [ "x$SYSROOT" == x ]
then SYSROOT=sysroot
fi

if [ "${SYSROOT:0:1}" != "/" ]
then SYSROOT="$(pwd)/$SYSROOT"
fi

export SYSROOT

sudo rm -rf "$SYSROOT" 2>&1
sudo mkdir "$SYSROOT" || exit 1

for i in scripts/sysroot/*
do bash "$i"
done | sort | uniq | tee /dev/stderr | while read path
do
    sudo mkdir -p "$SYSROOT/$path"
    if ! sudo test -d "$path"
    then
        sudo rmdir "$SYSROOT/$path"
        sudo cp "$path" "$SYSROOT/$path"
        sudo chown "$(whoami):$(whoami)" "$SYSROOT/$path"
    fi
done

for i in scripts/post-sysroot/*
do bash -x "$i"
done
