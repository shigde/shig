use crate::db::streams::Stream;
use crate::db::users::User;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Stream))]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_participants)]
pub struct StreamParticipant {
    pub id: i32,
    pub stream_id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
