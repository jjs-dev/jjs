#!/usr/bin/env bash
# invocation example: ./dev.sh
set -e
powershell ./dev-docker-image/build.ps1
cargo build --target=x86_64-unknown-linux-musl --package minion-cli
docker run \
--interactive \
--tty \
--rm \
--cpu-quota=60000 \
--privileged \
--volume=$(pwd):/jjs:ro \
--volume=$(pwd)/run/:/out \
--volume=/opt/strace:/strace:ro \
jjs-libminion-dev /bin/sh /jjs/_dev-inner.sh $@
