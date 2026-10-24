use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection};
use serde::{Deserialize, Serialize};
use crate::db::schema::lobbies::stream_id;

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::lobbies)]
pub struct LobbyUpdate {
    pub stream_id: Option<i32>,
    pub is_open: bool,
}

pub fn update_lobby_online_state(
    conn: &mut PgConnection,
    lobby_uuid: &str,
    stream_uuid: &str,
) -> DbResult<()> {
    use diesel::prelude::*;
    use crate::db::schema::lobbies::dsl::{lobbies, uuid, is_open, stream_id};
    use crate::db::schema::streams::dsl as streams;

    let stream_db_id = streams::streams
        .filter(streams::uuid.eq(stream_uuid))
        .select(streams::id)
        .first::<i32>(conn)
        .optional()?
        .ok_or(format!(
            "Stream not found to set lobby online (lobby_uuid={}, stream_uuid={})",
            lobby_uuid, stream_uuid
        ))?;

    let affected = diesel::update(lobbies.filter(uuid.eq(lobby_uuid)))
        .set((
            is_open.eq(true),
            stream_id.eq(stream_db_id),
        ))
        .execute(conn)?;

    if affected == 0 {
        return Err(format!("Lobby not found tp set online, lobby_uuid={}", lobby_uuid).into());
    }

    Ok(())
}

pub fn update_lobby_offline_state(
    conn: &mut PgConnection,
    lobby_uuid: &str,
) -> DbResult<()> {
    use diesel::prelude::*;
    use crate::db::schema::lobbies::dsl::is_open;
    use crate::db::schema::lobbies::dsl::lobbies;
    use crate::db::schema::lobbies::dsl::uuid;

    conn.transaction(|conn| {
    // A subsequent host session must be able to generate a fresh start notification.
    diesel::sql_query("INSERT INTO notification_stream_state (stream_id) SELECT stream_id FROM lobbies WHERE uuid=$1 AND stream_id IS NOT NULL ON CONFLICT DO NOTHING")
        .bind::<diesel::sql_types::Text, _>(lobby_uuid).execute(conn)?;
    diesel::sql_query("UPDATE notification_stream_state SET live = false WHERE stream_id IN (SELECT stream_id FROM lobbies WHERE uuid = $1)")
        .bind::<diesel::sql_types::Text, _>(lobby_uuid).execute(conn)?;
    let affected =  diesel::update(lobbies.filter(uuid.eq(lobby_uuid)))
        .set((
            is_open.eq(false),
            stream_id.eq(None::<i32>),
        ))
        .execute(conn)?;

    if affected == 0 {
        return Err(format!("Lobby not found to set offline, lobby_uuid={}", lobby_uuid).into());
    }

    Ok(())
    })
}
