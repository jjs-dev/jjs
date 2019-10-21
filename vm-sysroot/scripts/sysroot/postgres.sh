#!/usr/bin/env bash

firstof ()
{
    echo -n "$1"
}

rm -rf tmp
mkdir tmp
"$(firstof /usr/lib/postgresql/*/bin/initdb)" tmp >&2
# shellcheck disable=SC2016
{ strace -f -o /dev/fd/3 busybox sh -c "$(firstof /usr/lib/postgresql/*/bin/postgres)"' -D "$(pwd)/tmp" -k "$(pwd)/tmp" & sleep 3; psql -h "$(pwd)/tmp" -c ""; kill %1' >&2; } 3>&1 | python3 strace/strace-parser.py | python3 strace/soft.py
rm -rf tmp
find /usr/lib/postgresql
firstof /var/lib/postgresql/*/main
echo /var/run/postgresql
