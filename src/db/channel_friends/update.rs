use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::channel_friends)]
pub struct ChannelFriendUpdate {
    pub active: bool,
    pub accepted: bool,
}

#[allow(dead_code)]
pub fn update_channel_friend(
    conn: &mut PgConnection,
    user: i32,
    channel: i32,
    active: bool,
    accepted: bool,
) -> DbResult<()> {
    let friend = ChannelFriendUpdate { active, accepted };
    use crate::db::schema::channel_friends::dsl::channel_friends;
    use crate::db::schema::channel_friends::dsl::channel_id;
    use crate::db::schema::channel_friends::dsl::user_id;
    diesel::update(
        channel_friends
            .filter(user_id.eq(user))
            .filter(channel_id.eq(channel)),
    )
    .set::<ChannelFriendUpdate>(friend)
    .execute(conn)?;
    Ok(())
}
