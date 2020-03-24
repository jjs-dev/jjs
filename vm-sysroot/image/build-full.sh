#!/bin/bash

set -e

usage ()
{
    cat >&2 << EOF
usage: image/build-full.sh [--out <out_path>] [--sysroot <sysroot_path>]

Build the disk image \$out_path (default: image/full.img), using sysroot in \$sysroot_path (default: ./sysroot).
Unlike image/build-image.sh, the resulting image is a ready-to-boot raw disk image.
This script must be run as root in order to function properly.
EOF
    exit 1
}

dir="$(dirname "$0")"
out="$dir/full.img"
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

hdd_img="$(mktemp)"

abspath ()
{
    local x="$1"
    if [ "${x:0:1}" == "/" ]
    then echo "$x"
    else echo "$(pwd)/$x"
    fi
}

dir="$(abspath "$dir")"
out="$(abspath "$out")"
sysroot="$(abspath "$sysroot")"

bash "$dir/build-image.sh" --out "$hdd_img" --sysroot "$sysroot"

dd if=/dev/null of="$out"
dd if=/dev/null of="$out" bs=1048576 seek=1025
fdisk "$out" << EOF
n
p
1


w
EOF
dd if=/dev/null of="$out" bs=1048576 seek=1

tmp_dir="$(mktemp -d)"
(
cd "$tmp_dir"
mkdir tmpfs #this is intentional, don't rename
mount -t tmpfs tmpfs tmpfs
cp /usr/lib/grub/i386-pc/boot.img tmpfs/
grub-mkimage -O i386-pc -o tmpfs/core.img -p '(hd0,1)/boot/grub' biosdisk part_msdos ext2 linux normal
echo "(hd0) $out" > tmpfs/dmap.txt
grub-bios-setup --device-map tmpfs/dmap.txt -d tmpfs -s "$out"
umount tmpfs
rmdir tmpfs
cat "$hdd_img" >> "$out"
)
rmdir "$tmp_dir"
