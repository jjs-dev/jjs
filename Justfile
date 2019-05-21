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

install_tools:
    #! /bin/bash
    cargo install diesel_cli mdbook || true

package:
    cargo run --bin devtool -- pkg

lint:
    powershell ./scripts/lint.ps1
