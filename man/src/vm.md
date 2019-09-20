# JJS ready-to-use VM image

JJS supports generating ready-to-use VM images that run a specific version of JJS out of the box (vm-sysroot).

## Building JJS

To be usable with vm-sysroot, jjs must be built in the `target` directory.

```bash
mkdir -p target
cd target
touch .jjsbuild
../configure --enable-archive
make
```

## VM image build dependencies
* `postgres` (for inclusion in the image; must be stopped for the build process to work properly)
* `g++` (for inclusion in the image)
* `jjs` and its build dependencies  (see above)
* `user-mode-linux` (used for packing the filesystem image; not required if you pack the image yourself)

On Debian, all of these dependencies (except JJS) can be installed with a single command:

`sudo apt-get install postgresql g++ user-mode-linux`

## Building the VM image

All commands in this section are to be performed from the `vm-sysroot` directory.

There are 3 ways of building the VM image:
* `./build.sh` then `sudo image/build.sh`. This requires postgres to be stopped to work properly.
* `./netns-build.sh` then `sudo image/build.sh`. This resolves the port conflict by running the build in a separate network namespace.
* `./uml-build.sh`. Runs the build under UML. This is slower than the above options but enables non-sudoers to build JJS VM images. `./uml-build.sh` runs `image/build.sh` itself, there is no need to do it manually.

## Running the VM image

The resulting image in `vm-sysroot/image/hdd.img` is a raw disk image that contains a bootable JJS userspace but does not have any bootloaders/kernels.

This image can be run in QEMU:

`qemu-system-<arch> -m 4096 -kernel <kernel> -initrd <initrd> -append 'root=/dev/sda' -hda vm-sysroot/image/hdd.img`

Or in UML:

`linux.uml mem=4096M ubda=vm-sysroot/image/hdd.img root=/dev/ubda`

Or alternatively, you can install a proper bootloader to make the image standalone:

```bash
mount -o loop vm-sysroot/image/hdd.img /mnt
mkdir -p /mnt/boot/grub
cat > /mnt/boot/grub/grub.cfg << EOF
kernel /kernel root=/dev/sda
initrd /initrd.img
boot
EOF
grub-install --boot-directory /mnt/boot /dev/loop0
cp <kernel> /mnt/kernel
cp <initrd> /mnt/initrd.img
umount /mnt
```

## Running in chroot

If you don't want to run a full VM for JJS, you can run the generated sysroot image in chroot.

`chroot vm-sysroot/sysroot /init`

The `/init` script will start postgres and JJS automatically. Note: you may want to disable the network initialisation in `/init` if you run JJS this way.

(Note that `vm-sysroot/sysroot` is **NOT** generated if building with `uml-build.sh`. In that case you will have to mount `vm-sysroot/image/hdd.img` somewhere.)
