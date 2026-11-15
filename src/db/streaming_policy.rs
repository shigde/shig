use crate::db::error::DbResult;
use crate::db::user_roles::Role;
use diesel::{prelude::*, sql_types::{Integer, Text}};

#[derive(diesel::QueryableByName)]
struct TextRow { #[diesel(sql_type = Text)] value: String }

#[derive(diesel::QueryableByName)]
struct ChannelRow {
    #[diesel(sql_type = Integer)] id: i32,
    #[diesel(sql_type = Integer)] user_id: i32,
}

pub struct ChannelChange {
    pub user_id: i32,
    pub old_status: Option<String>,
    pub lobby_uuids: Vec<String>,
}

pub fn mode(conn: &mut PgConnection) -> DbResult<String> {
    diesel::sql_query("SELECT mode AS value FROM instance_streaming_policy WHERE id=1")
        .get_result::<TextRow>(conn).map(|r| r.value).map_err(Into::into)
}

pub fn public_content_enabled(conn: &mut PgConnection) -> DbResult<bool> {
    #[derive(diesel::QueryableByName)]
    struct Enabled { #[diesel(sql_type = diesel::sql_types::Bool)] enabled: bool }
    diesel::sql_query("SELECT public_content_enabled AS enabled FROM instance_streaming_policy WHERE id=1")
        .get_result::<Enabled>(conn).map(|row| row.enabled).map_err(Into::into)
}

pub fn set_public_content_enabled(conn: &mut PgConnection, enabled: bool) -> DbResult<()> {
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        diesel::sql_query("UPDATE instance_streaming_policy SET public_content_enabled=$1, updated_at=now() WHERE id=1")
            .bind::<diesel::sql_types::Bool,_>(enabled).execute(conn)?;
        if !enabled {
            // Hide content from anonymous visitors without changing replay publication state.
            diesel::sql_query("UPDATE streams SET is_public=false, updated_at=now() WHERE is_public=true").execute(conn)?;
            diesel::sql_query("UPDATE recorded_videos SET is_public=false, updated_at=now() WHERE is_public=true").execute(conn)?;
        }
        Ok(())
    })?;
    Ok(())
}

pub fn channel_status(conn: &mut PgConnection, channel_id: i32) -> DbResult<Option<String>> {
    diesel::sql_query("SELECT status AS value FROM channel_streaming_permissions WHERE channel_id=$1")
        .bind::<Integer,_>(channel_id).get_result::<TextRow>(conn).optional()
        .map(|r| r.map(|row| row.value)).map_err(Into::into)
}

pub fn request(conn: &mut PgConnection, channel_id: i32) -> DbResult<bool> {
    let inserted = diesel::sql_query("INSERT INTO channel_streaming_permissions(channel_id,status) VALUES($1,'requested') ON CONFLICT(channel_id) DO NOTHING")
        .bind::<Integer,_>(channel_id).execute(conn)?;
    Ok(inserted > 0)
}

pub fn set_global(conn: &mut PgConnection, mode: &str) -> DbResult<Vec<String>> {
    conn.transaction::<Vec<String>, diesel::result::Error, _>(|conn| {
        diesel::sql_query("UPDATE instance_streaming_policy SET mode=$1, updated_at=now() WHERE id=1")
            .bind::<Text,_>(mode).execute(conn)?;
        if mode != "locked" { return Ok(Vec::new()); }
        diesel::sql_query("DELETE FROM channel_streaming_permissions WHERE status IN ('allowed','requested')").execute(conn)?;
        let lobbies = diesel::sql_query("SELECT l.uuid AS value FROM lobbies l JOIN users u ON u.id=l.user_id JOIN streams s ON s.id=l.stream_id WHERE s.is_live=true AND u.user_role_id <> $1")
            .bind::<Integer,_>(Role::Admin.val()).load::<TextRow>(conn)?;
        diesel::sql_query("UPDATE streams SET is_public=false, is_live=false, updated_at=now() WHERE user_id IN (SELECT id FROM users WHERE user_role_id <> $1)")
            .bind::<Integer,_>(Role::Admin.val()).execute(conn)?;
        diesel::sql_query("UPDATE notification_stream_state SET live=false WHERE stream_id IN (SELECT s.id FROM streams s JOIN users u ON u.id=s.user_id WHERE u.user_role_id <> $1)")
            .bind::<Integer,_>(Role::Admin.val()).execute(conn)?;
        diesel::sql_query("UPDATE recorded_videos SET is_public=false, published_at=NULL, updated_at=now() WHERE user_id IN (SELECT id FROM users WHERE user_role_id <> $1)")
            .bind::<Integer,_>(Role::Admin.val()).execute(conn)?;
        Ok(lobbies.into_iter().map(|row| row.value).collect())
    }).map_err(Into::into)
}

pub fn set_channel(conn: &mut PgConnection, uuid: &str, status: &str) -> DbResult<Option<ChannelChange>> {
    let channel = diesel::sql_query("SELECT id, user_id FROM channels WHERE uuid=$1")
        .bind::<Text,_>(uuid).get_result::<ChannelRow>(conn).optional()?;
    let Some(channel) = channel else { return Ok(None); };
    let (old_status, lobby_uuids) = conn.transaction::<(Option<String>, Vec<String>), diesel::result::Error, _>(|conn| {
        let old_status = diesel::sql_query("SELECT status AS value FROM channel_streaming_permissions WHERE channel_id=$1")
            .bind::<Integer,_>(channel.id).get_result::<TextRow>(conn).optional()?.map(|r| r.value);
        if status == "inherited" {
            diesel::sql_query("DELETE FROM channel_streaming_permissions WHERE channel_id=$1").bind::<Integer,_>(channel.id).execute(conn)?;
        } else {
            diesel::sql_query("INSERT INTO channel_streaming_permissions(channel_id,status) VALUES($1,$2) ON CONFLICT(channel_id) DO UPDATE SET status=EXCLUDED.status, updated_at=now()")
                .bind::<Integer,_>(channel.id).bind::<Text,_>(status).execute(conn)?;
        }
        if status != "blocked" { return Ok((old_status, Vec::new())); }
        let active = diesel::sql_query("SELECT l.uuid AS value FROM lobbies l JOIN streams s ON s.id=l.stream_id WHERE l.channel_id=$1 AND s.is_live=true")
            .bind::<Integer,_>(channel.id).load::<TextRow>(conn)?;
        diesel::sql_query("UPDATE streams SET is_public=false, is_live=false, updated_at=now() WHERE channel_id=$1").bind::<Integer,_>(channel.id).execute(conn)?;
        diesel::sql_query("UPDATE notification_stream_state SET live=false WHERE stream_id IN (SELECT id FROM streams WHERE channel_id=$1)").bind::<Integer,_>(channel.id).execute(conn)?;
        diesel::sql_query("UPDATE recorded_videos SET is_public=false, published_at=NULL, updated_at=now() WHERE channel_id=$1").bind::<Integer,_>(channel.id).execute(conn)?;
        Ok((old_status, active.into_iter().map(|row| row.value).collect()))
    })?;
    Ok(Some(ChannelChange { user_id: channel.user_id, old_status, lobby_uuids }))
}

