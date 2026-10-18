use crate::db::error::DbResult;
use crate::db::stream_previews::StreamPreview;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};

pub fn find_stream_preview_by_uuid(
    conn: &mut PgConnection,
    needle_uuid: String,
    authenticated: bool,
) -> DbResult<StreamPreview> {
    use crate::db::schema_views::stream_previews::dsl::stream_previews;
    use crate::db::schema_views::stream_previews::uuid;

    let mut query = stream_previews.filter(uuid.eq(needle_uuid)).into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::stream_previews::is_public.eq(true));
    }
    let stream = query.select(StreamPreview::as_select()).first(conn)?;

    Ok(stream)
}

pub fn find_all_stream_previews(
    conn: &mut PgConnection,
    authenticated: bool,
) -> DbResult<Vec<StreamPreview>> {
    use crate::db::schema_views::stream_previews::date;
    use crate::db::schema_views::stream_previews::dsl::stream_previews;

    let mut query = stream_previews.into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::stream_previews::is_public.eq(true));
    }
    let stream_list = query
        .select(StreamPreview::as_select())
        .order_by(date)
        .load(conn)?;

    Ok(stream_list)
}

pub fn find_all_stream_previews_by_channel(
    conn: &mut PgConnection,
    needle_uuid: String,
    authenticated: bool,
) -> DbResult<Vec<StreamPreview>> {
    use crate::db::schema_views::stream_previews::channel_uuid;
    use crate::db::schema_views::stream_previews::date;
    use crate::db::schema_views::stream_previews::dsl::stream_previews;

    let mut query = stream_previews
        .filter(channel_uuid.eq(needle_uuid))
        .into_boxed();
    if !authenticated {
        query = query.filter(crate::db::schema_views::stream_previews::is_public.eq(true));
    }
    let stream_list = query
        .select(StreamPreview::as_select())
        .order_by(date)
        .load(conn)?;

    Ok(stream_list)
}
