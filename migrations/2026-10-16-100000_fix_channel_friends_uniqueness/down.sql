-- Rollback must fail transactionally rather than delete incompatible memberships.
CREATE UNIQUE INDEX index_channel_friends_channel_id ON channel_friends (channel_id);
DROP INDEX index_channel_friends_user_id;
CREATE UNIQUE INDEX index_channel_friends_user_id ON channel_friends (user_id);
DROP INDEX index_channel_friends_channel_user;
