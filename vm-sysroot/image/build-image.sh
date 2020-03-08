#!/bin/bash

set -e

usage ()
{
    cat >&2 << EOF
usage: image/build-image.sh [--out <output_path>] [--sysroot <sysroot_path>]

Build the disk image \$output_path (default: image/hdd.img), using sysroot in \$sysroot_path (default: ./sysroot).
The resulting image is a single partition without any bootloader/kernel/whatsoever.
This script must be run as root in order to function properly.
EOF
    exit 1
}

dir="$(dirname "$0")"
out="$dir/hdd.img"
sysroot="$dir/../sysroot"

while [ "x$1" != x ]
do
    if [ "x$1" == x--out ]
    then
        out="$2"
        shift; shift
    elif [ "x$1" == x--sysroot ]
    then
        sysroot="$2"
        shift; shift
    else
        usage
    fi
done

dd if=/dev/null of="$out"
dd if=/dev/null of="$out" bs=1048576 seek=1024
mke2fs -d "$sysroot" "$out"

