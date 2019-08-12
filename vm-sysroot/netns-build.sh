#!/bin/bash

env | sed "s/'/'\"'\"'/g" | sed "s/=/='/" | sed "s/$/'/g" | sed 's/^/export /g' > netns-env.txt 
sudo ip netns add jjs-build-netns
sudo ip netns exec jjs-build-netns sudo -u "$(whoami)" bash -c '. netns-env.txt; rm netns-env.txt; ./build.sh "$0"' "$1"
