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
TAR_FLAGS="cvJf"
TAR_FILE="data.tar.xz"

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

    --uncompressed
        Do not compress the rootfs archive. Useful if packaging debug binaries.
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
    elif [ "x$1" == x--uncompressed ]
    then
        TAR_FLAGS="cvf"
        TAR_FILE="data.tar"
        shift
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

mkdir data
mkdir data/opt
mv jjs data/opt/jjs

mkdir -p data/lib/systemd/system
(
cd data/opt/jjs
for i in lib/systemd/system/*
do
    ln -s /opt/jjs/"$i" ../../"$i"
done
)

mkdir data/usr
mkdir data/usr/bin
(
cd data/opt/jjs
for i in bin/*
do cat > ../../usr/"$i" << EOF
#!/bin/sh

set -a
if [ -f /var/lib/jjs/etc/env.txt ]
then . /var/lib/jjs/etc/env.txt
else . /usr/share/jjs/env.txt
fi
set +a
exec /opt/jjs/$i "\$@"
EOF
chmod 755 ../../usr/"$i"
done
)
cp "$DIRNAME/jjs-oneclick" data/usr/bin

mkdir data/usr/lib
(
cd data/opt/jjs
for i in lib/*.{a,so}
do ln -s /opt/jjs/"$i" ../../usr/"$i"
done
)

mkdir data/usr/include
ln -s /opt/jjs/include/jjs data/usr/include/jjs

mkdir -p data/usr/share/jjs
ln -s /opt/jjs/example-config data/usr/share/jjs/example-config
ln -s /opt/jjs/example-problems data/usr/share/jjs/example-problems
ln -s /opt/jjs/share/db-setup.sql data/usr/share/jjs/db-setup.sql
cp "$DIRNAME/env.txt" data/opt/jjs
ln -s /opt/jjs/env.txt data/usr/share/jjs/env.txt
cd data; tar --owner=root "-$TAR_FLAGS" "../$TAR_FILE" .; cd ..

mkdir control
cp "$DIRNAME/manifest.txt" control/control
sed -i 's/^Version:.*$/Version: '"$(cat "$DIRNAME/../Version.txt")"'/g' control/control
sed -i 's/^Architecture:.*$/Architecture: '"$(dpkg --print-architecture)"'/g' control/control
cp "$DIRNAME"/scripts/* control/ || true
cd control; tar --owner=root -cvJf ../control.tar.xz .; cd ..

echo '2.0' > debian-binary
ar -q "$OUT" debian-binary control.tar.xz "$TAR_FILE"

) #cd build
rm -rf "$BUILD"
