-- indicies
DROP INDEX submissions_id_unique_index;
DROP INDEX submissions_state_index;
-- tables
DROP TABLE submissions;
-- sequences
DROP SEQUENCE submission_id_seq;
-- types
DROP TYPE submission_state;
DROP DOMAIN unsigned_integer;
