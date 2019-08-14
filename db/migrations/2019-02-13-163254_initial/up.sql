CREATE DOMAIN unsigned_integer AS INTEGER
    CHECK (VALUE >= 0);

CREATE SEQUENCE run_id_seq START WITH 0 MINVALUE 0;

CREATE TABLE runs
(
    id           unsigned_integer DEFAULT nextval('run_id_seq') PRIMARY KEY NOT NULL,
    toolchain_id VARCHAR(100)                                               NOT NULL,
    status_code  VARCHAR(100)                                               NOT NULL,
    status_kind  VARCHAR(100)                                               NOT NULL,
    problem_id   VARCHAR(100)                                               NOT NULL,
    score        INTEGER                                                    NOT NULL,
    rejudge_id   unsigned_integer                                           NOT NULL
);

CREATE UNIQUE INDEX runs_id_unique_index ON runs (id);

CREATE SEQUENCE user_id_seq START WITH 0 MINVALUE 0;

CREATE TABLE users
(
    id            UUID  UNIQUE PRIMARY KEY NOT NULL,
    username      VARCHAR(100) UNIQUE     NOT NULL,
    password_hash CHAR(128)               NOT NULL, -- SHA3-512, in hex encoding
    groups        TEXT[]                  NOT NULL
);

CREATE SEQUENCE inv_req_id_seq START WITH 0 MINVALUE 0;

CREATE table invocation_requests
(
    id              unsigned_integer DEFAULT nextval('inv_req_id_seq') UNIQUE PRIMARY KEY NOT NULL,
    run_id   unsigned_integer REFERENCES runs (id)                                 NOT NULL,
    invoke_revision unsigned_integer                                                      NOT NULL
);