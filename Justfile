reset_db:
    dropdb jjs
    createdb jjs
    psql jjs -a -f ./setup_db.sql