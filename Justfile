phony:
    @echo specify command

db_reset:
    dropdb jjs
    createdb jjs

db_refresh: db_reset
    #! /bin/bash
    cd db
    diesel migration run
    diesel migration redo

install_tools:
    cargo install diesel_cli mdbook