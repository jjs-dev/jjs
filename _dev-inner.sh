#!/usr/bin/env bash
RUST_BACKTRACE=1 timeout --foreground --signal=SIGKILL 10 \
 /strace/bin/strace -f -o /out/strace.log \
 /jjs/target/x86_64-unknown-linux-musl/debug/minion_cli -r -d -p /is $@
/bin/bash