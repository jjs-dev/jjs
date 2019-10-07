#!/bin/bash

packages="libpq-dev libssl-dev cmake"

if command -v sudo >/dev/null
then sudo=sudo
else sudo=
fi

if command -v apt >/dev/null
then apt='apt'
elif command -v apt-get >/dev/null
then apt='apt-get'
else echo "Warning: apt not found!
You're probably using a non-Debian-based distribution. To build JJS you must install the development packages of libpq and OpenSSL." >&2
fi

if [ "x$apt" != x ]
then $sudo $apt update
# shellcheck disable=SC2086
$sudo $apt install $packages
fi

cargo install cbindgen -Z install-upgrade
cargo install mdbook -Z install-upgrade
