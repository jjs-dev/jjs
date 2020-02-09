-- This table contains temporary key-value pairs, when Redis usage is disabled
CREATE TABLE kv (
    name VARCHAR UNIQUE PRIMARY KEY NOT NULL,
    value bytea NOT NULL 
);
UPDATE __revision SET revision = '2020-04-01-171741_kv';