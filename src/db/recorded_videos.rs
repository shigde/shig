use crate::models::{
    error::ApiError,
    user::recorded_video::{RecordedVideo, RecordedVideoUpdate},
};
use diesel::{
    prelude::*,
    sql_types::{Bool, Integer, Nullable, Text},
};

const SELECT: &str = "SELECT v.*, 'video'::varchar AS content_type, u.user_uuid AS owner_uuid, c.uuid AS channel_uuid
    FROM recorded_videos v JOIN users u ON u.id=v.user_id JOIN channels c ON c.id=v.channel_id";

pub fn find(conn: &mut PgConnection, uuid: &str) -> Result<RecordedVideo, ApiError> {
    Ok(
        diesel::sql_query(format!("{SELECT} WHERE v.uuid=$1 AND u.active=true"))
            .bind::<Text, _>(uuid)
            .get_result(conn)
            .map_err(crate::db::error::DbError::from)?,
    )
}
pub fn list(
    conn: &mut PgConnection,
    stream: &str,
    owner: i32,
) -> Result<Vec<RecordedVideo>, ApiError> {
    Ok(diesel::sql_query(format!("{SELECT} WHERE v.source_stream_uuid=$1 AND v.user_id=$2 ORDER BY v.created_at DESC, v.id DESC"))
        .bind::<Text,_>(stream).bind::<Integer,_>(owner).load(conn).map_err(crate::db::error::DbError::from)?)
}
pub fn list_owned(conn: &mut PgConnection, owner: i32) -> Result<Vec<RecordedVideo>, ApiError> {
    Ok(diesel::sql_query(format!(
        "{SELECT} WHERE v.user_id=$1 ORDER BY v.created_at DESC, v.id DESC"
    ))
    .bind::<Integer, _>(owner)
    .load(conn)
    .map_err(crate::db::error::DbError::from)?)
}

/// Snapshot the source under a row lock. Ownership is checked before inserting anything.
pub fn create(
    conn: &mut PgConnection,
    stream: &str,
    owner: i32,
) -> Result<RecordedVideo, ApiError> {
    conn.transaction::<_, ApiError, _>(|conn| {
        let source = crate::db::schema::streams::table
            .filter(crate::db::schema::streams::uuid.eq(stream))
            .for_update().select(crate::db::streams::Stream::as_select())
            .first::<crate::db::streams::Stream>(conn).map_err(crate::db::error::DbError::from)?;
        if source.user_id != owner {
            return Err(ApiError::Forbidden { error_message: "Only the stream owner can create recordings".into() });
        }
        let uuid = uuid::Uuid::new_v4().to_string();
        diesel::sql_query("INSERT INTO recorded_videos (uuid, source_stream_id, source_stream_uuid, user_id, channel_id,
            title, description, support, thumbnail, licence, is_public)
            SELECT $1, s.id, s.uuid, s.user_id, s.channel_id, s.title, COALESCE(s.description,''), COALESCE(s.support,''),
                COALESCE(t.file_url,''), s.licence, s.is_public
            FROM streams s LEFT JOIN stream_thumbnails t ON t.stream_id=s.id WHERE s.uuid=$2")
            .bind::<Text,_>(&uuid).bind::<Text,_>(stream).execute(conn)?;
        find(conn, &uuid)
    })
}

pub fn update(
    conn: &mut PgConnection,
    uuid: &str,
    owner: i32,
    changes: &RecordedVideoUpdate,
) -> Result<RecordedVideo, ApiError> {
    changes.validate()?;
    conn.transaction::<_, ApiError, _>(|conn| {
        diesel::sql_query("SELECT id FROM recorded_videos WHERE uuid=$1 FOR UPDATE")
            .bind::<Text,_>(uuid).execute(conn)?;
        let current = find(conn, uuid)?;
        current.require_owner(owner)?;
        if current.status == "deleting" {
            return Err(ApiError::Conflict { error_message: "Replay is being permanently deleted".into() });
        }
        if changes.published == Some(true) && current.status != "ready" {
            return Err(ApiError::Conflict { error_message: "Finish processing the video before publishing".into() });
        }
        diesel::sql_query("UPDATE recorded_videos SET title=COALESCE($2,title), description=COALESCE($3,description),
            support=COALESCE($4,support), thumbnail=COALESCE($5,thumbnail), licence=COALESCE($6,licence),
            is_public=COALESCE($7,is_public), published_at=CASE WHEN $8::boolean IS NULL THEN published_at
                WHEN $8 THEN COALESCE(published_at,now()) ELSE NULL END WHERE uuid=$1")
            .bind::<Text,_>(uuid).bind::<Nullable<Text>,_>(&changes.title)
            .bind::<Nullable<Text>,_>(&changes.description).bind::<Nullable<Text>,_>(&changes.support)
            .bind::<Nullable<Text>,_>(&changes.thumbnail).bind::<Nullable<Integer>,_>(changes.licence)
            .bind::<Nullable<Bool>,_>(changes.is_public).bind::<Nullable<Bool>,_>(changes.published)
            .execute(conn)?;
        find(conn, uuid)
    })
}

pub fn recording_started(conn: &mut PgConnection, uuid: &str) -> Result<RecordedVideo, ApiError> {
    diesel::sql_query("UPDATE recorded_videos SET status='recording', started_at=now(), error=NULL WHERE uuid=$1 AND status='pending'")
        .bind::<Text,_>(uuid).execute(conn)?;
    find(conn, uuid)
}

/// Commit a tombstone before removing files. Concurrent edits cannot republish it,
/// and an interrupted filesystem deletion can be retried by its owner.
pub fn prepare_delete(
    conn: &mut PgConnection,
    uuid: &str,
    owner: i32,
) -> Result<RecordedVideo, ApiError> {
    conn.transaction::<_, ApiError, _>(|conn| {
        diesel::sql_query("SELECT id FROM recorded_videos WHERE uuid=$1 FOR UPDATE")
            .bind::<Text,_>(uuid).execute(conn)?;
        let video = find(conn, uuid)?;
        video.require_owner(owner)?;
        if matches!(video.status.as_str(), "recording" | "processing") {
            return Err(ApiError::Conflict { error_message: "Stop the recording before deleting it".into() });
        }
        diesel::sql_query("UPDATE recorded_videos SET status='deleting', published_at=NULL, error=NULL WHERE uuid=$1")
            .bind::<Text,_>(uuid).execute(conn)?;
        find(conn, uuid)
    })
}

pub fn deletion_failed(conn: &mut PgConnection, uuid: &str, error: &str) -> Result<(), ApiError> {
    diesel::sql_query("UPDATE recorded_videos SET error=$2 WHERE uuid=$1 AND status='deleting'")
        .bind::<Text, _>(uuid)
        .bind::<Text, _>(error)
        .execute(conn)?;
    Ok(())
}

pub fn finish_delete(conn: &mut PgConnection, uuid: &str, owner: i32) -> Result<(), ApiError> {
    diesel::sql_query(
        "DELETE FROM recorded_videos WHERE uuid=$1 AND user_id=$2 AND status='deleting'",
    )
    .bind::<Text, _>(uuid)
    .bind::<Integer, _>(owner)
    .execute(conn)?;
    Ok(())
}
pub fn recording_failed(
    conn: &mut PgConnection,
    uuid: &str,
    error: &str,
) -> Result<RecordedVideo, ApiError> {
    diesel::sql_query(
        "UPDATE recorded_videos SET status='failed', error=$2, published_at=NULL WHERE uuid=$1",
    )
    .bind::<Text, _>(uuid)
    .bind::<Text, _>(error)
    .execute(conn)?;
    find(conn, uuid)
}
pub fn recording_stopped(conn: &mut PgConnection, uuid: &str) -> Result<RecordedVideo, ApiError> {
    diesel::sql_query("UPDATE recorded_videos SET status='processing', ended_at=now() WHERE uuid=$1 AND status='recording'")
        .bind::<Text,_>(uuid).execute(conn)?;
    find(conn, uuid)
}

/// The HLS master playlist is written last, before a video becomes publishable.
pub fn recording_ready(
    conn: &mut PgConnection,
    uuid: &str,
    duration_ms: u64,
) -> Result<RecordedVideo, ApiError> {
    let duration = i64::try_from(duration_ms).map_err(|_| ApiError::InternalServerError {
        error_message: "Recording duration exceeds database range".into(),
    })?;
    diesel::sql_query(
        "UPDATE recorded_videos SET status='ready', archive_prefix=$2,
        duration_ms=$3, started_at=COALESCE(started_at,now()), ended_at=now(), error=NULL
        WHERE uuid=$1 AND status IN ('pending','recording','processing')",
    )
    .bind::<Text, _>(uuid)
    .bind::<Text, _>(format!("recordings/{uuid}"))
    .bind::<diesel::sql_types::BigInt, _>(duration)
    .execute(conn)?;
    find(conn, uuid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    #[test]
    #[ignore = "Requires SHIG_MIGRATION_TEST_DATABASE_URL pointing to a dedicated PostgreSQL test database"]
    fn snapshots_video_metadata_and_only_exposes_published_ready_recorded_videos() {
        let url = std::env::var("SHIG_MIGRATION_TEST_DATABASE_URL").unwrap();
        let mut conn = PgConnection::establish(&url).unwrap();
        conn.test_transaction::<_, ApiError, _>(|conn| {
            conn.batch_execute("CREATE SCHEMA video_model_test; SET LOCAL search_path TO video_model_test, public;")?;
            crate::db::run_migrations(conn).unwrap();
            conn.batch_execute("INSERT INTO actors (preferred_username, actor_type, actor_iri, created_at) VALUES
                ('owner', 'Person', 'https://test/owner', now()), ('channel', 'Group', 'https://test/channel', now());
                INSERT INTO users (name, email, user_uuid, user_role_id, password, active, actor_id, created_at) VALUES
                ('owner', 'owner@test', 'owner', 2, 'unused', true, 1, now());
                INSERT INTO channels (uuid,user_id,actor_id,name,created_at) VALUES ('channel',1,2,'Channel',now());
                INSERT INTO streams (uuid,user_id,channel_id,title,description,support,date,licence,is_public,created_at) VALUES
                ('source',1,1,'Original title','Description','Support',now(),1,true,now());")?;
            assert!(matches!(create(conn, "source", 999), Err(ApiError::Forbidden { .. })));
            let first = create(conn, "source", 1)?;
            let second = create(conn, "source", 1)?;
            assert_ne!(first.uuid, second.uuid);
            assert_eq!(first.source_stream_uuid, "source");
            assert_eq!(first.status, "pending");
            assert!(first.published_at.is_none());
            assert!(!first.is_visible_to(None));
            assert!(!first.is_visible_to(Some(999)));
            assert!(first.is_visible_to(Some(1)));
            conn.batch_execute("UPDATE streams SET title='Changed livestream title' WHERE uuid='source';")?;
            assert_eq!(find(conn, &first.uuid)?.title, "Original title");
            update(conn, &second.uuid, 1, &RecordedVideoUpdate { title: Some("Edited video".into()), ..Default::default() })?;
            assert_eq!(find(conn, &first.uuid)?.title, "Original title");
            assert_eq!(find(conn, &second.uuid)?.title, "Edited video");
            assert!(matches!(update(conn, &first.uuid, 999, &RecordedVideoUpdate::default()), Err(ApiError::Forbidden { .. })));
            assert!(matches!(update(conn, &first.uuid, 1, &RecordedVideoUpdate { published: Some(true), ..Default::default() }), Err(ApiError::Conflict { .. })));
            recording_started(conn, &first.uuid)?;
            assert!(conn.transaction::<_, ApiError, _>(|conn| recording_started(conn, &second.uuid)).is_err());
            recording_stopped(conn, &first.uuid)?;
            recording_started(conn, &second.uuid)?;
            let ready = recording_ready(conn, &first.uuid, 12000)?;
            assert_eq!(ready.status, "ready");
            assert_eq!(ready.duration_ms, 12000);
            assert_eq!(crate::db::stream_previews::read::find_all_stream_previews(conn, true)?.len(), 1);
            let published = update(conn, &first.uuid, 1, &RecordedVideoUpdate {published: Some(true), ..Default::default()})?;
            assert!(published.is_visible_to(None));
            let preview = crate::db::stream_previews::read::find_stream_preview_by_uuid(conn, first.uuid.clone(), false)?;
            assert_eq!(preview.content_type, "video");
            assert_eq!(preview.duration_ms, Some(12000));
            assert!(!preview.is_live);
            let video_previews = crate::db::recorded_video_previews::read::find_all_recorded_video_previews(conn, false)?;
            assert_eq!(video_previews.len(), 1);
            assert_eq!(video_previews[0].uuid, first.uuid);
            assert_eq!(crate::db::recorded_video_previews::read::find_all_recorded_video_previews_by_channel(conn, "channel".into(), false)?.len(), 1);
            assert!(crate::db::recorded_video_previews::read::find_all_recorded_video_previews_by_channel(conn, "other".into(), false)?.is_empty());
            update(conn, &first.uuid, 1, &RecordedVideoUpdate {is_public: Some(false), ..Default::default()})?;
            assert!(crate::db::stream_previews::read::find_stream_preview_by_uuid(conn, first.uuid.clone(), true).is_err());
            assert!(crate::db::recorded_video_previews::read::find_all_recorded_video_previews(conn, true)?.is_empty());
            update(conn, &first.uuid, 1, &RecordedVideoUpdate {published: Some(false), ..Default::default()})?;
            conn.batch_execute("DELETE FROM streams WHERE uuid='source';")?;
            assert_eq!(find(conn, &first.uuid)?.source_stream_uuid, "source");
            assert_eq!(list(conn, "source", 1)?.len(), 2);
            assert!(matches!(prepare_delete(conn, &first.uuid, 999), Err(ApiError::Forbidden { .. })));
            assert!(matches!(prepare_delete(conn, &second.uuid, 1), Err(ApiError::Conflict { .. })));
            let deleting = prepare_delete(conn, &first.uuid, 1)?;
            assert_eq!(deleting.status, "deleting");
            assert!(deleting.published_at.is_none());
            assert!(matches!(update(conn, &first.uuid, 1, &RecordedVideoUpdate {published: Some(true), ..Default::default()}), Err(ApiError::Conflict { .. })));
            prepare_delete(conn, &first.uuid, 1)?; // deletion resumes after a failed filesystem attempt
            finish_delete(conn, &first.uuid, 1)?;
            assert!(find(conn, &first.uuid).is_err());
            assert_eq!(list(conn, "source", 1)?.len(), 1);
            Ok(())
        });
    }
}
