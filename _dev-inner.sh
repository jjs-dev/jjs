#!/usr/bin/env bash
OUT=/out
ulimit -c unlimited
echo ${OUT}/core_%P > /proc/sys/kernel/core_pattern
export RUST_BACKTRACE=1
# timeout --foreground --signal=SIGKILL 10 \
 #gdb --quiet  --args \
 strace -f -o ${OUT}/strace.log  -s 64 \
    /jjs/target/x86_64-unknown-linux-musl/debug/minion-cli \
    run --dump-generated-security-settings --dump-argv --root /is $@
#/bin/bash