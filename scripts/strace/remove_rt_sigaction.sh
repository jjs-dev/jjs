#!/bin/bash

if [ "x$1" == x ] || [ "x$1" == x--help ] || [ "x$1" == x-h ]
then echo 'usage: remove_rt_sigaction.sh <strace_log>' >&1
else grep -v '^[^"]*rt_sigaction' "$1"
fi
