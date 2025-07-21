use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use crate::db::lobbies::Lobby;

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
