use diesel::{Identifiable, Insertable, Queryable, Selectable};

pub mod read;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = crate::db::schema_views::session_principals_view)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Principal {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub user_actor: String,
    pub channel_actor: String,
}
