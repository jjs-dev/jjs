/*
This script fills empty db with all necessary tables
*/

--submissions
CREATE TYPE submission_state AS ENUM ('wait_invoke', 'invoke', 'done', 'error');

CREATE TABLE submissions
(
  submission_id serial PRIMARY KEY NOT NULL,
  toolchain     varchar(100)       NOT NULL,
  state         submission_state   NOT NULL
);

CREATE UNIQUE INDEX submissions_submission_id_uindex ON submissions (submission_id);
CREATE INDEX submissions_state_index
  ON submissions (state);



COMMENT ON TABLE submissions IS 'Contains information on all submissions';
