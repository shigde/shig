use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::stream_friends)]
pub struct StreamFriendUpdate {
    pub active: bool,
    pub accepted: bool,
}

#[allow(dead_code)]
pub fn update_stream_friend(
    conn: &mut PgConnection,
    user: i32,
    stream: i32,
    active: bool,
    accepted: bool,
) -> DbResult<()> {
    let friend = StreamFriendUpdate { active, accepted };
    use crate::db::schema::stream_friends::dsl::stream_friends;
    use crate::db::schema::stream_friends::dsl::stream_id;
    use crate::db::schema::stream_friends::dsl::user_id;
    diesel::update(
        stream_friends
            .filter(user_id.eq(user))
            .filter(stream_id.eq(stream)),
    )
    .set::<StreamFriendUpdate>(friend)
    .execute(conn)?;
    Ok(())
}
