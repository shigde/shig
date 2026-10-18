-- A stream may have multiple guests, and a guest may join multiple streams.
-- Install pair uniqueness before removing the old one-to-one restrictions.
CREATE UNIQUE INDEX index_stream_friends_stream_user
    ON stream_friends (stream_id, user_id);

DROP INDEX index_stream_friends_stream_id;
DROP INDEX index_stream_friends_user_id;

-- The pair index already supports lookups by stream_id.
CREATE INDEX index_stream_friends_user_id ON stream_friends (user_id);
