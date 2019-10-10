#!/usr/bin/env bash

bash src/deploy/deps.sh
wget -q -O - https://github.com/Kitware/CMake/releases/download/v3.15.2/cmake-3.15.2-Linux-x86_64.sh > /tmp/cmake.sh
sudo bash /tmp/cmake.sh --skip-license --prefix=/usr
sudo rm -rf /usr/local/bin/cmake # this is hack to make script use new cmake version
if [[ "x$SECRET_ENABLED" != "x" ]]; then
    wget -q -O - https://files.viva64.com/etc/pubkey.txt | sudo apt-key add -;
    sudo wget -O /etc/apt/sources.list.d/viva64.list https://files.viva64.com/etc/viva64.list;
    sudo apt-get update;
    sudo apt-get install -y pvs-studio;
    pvs-studio-analyzer credentials "$PVS_NAME" "$PVS_LICENSE_KEY";
fi
rustup component add clippy rustfmt