/*
This script fills empty db with all necessary tables
*/

--submissions

CREATE TABLE submissions (
    submission_id serial PRIMARY KEY NOT NULL
);
CREATE UNIQUE INDEX submissions_submission_id_uindex ON submissions (submission_id);
COMMENT ON TABLE submissions IS 'Contains information on all submissions';
