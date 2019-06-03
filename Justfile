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
    @cargo run --bin init-jjs-root -- /tmp/jjs ./example-config --symlink-config
    pwsh ./soft/example-linux.ps1

problems:
    mkdir /tmp/jjs/var/problems/TODO
    @cargo run --bin tt -- --pkg ./example-problems/a-plus-b --out /tmp/jjs/var/problems/TODO

install_tools:
    #! /bin/bash
    cargo install diesel_cli mdbook || true
