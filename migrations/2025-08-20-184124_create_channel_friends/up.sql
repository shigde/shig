CREATE TABLE channel_friends
(
    id             INTEGER   NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id        INTEGER   NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    channel_id     INTEGER   NOT NULL
        REFERENCES channels (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    friend_role_id INTEGER   NOT NULL DEFAULT 1
        REFERENCES friend_roles (id),
    active         BOOLEAN   NOT NULL DEFAULT true,
    accepted       BOOLEAN   NOT NULL DEFAULT false,
    created_at     TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX index_channel_friends_id ON channel_friends (id);
CREATE UNIQUE INDEX index_channel_friends_user_id ON channel_friends (user_id);
CREATE UNIQUE INDEX index_channel_friends_channel_id ON channel_friends (channel_id);
