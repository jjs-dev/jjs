-- indicies
DROP INDEX submissions_id_unique_index;
-- tables
DROP TABLE invokation_requests;
DROP TABLE submissions;
DROP TABLE users;
-- sequences
DROP SEQUENCE user_id_seq;
DROP SEQUENCE submission_id_seq;
DROP SEQUENCE inv_req_id_seq;
-- types
DROP TYPE submission_state;
DROP DOMAIN unsigned_integer;
