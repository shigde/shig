use crate::db::channel_friends::ChannelFriend;
use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::channel_friends)]
pub struct NewChannelFriend {
    pub user_id: i32,
    pub channel_id: i32,
    pub friend_role_id: i32,
    pub created_at: NaiveDateTime,
}

#[allow(dead_code)]
pub fn insert_new_channel_friend(
    conn: &mut PgConnection,
    user_id: i32,
    channel_id: i32,
    friend_role_id: i32,
) -> DbResult<ChannelFriend> {
    let new_friend = NewChannelFriend {
        user_id,
        channel_id,
        friend_role_id,
        created_at: Utc::now().naive_utc().clone(),
    };

    use crate::db::schema::channel_friends::dsl::channel_friends;
    let friend = diesel::insert_into(channel_friends)
        .values(&new_friend)
        .returning(ChannelFriend::as_returning())
        .get_result::<ChannelFriend>(conn)
        .map_err(|e| -> String { format!("create ChannelFriend: {}", e) })?;
    Ok(friend)
}
