# Running JJS

## Database

Some JJS components use database.
How to get it working:
- Install PostgresQL version 11
- Set POSTGRES_URL to connection URI for database (e.g. `postgres://jjs:internal@localhost:5432/jjs`
means: host -> `localhost`, port -> `5432`, username -> `jjs`, password -> `internal`, database name -> `jjs`.
See Postgres docs for more details.)
- (Optional, recommended for serious usage) configure access rights for DB user used in previous step.

Note: You _can_ use different connection URI for different JJS instances, but they **must**  refer to same database