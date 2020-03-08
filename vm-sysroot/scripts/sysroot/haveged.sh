#!/usr/bin/env bash

{ strace -f -o /dev/fd/3 "$(sudo which haveged)" --help >&2; } 3>&1 | python3 strace/strace-parser.py | python3 strace/soft.py
