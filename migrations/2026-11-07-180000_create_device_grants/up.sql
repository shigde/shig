CREATE TABLE device_grants (
    device_uuid UUID PRIMARY KEY REFERENCES devices(uuid) ON DELETE CASCADE,
    stream_id INTEGER NOT NULL REFERENCES streams(id) ON DELETE CASCADE,
    invite_hash TEXT UNIQUE,
    session_hash TEXT UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK ((invite_hash IS NULL) <> (session_hash IS NULL))
);

CREATE INDEX device_grants_stream_idx ON device_grants (stream_id);
