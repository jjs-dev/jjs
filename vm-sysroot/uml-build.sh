#!/usr/bin/env bash

set -e

if [ "$$" != 1 ] && [ "x$1" != x ]
then cat >&2 << EOF
usage: $0

Build a disk image at image/hdd.img, using scripts in the scripts/ directory.
See ./build.sh and image/build-image.sh for details.
This script doesn't require root access.
EOF
exit 1
fi

SELF="$0"

if [ "${SELF:0:1}" != / ]
then SELF="$(pwd)/$SELF"
fi

if [ "$$" != 1 ]
then
    env | sed "s/'/'\"'\"'/g" | sed "s/=/='/" | sed "s/$/'/g" | sed 's/^/export /g' > uml-env.txt 
    dd if=/dev/urandom of=random.bin bs=1024 count=1
    exec linux.uml mem=4096M root=/dev/root rw rootflags=/ rootfstype=hostfs init="$SELF" whoami="$(whoami)"
fi

mount -t proc proc /proc
ln -s /proc/self/fd /dev/fd
ln -s /proc/self/fd/0 /dev/stdin
ln -s /proc/self/fd/1 /dev/stdout
ln -s /proc/self/fd/2 /dev/stderr
mkdir -p /dev/pts
mount -t devpts devpts /dev/pts
mount -t tmpfs -o mode=777 tmpfs /tmp
mkdir -p /dev/shm
chmod 777 /dev/shm
hostname -F /etc/hostname

cd "$(dirname "$0")"

dd if=random.bin of=/dev/urandom
rm random.bin

WHOAMI="$(sed 's/.* whoami=//' /proc/cmdline | cut -d ' ' -f 1-1)"

cat > /dev/sudoers << EOF
Defaults	secure_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

$WHOAMI	ALL=(ALL:ALL) NOPASSWD:ALL
EOF

mount --bind /dev/sudoers /etc/sudoers

mkdir /dev/sysroot
chown "$WHOAMI:$WHOAMI" /dev/sysroot

su "$WHOAMI" -c 'script -c '"'"'. uml-env.txt; rm uml-env.txt; bash ./build.sh /dev/sysroot/sysroot'"'"

rm -rf sysroot || true
ln -s /dev/sysroot/sysroot sysroot

umount /etc/sudoers
umount -l /dev/pts
rm /dev/fd /dev/std{in,out,err}
umount -l /proc
umount -l /tmp
exec image/build-image.sh
