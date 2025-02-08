CREATE TABLE channels
(
    id          INTEGER        NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id     INTEGER UNIQUE NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    actor_id    INTEGER UNIQUE NOT NULL
        REFERENCES actors (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    name        VARCHAR UNIQUE NOT NULL,
    description TEXT,
    support     TEXT,
    public      BOOLEAN        NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP      NOT NULL,
    updated_at  TIMESTAMP      NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_channels_user_id ON channels (user_id);
CREATE UNIQUE INDEX index_channels_name ON channels (name);
