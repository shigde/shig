use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::lobbies)]
pub struct LobbyUpdate<'a>  {
    pub stream_id: Option<i32>,
    pub secret: &'a str,
    pub is_open: bool,
}

#[allow(dead_code)]
pub fn update_lobby(
    conn: &mut PgConnection,
    uuid_id: &str,
    stream_id: Option<i32>,
    secret:  &str,
    is_open: bool,
) -> DbResult<()> {
    let lobby = LobbyUpdate { stream_id, is_open, secret };
    use crate::db::schema::lobbies::dsl::lobbies;
    use crate::db::schema::lobbies::dsl::uuid;
    diesel::update(lobbies.filter(uuid.eq(uuid_id)))
        .set::<LobbyUpdate>(lobby)
        .execute(conn)?;
    Ok(())
}
