#!/bin/bash

set -e

if [ "$$" != 1 ] && [ "x$1" != x ]
then cat >&2 << EOF
usage: image/build-full.sh

Build the disk image image/full.img, using sysroot in ./sysroot.
Unlike image/build-image.sh, the resulting image is a ready-to-boot raw disk image.
This script must be run as root in order to function properly.
EOF
exit 1
fi

cd "$(dirname "$0")"
bash ./build-image.sh

dd if=/dev/null of=full.img
dd if=/dev/null of=full.img bs=1048576 seek=1025
fdisk full.img << EOF
n
p
1


w
EOF
dd if=/dev/null of=full.img bs=1048576 seek=1

mkdir tmpfs #this is intentional, don't rename
mount -t tmpfs tmpfs tmpfs
cp /usr/lib/grub/i386-pc/boot.img tmpfs/
grub-mkimage -O i386-pc -o tmpfs/core.img -p '(hd0,1)/boot/grub' biosdisk part_msdos ext2 linux normal
echo '(hd0) ./full.img' > tmpfs/dmap.txt
grub-bios-setup --device-map tmpfs/dmap.txt -d tmpfs -s full.img
umount tmpfs
rmdir tmpfs
cat hdd.img >> full.img

