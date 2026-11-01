use crate::db::error::DbResult;
use crate::db::recorded_video_previews::RecordedVideoPreview;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};

pub fn find_recorded_video_preview_by_uuid(
    conn: &mut PgConnection,
    needle_uuid: String,
    authenticated: bool,
) -> DbResult<RecordedVideoPreview> {
    use crate::db::schema_views::recorded_video_previews::dsl::recorded_video_previews;
    use crate::db::schema_views::recorded_video_previews::uuid;

    let mut query = recorded_video_previews.filter(uuid.eq(needle_uuid)).into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::recorded_video_previews::is_public.eq(true));
    }
    let stream = query.select(RecordedVideoPreview::as_select()).first(conn)?;

    Ok(stream)
}

pub fn find_all_recorded_video_previews(
    conn: &mut PgConnection,
    authenticated: bool,
) -> DbResult<Vec<RecordedVideoPreview>> {
    use crate::db::schema_views::recorded_video_previews::date;
    use crate::db::schema_views::recorded_video_previews::dsl::recorded_video_previews;

    let mut query = recorded_video_previews.into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::recorded_video_previews::is_public.eq(true));
    }
    let stream_list = query
        .select(RecordedVideoPreview::as_select())
        .order_by(date.desc())
        .load(conn)?;

    Ok(stream_list)
}

pub fn find_all_recorded_video_previews_by_channel(
    conn: &mut PgConnection,
    needle_uuid: String,
    authenticated: bool,
) -> DbResult<Vec<RecordedVideoPreview>> {
    use crate::db::schema_views::recorded_video_previews::channel_uuid;
    use crate::db::schema_views::recorded_video_previews::date;
    use crate::db::schema_views::recorded_video_previews::dsl::recorded_video_previews;

    let mut query = recorded_video_previews
        .filter(channel_uuid.eq(needle_uuid))
        .into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::recorded_video_previews::is_public.eq(true));
    }
    let stream_list = query
        .select(RecordedVideoPreview::as_select())
        .order_by(date.desc())
        .load(conn)?;

    Ok(stream_list)
}
