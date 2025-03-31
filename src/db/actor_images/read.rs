use crate::db::actor_images::{ActorImage, ActorImageType};
use crate::db::error::DbResult;
use diesel::dsl::{exists, select};
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

pub fn find_actor_image_by_actor_id(
    conn: &mut PgConnection,
    actor: i32,
    actor_image_type: ActorImageType,
) -> DbResult<ActorImage> {
    use crate::db::schema::actor_images;
    use crate::db::schema::actor_images::dsl::*;
    let image = actor_images::table
        .filter(actor_id.eq(actor))
        .filter(image_type.eq(actor_image_type.value_as_str()))
        .select(ActorImage::as_select())
        .get_result(conn)?;

    Ok(image)
}

pub fn exists_actor_image(
    conn: &mut PgConnection,
    actor: i32,
    actor_image_type: ActorImageType,
) -> DbResult<bool> {
    use crate::db::schema::actor_images::dsl::*;
    let exists = select(exists(
        actor_images
            .filter(actor_id.eq(actor))
            .filter(image_type.eq(actor_image_type.value_as_str())),
    ))
    .get_result(conn)?;
    Ok(exists)
}
