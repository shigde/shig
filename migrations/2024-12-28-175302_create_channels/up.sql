CREATE TABLE channels
(
    id          INTEGER        NOT NULL PRIMARY KEY,
    user_id     INTEGER UNIQUE NOT NULL,
    actor_id    INTEGER UNIQUE NOT NULL,
    name        VARCHAR UNIQUE NOT NULL,
    description TEXT,
    support     TEXT,
    public      BOOLEAN        NOT NULL DEFAULT 0,
    created_at  TIMESTAMP      NOT NULL,
    updated_at  TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES actors (id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_channels_user_id ON channels (user_id);
CREATE INDEX index_channels_name ON channels (name);
