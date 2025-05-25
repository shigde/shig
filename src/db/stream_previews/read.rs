use crate::db::error::DbResult;
use crate::db::stream_previews::StreamPreview;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};

pub fn find_stream_preview_by_uuid(
    conn: &mut PgConnection,
    needle_uuid: String,
) -> DbResult<StreamPreview> {
    use crate::db::schema_views::stream_previews::dsl::stream_previews;
    use crate::db::schema_views::stream_previews::uuid;

    let stream = stream_previews
        .filter(uuid.eq(needle_uuid))
        .select(StreamPreview::as_select())
        .first(conn)?;

    Ok(stream)
}

pub fn find_all_stream_previews(conn: &mut PgConnection) -> DbResult<Vec<StreamPreview>> {
    use crate::db::schema_views::stream_previews::date;
    use crate::db::schema_views::stream_previews::dsl::stream_previews;

    let stream_list = stream_previews
        .select(StreamPreview::as_select())
        .order_by(date)
        .load(conn)?;

    Ok(stream_list)
}
