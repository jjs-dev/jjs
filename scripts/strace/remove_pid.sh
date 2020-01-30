#!/bin/bash

if [ "x$2" == x ] || [ "x$3" != x ]
then echo 'usage: remove_pid.sh <strace_log> <pid>' >&1
else grep -v "^$2[^0-9]" "$1"
fi
