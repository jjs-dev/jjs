#!/bin/bash

set -e

if [ "$$" != 1 ] && [ "x$1" != x ]
then cat >&2 << EOF
usage: image/build-image.sh

Build the disk image image/hdd.img, using sysroot in ./sysroot.
The resulting image is a single partition without any bootloader/kernel/whatsoever.
This script must be run as root in order to function properly.
EOF
exit 1
fi

cd "$(dirname "$0")"

dd if=/dev/null of=hdd.img
dd if=/dev/null of=hdd.img bs=1048576 seek=1024
mke2fs -d ../sysroot hdd.img

