CREATE TABLE streams
(
    id           INTEGER             NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uuid         VARCHAR(255) UNIQUE NOT NULL,
    user_id      INTEGER             NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    channel_id   INTEGER             NOT NULL
        REFERENCES channels (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    title        VARCHAR             NOT NULL,
    description  TEXT,
    support      TEXT,
    date         TIMESTAMP           NOT NULL,
    start_time   TIMESTAMP,
    end_time     TIMESTAMP,
    licence      INTEGER             NOT NULL,
    is_repeating BOOLEAN             NOT NULL DEFAULT FALSE,
    repeat       INTEGER,
    is_public    BOOLEAN             NOT NULL DEFAULT FALSE,
    is_live      BOOLEAN             NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMP           NOT NULL,
    updated_at   TIMESTAMP           NOT NULL DEFAULT now()
);

CREATE INDEX index_streams_user_id ON streams (user_id);
CREATE INDEX index_streams_channel_id ON streams (channel_id);
CREATE UNIQUE INDEX index_streams_uuid ON streams (uuid);
CREATE INDEX index_streams_title ON streams (title);-- Your SQL goes here

CREATE TABLE stream_meta_data
(
    id             INTEGER             NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    stream_id      INTEGER             NOT NULL
        REFERENCES streams (id)
            ON DELETE CASCADE ON UPDATE CASCADE,

    is_shig        BOOLEAN             NOT NULL DEFAULT TRUE,
    stream_key     VARCHAR(255) UNIQUE NOT NULL,
    url            VARCHAR(255) UNIQUE NOT NULL,
    protocol       INTEGER             NOT NULL,
    permanent_live BOOLEAN             NOT NULL DEFAULT FALSE,
    save_replay    BOOLEAN             NOT NULL DEFAULT FALSE,
    latency_mode   INTEGER             NOT NULL,
    created_at     TIMESTAMP           NOT NULL,
    updated_at     TIMESTAMP           NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_stream_meta_data ON stream_meta_data (stream_id);

CREATE TABLE stream_participants
(
    id         INTEGER   NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    stream_id  INTEGER   NOT NULL
        REFERENCES streams (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    user_id    INTEGER   NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_stream_participants_stream ON stream_participants (stream_id);
