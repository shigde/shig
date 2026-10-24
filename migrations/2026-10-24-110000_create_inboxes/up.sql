CREATE TABLE messages (
    id VARCHAR(36) PRIMARY KEY,
    recipient_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stream_id INTEGER REFERENCES streams(id) ON DELETE SET NULL,
    event_key TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ,
    UNIQUE (recipient_id, event_key)
);
CREATE INDEX messages_inbox ON messages (recipient_id, created_at DESC, id);
CREATE INDEX messages_unread ON messages (recipient_id) WHERE read_at IS NULL;
CREATE TABLE notifications (
    id VARCHAR(36) PRIMARY KEY,
    recipient_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stream_id INTEGER REFERENCES streams(id) ON DELETE SET NULL,
    event_key TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at TIMESTAMPTZ,
    UNIQUE (recipient_id, event_key)
);
CREATE INDEX notifications_inbox ON notifications (recipient_id, created_at DESC, id);
CREATE INDEX notifications_unread ON notifications (recipient_id) WHERE read_at IS NULL;
CREATE TABLE notification_stream_state (
    stream_id INTEGER PRIMARY KEY REFERENCES streams(id) ON DELETE CASCADE,
    live BOOLEAN NOT NULL DEFAULT FALSE
);
