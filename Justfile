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

users:
    cargo run --bin userlist -- add   --auth dev_root  ./example-config/userlist.txt

problems:
    rm -rf /tmp/jjs/var/problems/*
    mkdir /tmp/jjs/var/problems/a-plus-b
    @cargo run --bin tt -- compile --pkg ./example-problems/a-plus-b --out /tmp/jjs/var/problems/a-plus-b
    mkdir /tmp/jjs/var/problems/array-sum
    @cargo run --bin tt -- compile --pkg ./example-problems/array-sum --out /tmp/jjs/var/problems/array-sum
    mkdir /tmp/jjs/var/problems/sqrt
    @cargo run --bin tt -- compile --pkg ./example-problems/sqrt --out /tmp/jjs/var/problems/sqrt

install_tools:
    #! /bin/bash
    cargo install diesel_cli mdbook || true
