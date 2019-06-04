This is a tool for building jjs into a sysroot (e.g. as a VM disk image).
All scripts must be run from this directory.

Executable scripts:

./build.sh [sysroot_path]
Build a jjs sysroot at $sysroot_path (default = ./sysroot)
This script assumes that you have working sudo command. Don't run directly as root!

sudo image/build-image.sh
Build the disk image image/hdd.img, using sysroot in ./sysroot. Uses UML to isolate itself.
The resulting image is a single partition without any bootloader/kernel/whatsoever.

./uml-build.sh
Executes the two previous scripts, using UML to simulate root access. Doesn't require to be launched as root.

Environment variables during build:

SKIP_DEVTOOL_PKG=1
Not run the `cd ../devtool; cargo run -- pkg` command during the build. You'll have to run it manually.
This part of the build process is totally broken, so as of now you have to specify this flag.

Other files:

scripts/sysroot/*.sh
These scripts are executed by ./build.sh and should output a newline-separated list of host files to be included.

scripts/post-sysroot/*.sh
These scripts are executed by ./build.sh after the core sysroot has been built to make some finishing touches.

etc-network-interfaces.conf
This file will be placed at /etc/network/interfaces inside the sysroot. Modify to match your network configuration.
