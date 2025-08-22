CREATE TABLE stream_friends
(
    id             INTEGER   NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    user_id        INTEGER   NOT NULL
        REFERENCES users (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    stream_id      INTEGER   NOT NULL
        REFERENCES streams (id)
            ON DELETE CASCADE ON UPDATE CASCADE,
    active         BOOLEAN   NOT NULL DEFAULT true,
    accepted       BOOLEAN   NOT NULL DEFAULT false,
    friend_role_id INTEGER   NOT NULL DEFAULT 1
        REFERENCES friend_roles (id),
    created_at     TIMESTAMP NOT NULL,
    updated_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX index_stream_friends_id ON stream_friends (id);
CREATE UNIQUE INDEX index_stream_friends_stream_id ON stream_friends (stream_id);-- Your SQL goes here
CREATE UNIQUE INDEX index_stream_friends_user_id ON stream_friends (user_id);-- Your SQL goes here
