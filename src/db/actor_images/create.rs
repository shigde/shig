use crate::db::actor_images::{ActorImage, ActorImageType};
use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};

#[derive(Insertable, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::actor_images)]
pub struct NewActorImage<'a> {
    pub filename: &'a str,
    pub height: i32,
    pub width: i32,
    pub file_url: &'a str,
    pub on_disk: bool,
    pub image_type: &'a str,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_actor_image(
    conn: &mut PgConnection,
    filename: &str,
    height: i32,
    width: i32,
    file_url: &str,
    on_disk: bool,
    image_type: ActorImageType,
    actor_id: i32,
) -> DbResult<ActorImage> {
    let new_image = NewActorImage {
        filename,
        height,
        width,
        file_url,
        on_disk,
        image_type: image_type.value_as_str(),
        actor_id,
        created_at: Utc::now().naive_utc(),
    };

    use crate::db::schema::actor_images;
    let image = diesel::insert_into(actor_images::table)
        .values(&new_image)
        .returning(ActorImage::as_returning())
        .get_result::<ActorImage>(conn)?;

    Ok(image)
}
