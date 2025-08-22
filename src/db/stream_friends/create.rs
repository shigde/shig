use crate::db::error::DbResult;
use crate::db::stream_friends::StreamFriend;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::stream_friends)]
pub struct NewStreamFriend {
    pub user_id: i32,
    pub stream_id: i32,
    pub friend_role_id: i32,
    pub created_at: NaiveDateTime,
}

#[allow(dead_code)]
pub fn insert_new_stream_friend(
    conn: &mut PgConnection,
    user_id: i32,
    stream_id: i32,
    friend_role_id: i32,
) -> DbResult<StreamFriend> {
    let new_friend = NewStreamFriend {
        user_id,
        stream_id,
        friend_role_id,
        created_at: Utc::now().naive_utc().clone(),
    };

    use crate::db::schema::stream_friends::dsl::stream_friends;
    let friend = diesel::insert_into(stream_friends)
        .values(&new_friend)
        .returning(StreamFriend::as_returning())
        .get_result::<StreamFriend>(conn)
        .map_err(|e| -> String { format!("create StreamFriend: {}", e) })?;
    Ok(friend)
}
