CREATE SEQUENCE  participations_id_seq START WITH 0 MINVALUE 0;

CREATE TABLE participations
(
    id unsigned_integer DEFAULT nextval('participations_id_seq') UNIQUE PRIMARY KEY NOT NULL,
    user_id  UUID REFERENCES users(id) NOT NULL,
    contest_id VARCHAR NOT NULL,
    phase SMALLINT NOT NULL,
    virtual_contest_start_time TIMESTAMP
);

CREATE INDEX participation_lookup_index ON participations (user_id, contest_id);

UPDATE __revision SET revision = '2020-04-03-210736_contest_registration';
