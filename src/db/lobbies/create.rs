use crate::db::error::DbResult;
use crate::db::lobbies::Lobby;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::lobbies)]
pub struct NewLobby<'a> {
    pub uuid: &'a str,
    pub user_id: i32,
    pub channel_id: i32,
    pub stream_id: Option<i32>,
    pub is_open: bool,
    pub secret: &'a str,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_lobby(
    conn: &mut PgConnection,
    user_id: i32,
    channel_id: i32,
    stream_id: Option<i32>,
    is_open: bool,
) -> DbResult<Lobby> {
    let uuid = Uuid::new_v4().to_string();
    let secret = Uuid::new_v4().to_string();
    let new_lobby = NewLobby {
        uuid: &uuid,
        user_id,
        channel_id,
        stream_id,
        is_open,
        secret: &secret,
        created_at: Utc::now().naive_utc().clone(),
    };

    use crate::db::schema::lobbies::dsl::lobbies;
    let lobby = diesel::insert_into(lobbies)
        .values(&new_lobby)
        .returning(Lobby::as_returning())
        .get_result::<Lobby>(conn)
        .map_err(|e| -> String { format!("create stream: {}", e) })?;
    Ok(lobby)
}
