use diesel::{Identifiable, Queryable, Selectable};

pub mod read;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = crate::db::schema_views::active_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ActiveUser {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub user_actor_id: i32,
    pub channel_id: i32,
    pub channel_uuid: String,
    pub channel_actor_id: i32,
    pub avatar: Option<String>,
}
