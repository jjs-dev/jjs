#!/usr/bin/env bash

firstof ()
{
    echo -n "$1"
}

rm -rf tmp
mkdir tmp
"$(firstof /usr/lib/postgresql/*/bin/initdb)" tmp >&2
timeout 5 strace -f -o >(python3 ../soft/strace-parser.py | RUST_BACKTRACE=1 ../target/debug/soft --dest /dev/stdout --format text --data /dev/stdin --skip /dev --skip "$(pwd)" | tail +3) busybox sh -c '/usr/lib/postgresql/*/bin/postgres -D "$(pwd)/tmp" -k "$(pwd)/tmp" & sleep 3; psql -h "$(pwd)/tmp" -c ""'
sleep 1
echo
rm -rf tmp
find /usr/lib/postgresql
echo "$(firstof /var/lib/postgresql/*/main)"
echo /var/run/postgresql
