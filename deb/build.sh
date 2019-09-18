#!/bin/bash

set -e
shopt -s nullglob
rm jjs.deb || true

mkdir build
<<<<<<< HEAD
(
=======
>>>>>>> a8b35ed... Add .deb build scripts
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
<<<<<<< HEAD
=======
sed -i 's/^Version:.*$/Version: '"$(cat ../../Version.txt)"'/g' /control/control
sed -i 's/^Architecture:.*$/Architecture: '"$(dpkg --print-architecture)"'/g' control/control
>>>>>>> a8b35ed... Add .deb build scripts
cp ../scripts/* control/ || true
cd control; tar --owner=root -cvJf ../control.tar.xz .; cd ..

echo '2.0' > debian-binary
ar -q ../jjs.deb debian-binary control.tar.xz data.tar.xz

<<<<<<< HEAD
) #cd build
=======
cd ..
>>>>>>> a8b35ed... Add .deb build scripts
rm -rf build
