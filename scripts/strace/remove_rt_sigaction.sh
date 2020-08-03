#!/usr/bin/env bash

if [ "x$1" == x ] || [ "x$1" == x--help ] || [ "x$1" == x-h ]
then echo "usage: $0 <strace_log>"
else grep -v '^[^"]*rt_sigaction' "$1"
fi
