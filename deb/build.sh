#!/bin/bash

set -e
shopt -s nullglob

DIRNAME="$(dirname "$0")"
BUILD="$DIRNAME/build"
JJS_TGZ="$DIRNAME/../target/jjs.tgz"
OUT="$DIRNAME/jjs.deb"

BUILD_="$BUILD"
JJS_TGZ_="$JJS_TGZ"
OUT_="$OUT"

usage ()
{
    cat >&2 << EOF
usage: $0 [options]

Build JJS .deb format package.
Options:

    --build-dir <path>
        Place temporary files at \`path\` (instead of $BUILD). Must NOT exist before the build.

    --archive-path <path>
        Use \`path\` as a path to JJS archive (instead of $JJS_TGZ)

    --out <out_path>
        Place the resulting package file at \`out_path\` (instead of $OUT)
EOF
    exit 1
}

while [ "x$1" != x ]
do
    if [ "x$1" == x--build-dir ]
    then
        BUILD_="$2"
        shift; shift
    elif [ "x$1" == x--archive-path ]
    then
        JJS_TGZ_="$2"
        shift; shift
    elif [ "x$1" == x--out ]
    then
        OUT_="$2"
        shift; shift
    else
        usage
    fi
done

abspath ()
{
    local x="$1"
    if [ "x${x:0:1}" != x/ ]
    then x="$(pwd)/$x"
    fi
    echo -n "$x"
}

DIRNAME="$(abspath "$DIRNAME")"
BUILD="$(abspath "$BUILD_")"
JJS_TGZ="$(abspath "$JJS_TGZ_")"
OUT="$(abspath "$OUT_")"

rm "$OUT" || true

mkdir "$BUILD"
(
cd "$BUILD"

tar -xvf "$JJS_TGZ"
mv jjs pkg

mkdir data
mkdir data/usr
mv pkg/bin data/usr/bin
mv pkg/lib data/usr/lib
mkdir -p data/usr/share/jjs
mv pkg/example-config data/usr/share/jjs
cd data; tar --owner=root -cvJf ../data.tar.xz .; cd ..

mkdir control
cp "$DIRNAME/manifest.txt" control/control
sed -i 's/^Version:.*$/Version: '"$(cat "$DIRNAME/Version.txt")"'/g' control/control
sed -i 's/^Architecture:.*$/Architecture: '"$(dpkg --print-architecture)"'/g' control/control
cp ../scripts/* control/ || true
cd control; tar --owner=root -cvJf ../control.tar.xz .; cd ..

echo '2.0' > debian-binary
ar -q "$OUT" debian-binary control.tar.xz data.tar.xz

) #cd build
rm -rf "$BUILD"
