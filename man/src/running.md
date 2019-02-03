# Running JJS

## Database

Some JJS components use database.
How to get it working:
- Install postgresql version 11
- Set POSTGRES_URL to connection URL for database (e.g. postgres://jjs:internal@localhost:5432/jjs)

## Inter-component dependencies

Frontend:
- expects invoker to be running

Webclient:
- depends on frontend