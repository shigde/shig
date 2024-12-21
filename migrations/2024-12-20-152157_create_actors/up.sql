create table actors
(
    id                 INTEGER        NOT NULL PRIMARY KEY AUTOINCREMENT,
    preferred_username VARCHAR        NOT NULL,
    actor_type         VARCHAR        NOT NULL,
    actor_iri          VARCHAR UNIQUE NOT NULL,
    public_key         TEXT,
    private_key        TEXT,
    following_iri      VARCHAR,
    followers_iri      VARCHAR,
    inbox_iri          VARCHAR,
    outbox_iri         VARCHAR,
    shared_inbox_iri   VARCHAR,
    server_id          INTEGER,
    remote_created_at  TIMESTAMP,
    created_at         TIMESTAMP      NOT NULL,
    updated_at         TIMESTAMP
);
