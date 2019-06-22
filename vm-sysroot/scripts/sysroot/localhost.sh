#!/usr/bin/env bash
sudo bash -c 'strace -f -o >(python3 ../soft/strace-parser.py | RUST_BACKTRACE=1 ../target/debug/soft --dest /dev/stdout --format text --data /dev/stdin --skip /dev --skip "$(pwd)" | tail +3) busybox ping -c 1 localhost >/dev/null 2>&1'
sleep 5
echo
