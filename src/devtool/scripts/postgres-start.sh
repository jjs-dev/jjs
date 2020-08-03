#!/usr/bin/env bash

#/usr/lib/postgresql/10/bin/pg_ctl -D /var/lib/postgresql/10/main -l /tmp/pg.log start
createdb jjs
#psql -d jjs -a -f /usr/bin/jjs-db-init
psql -d jjs -a -f "$ORIG_CWD/pkg/share/db-setup.sql"
