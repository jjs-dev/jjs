/*
This script fills empty db with all necessary tables, types and other schema info
*/

--submissions
CREATE TYPE submission_state AS ENUM ('wait_invoke', 'invoke', 'done', 'error');

CREATE TABLE submissions
(
    id SERIAL PRIMARY KEY NOT NULL,
    toolchain_id VARCHAR(100) NOT NULL,
    state  submission_state NOT NULL
);

CREATE UNIQUE INDEX submissions_submission_id_uindex ON submissions (submission_id);
CREATE INDEX submissions_state_index ON submissions (state); -- optimizes invoker queries

COMMENT ON TABLE submissions IS 'Contains information on all submissions';
