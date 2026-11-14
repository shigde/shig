use crate::db::error::DbResult;
use diesel::{
    dsl::exists,
    prelude::*,
    sql_types::{BigInt, Bool, Integer},
    PgConnection,
};

#[derive(diesel::QueryableByName)]
struct LikeStats {
    #[diesel(sql_type = BigInt)]
    likes: i64,
    #[diesel(sql_type = Bool)]
    liked_by_me: bool,
}

pub fn stats_for_user(conn: &mut PgConnection, stream_id: i32, user_id: i32) -> DbResult<(i64, bool)> {
    let stats = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS likes,
                COALESCE(BOOL_OR(user_id = $2), FALSE) AS liked_by_me
         FROM stream_likes WHERE stream_id = $1",
    )
    .bind::<Integer, _>(stream_id)
    .bind::<Integer, _>(user_id)
    .get_result::<LikeStats>(conn)?;
    Ok((stats.likes, stats.liked_by_me))
}

pub fn exists_for_user(
    conn: &mut PgConnection,
    stream_id: i32,
    user_id: i32,
) -> DbResult<bool> {
    use crate::db::schema::stream_likes::dsl as likes;

    Ok(diesel::select(exists(
        likes::stream_likes
            .filter(likes::stream_id.eq(stream_id))
            .filter(likes::user_id.eq(user_id)),
    ))
    .get_result(conn)?)
}
