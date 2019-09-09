#!/bin/bash

set -e
shopt -s nullglob
rm jjs.deb || true

mkdir build
(
cd build

tar -xvf ../../target/jjs.tgz
mv jjs pkg

mkdir data
mkdir data/usr
mv pkg/bin data/usr/bin
mv pkg/lib data/usr/lib
mkdir -p data/usr/share/jjs
mv pkg/example-config data/usr/share/jjs
cd data; tar --owner=root -cvJf ../data.tar.xz .; cd ..

mkdir control
cp ../manifest.txt control/control
cp ../scripts/* control/ || true
cd control; tar --owner=root -cvJf ../control.tar.xz .; cd ..

echo '2.0' > debian-binary
ar -q ../jjs.deb debian-binary control.tar.xz data.tar.xz

) #cd build
rm -rf build
