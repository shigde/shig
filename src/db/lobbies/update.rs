use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::lobbies)]
pub struct LobbyUpdate {
    pub stream_id: Option<i32>,
    pub is_open: bool,
}

#[allow(dead_code)]
pub fn update_lobby(
    conn: &mut PgConnection,
    uuid_id: &str,
    stream_id: Option<i32>,
    is_open: bool,
) -> DbResult<()> {
    let lobby = LobbyUpdate { stream_id, is_open };
    use crate::db::schema::lobbies::dsl::lobbies;
    use crate::db::schema::lobbies::dsl::uuid;
    diesel::update(lobbies.filter(uuid.eq(uuid_id)))
        .set::<LobbyUpdate>(lobby)
        .execute(conn)?;
    Ok(())
}

pub fn update_lobby_online_state(
    conn: &mut PgConnection,
    uuid_id: &str,
    is_open_state: bool,
) -> DbResult<()> {
    use crate::db::schema::lobbies::dsl::is_open;
    use crate::db::schema::lobbies::dsl::lobbies;
    use crate::db::schema::lobbies::dsl::uuid;

    diesel::update(lobbies.filter(uuid.eq(uuid_id)))
        .set(is_open.eq(is_open_state))
        .execute(conn)?;
    Ok(())
}
