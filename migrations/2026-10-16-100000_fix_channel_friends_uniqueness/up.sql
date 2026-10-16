CREATE UNIQUE INDEX index_channel_friends_channel_user ON channel_friends (channel_id, user_id);
DROP INDEX index_channel_friends_channel_id;
DROP INDEX index_channel_friends_user_id;
CREATE INDEX index_channel_friends_user_id ON channel_friends (user_id);
