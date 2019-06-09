CREATE TYPE submission_state AS ENUM ('wait_invoke', 'invoke', 'done', 'error');
CREATE DOMAIN unsigned_integer AS INTEGER
  CHECK (VALUE >= 0);

CREATE SEQUENCE submission_id_seq START WITH 0 MINVALUE 0;

CREATE TABLE submissions
(
  id           unsigned_integer DEFAULT nextval('submission_id_seq') PRIMARY KEY NOT NULL,
  toolchain_id VARCHAR(100)                                                      NOT NULL,
  state        submission_state                                                  NOT NULL,
  status_code  VARCHAR(100)                                                      NOT NULL,
  status_kind  VARCHAR(100)                                                      NOT NULL
);

CREATE UNIQUE INDEX submissions_id_unique_index ON submissions (id);
CREATE INDEX submissions_state_index ON submissions (state); -- optimizes invoker queries

CREATE SEQUENCE user_id_seq START WITH 0 MINVALUE 0;

CREATE TABLE users
(
    id unsigned_integer DEFAULT nextval('user_id_seq') PRIMARY KEY NOT NULL,
    username VARCHAR(100) NOT NULL,
    password_hash VARCHAR(128) NOT NULL -- SHA3-512, in hex encoding
);
