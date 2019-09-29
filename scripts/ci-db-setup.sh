#!/usr/bin/env bash
set -e

# TODO copy-pasted from vm-sysroot/scripts/post-sysroot/postgres.sh

psql -c "create role jjs with password 'internal';"
psql -c "alter role jjs with login;"
psql -c "create database jjs;"
psql -f /opt/jjs/share/db-setup.sql
psql -d jjs -c "grant all on all tables in schema public to jjs;"
psql -d jjs -c "grant all on all sequences in schema public to jjs;"