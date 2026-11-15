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

pub fn lock_open_lobby_for_stream(
    conn: &mut PgConnection,
    stream: i32,
    owner: i32,
) -> DbResult<Option<i32>> {
    use crate::db::schema::lobbies;

    Ok(lobbies::table
        .filter(lobbies::stream_id.eq(Some(stream)))
        .filter(lobbies::user_id.eq(owner))
        .filter(lobbies::is_open)
        .select(lobbies::id)
        .for_update()
        .first(conn)
        .optional()?)
}

pub fn is_open_for_stream(conn: &mut PgConnection, stream: i32) -> DbResult<bool> {
    use crate::db::schema::lobbies;

    let open = lobbies::table
        .filter(lobbies::stream_id.eq(Some(stream)))
        .filter(lobbies::is_open);
    Ok(diesel::select(diesel::dsl::exists(open)).get_result(conn)?)
}

pub fn find_open_lobby_for_stream(conn: &mut PgConnection, stream: i32) -> DbResult<Option<Lobby>> {
    use crate::db::schema::lobbies;

    Ok(lobbies::table
        .filter(lobbies::stream_id.eq(Some(stream)))
        .filter(lobbies::is_open)
        .select(Lobby::as_select())
        .first(conn)
        .optional()?)
}
