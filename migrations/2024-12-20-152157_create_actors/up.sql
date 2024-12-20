create table actors
(
    id                 INTEGER     NOT NULL PRIMARY KEY AUTOINCREMENT,
    preferred_username TEXT        NOT NULL,
    actor_type         TEXT        NOT NULL,
    actor_iri          TEXT UNIQUE NOT NULL,
    public_key         TEXT,
    private_key        TEXT,
    following_iri      TEXT,
    followers_iri      TEXT,
    inbox_iri          TEXT,
    outbox_iri         TEXT,
    shared_inbox_iri   TEXT,
    server_id          INTEGER,
    remote_created_at  TIMESTAMP,
    created_at         TIMESTAMP NOT NULL,
    updated_at         TIMESTAMP
);
