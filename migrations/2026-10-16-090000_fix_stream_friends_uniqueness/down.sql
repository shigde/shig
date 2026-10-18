-- Refuse rollback if existing memberships violate the old restrictions.
-- Diesel runs this transactionally; never delete memberships to force rollback.
CREATE UNIQUE INDEX index_stream_friends_stream_id ON stream_friends (stream_id);

DROP INDEX index_stream_friends_user_id;
CREATE UNIQUE INDEX index_stream_friends_user_id ON stream_friends (user_id);

DROP INDEX index_stream_friends_stream_user;
