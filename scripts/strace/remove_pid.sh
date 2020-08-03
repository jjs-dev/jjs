#!/usr/bin/env bash

if [ "x$2" == x ] || [ "x$3" != x ]
then echo "usage: $0 <strace_log> <pid>"
else grep -v "^$2[^0-9]" "$1"
fi
