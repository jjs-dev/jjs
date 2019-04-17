phony:
    @just -l

db:
    #! /bin/bash
    echo "recreating db"
    dropdb jjs
    createdb jjs
    echo "running migrations"
    cd db
    diesel migration run
    echo "re-running migrations"
    diesel migration redo

sysroot:
    sh -c "rm -rf /tmp/jjs || true"
    mkdir /tmp/jjs
    @cargo run --bin init-jjs-root -- /tmp/jjs ./example-config
    @cargo run --bin soft -- --root=/tmp/jjs/opt --bin=python3 --bin=bash --bin=busybox
    pwsh ./soft/gcc.ps1

install_tools:
    #! /bin/bash
    cargo install diesel_cli mdbook || true

package:
    cargo run --bin devtool -- Pkg

lint:
    bash ./scripts/ci.sh