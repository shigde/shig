use diesel::{Insertable, PgConnection, RunQueryDsl};
use crate::db::error::DbResult;
use diesel::prelude::*;

#[derive(Insertable, AsChangeset ,Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::actor_images)]
pub struct ActorImageUpdate<'a> {
    pub filename: &'a str,
    pub height: i32,
    pub width: i32,
    pub file_url: &'a str,
    pub on_disk: bool,
}

pub fn update_actor_image(
    conn: &mut PgConnection,
    image_id: i32,
    filename: &str,
    height: i32,
    width: i32,
    file_url: &str,
    on_disk: bool,
) -> DbResult<()> {
    let update_image = ActorImageUpdate {
        filename,
        height,
        width,
        file_url,
        on_disk,
    };

    use crate::db::schema::actor_images::dsl::actor_images;
    use crate::db::schema::actor_images::dsl::id;
    diesel::update(actor_images.filter(id.eq(image_id)))
        .set::<ActorImageUpdate>(update_image)
        .execute(conn)?;
    Ok(())
}
