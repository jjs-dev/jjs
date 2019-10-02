-- indicies
DROP INDEX runs_id_unique_index;
-- tables
DROP TABLE invocation_requests;
DROP TABLE runs;
DROP TABLE users;
-- sequences
DROP SEQUENCE user_id_seq;
DROP SEQUENCE run_id_seq;
DROP SEQUENCE inv_req_id_seq;
-- types
DROP DOMAIN unsigned_integer;
