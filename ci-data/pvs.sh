#!/usr/bin/env bash

if [[ "x$SECRET_ENABLED" != "x" ]]; then
    wget -q -O - https://files.viva64.com/etc/pubkey.txt | sudo apt-key add -;
    sudo wget -O /etc/apt/sources.list.d/viva64.list https://files.viva64.com/etc/viva64.list;
    sudo apt-get update;
    sudo apt-get install -y pvs-studio;
    pvs-studio-analyzer credentials "$PVS_NAME" "$PVS_LICENSE_KEY";
fi

