CREATE TABLE instance_streaming_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    mode VARCHAR(16) NOT NULL DEFAULT 'request'
        CHECK (mode IN ('request', 'open', 'locked')),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

INSERT INTO instance_streaming_policy (id, mode) VALUES (1, 'request');

CREATE TABLE channel_streaming_permissions (
    channel_id INTEGER PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE ON UPDATE CASCADE,
    status VARCHAR(16) NOT NULL CHECK (status IN ('requested', 'allowed', 'blocked')),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
