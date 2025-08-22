use crate::db::error::DbResult;
use crate::db::lobbies::Lobby;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

#[allow(dead_code)]
pub fn find_lobby_by_uuid(conn: &mut PgConnection, lobby_uuid: String) -> DbResult<Lobby> {
    use crate::db::schema::lobbies::dsl::lobbies;
    use crate::db::schema::lobbies::uuid;
    let lobby = lobbies
        .filter(uuid.eq(lobby_uuid.to_owned()))
        .select(Lobby::as_select())
        .first(conn)?;

    Ok(lobby)
}

#[allow(dead_code)]
pub fn find_lobby_by_channel_id(conn: &mut PgConnection, channel: i32) -> DbResult<Lobby> {
    use crate::db::schema::lobbies::channel_id;
    use crate::db::schema::lobbies::dsl::lobbies;
    let lobby = lobbies
        .filter(channel_id.eq(channel))
        .select(Lobby::as_select())
        .first(conn)?;

    Ok(lobby)
}
