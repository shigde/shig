use crate::db::channels::Channel;
use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, RunQueryDsl, SelectableHelper, PgConnection};

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::channels)]
pub struct NewChannel<'a> {
    pub user_id: i32,
    pub actor_id: i32,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub support: Option<&'a str>,
    pub public: bool,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_channel(
    conn: &mut PgConnection,
    name: &str,
    user_id: i32,
    actor_id: i32,
) -> DbResult<Channel> {
    let new_user = NewChannel {
        user_id,
        actor_id,
        name,
        description: None,
        support: None,
        public: false,
        created_at: Utc::now().naive_utc(),
    };

    use crate::db::schema::channels::dsl::channels;
    let chan = diesel::insert_into(channels)
        .values(&new_user)
        .returning(Channel::as_returning())
        .get_result::<Channel>(conn)
        .map_err(|e| -> String { format!("create channel: {}", e) })?;

    Ok(chan)
}
