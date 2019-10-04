#!/usr/bin/env bash

env | sed "s/'/'\"'\"'/g" | sed "s/=/='/" | sed "s/$/'/g" | sed 's/^/export /g' > tmp-env.txt 
sudo bash -c '. tmp-env.txt; rm tmp-env.txt; strace -f -o >(python3 ../src/soft/strace-parser.py | RUST_BACKTRACE=1 cargo run -p soft -- --dest /dev/stdout --format text --data /dev/stdin --skip /dev --skip "$(pwd)" | tail +3) busybox ping -c 1 localhost >/dev/null 2>&1'
sleep 5
echo
