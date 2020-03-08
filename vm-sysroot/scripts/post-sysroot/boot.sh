#!/usr/bin/env bash

kernel="$(find /boot/vmlinuz-* | sort -V | tail -1)"
kernel="${kernel#/boot/vmlinuz-}"

sudo mkdir "$SYSROOT/boot"
sudo mkdir "$SYSROOT/boot/tmp"

(
cd "$SYSROOT/boot" || exit 1
sudo KERNEL="$kernel" unshare -m bash -c '
mount --bind tmp /etc/initramfs-tools/scripts
mount --bind /dev/null /etc/crypttab
mount --bind /dev/null /etc/fstab
update-initramfs -b . -c -k "$KERNEL"
'
)

sudo rmdir "$SYSROOT/boot/tmp"
sudo mv "$SYSROOT/boot/initrd.img-$kernel" "$SYSROOT/boot/initrd.img"
sudo cp "/boot/vmlinuz-$kernel" "$SYSROOT/boot/vmlinuz"

sudo mkdir "$SYSROOT/boot/grub"
sudo tee "$SYSROOT/boot/grub/grub.cfg" << EOF
linux /boot/vmlinuz root=/dev/sda1
initrd /boot/initrd.img
boot
EOF
