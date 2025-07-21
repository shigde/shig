CREATE TABLE lobbies
(
    id         INTEGER             NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uuid       VARCHAR(255) UNIQUE NOT NULL,
    user_id    INTEGER             NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    channel_id INTEGER UNIQUE      NOT NULL
        REFERENCES channels (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    stream_id  INTEGER REFERENCES streams (id),
    secret     VARCHAR(45)         NOT NULL,
    is_open    BOOLEAN             NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP           NOT NULL,
    updated_at TIMESTAMP           NOT NULL DEFAULT now()
);

CREATE INDEX index_lobbies_uuid ON lobbies (uuid);
CREATE UNIQUE INDEX index_lobbies_stream_id ON lobbies (stream_id);
