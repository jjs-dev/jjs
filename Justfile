phony:
    @just -l

db_reset:
    dropdb jjs
    createdb jjs

db_refresh: db_reset
    #! /bin/bash
    cd db
    diesel migration run
    diesel migration redo

sysroot:
    sh -c "rm -rf /tmp/jjs || true"
    mkdir /tmp/jjs
    cargo run --bin init-jjs-root -- /tmp/jjs ./example-config
    cargo run --bin soft -- --root=/tmp/jjs/opt --with=python3 --with=gcc --with=g++ --with=bash

install_tools:
    cargo install diesel_cli mdbook