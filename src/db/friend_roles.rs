use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = crate::db::schema::friend_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FriendRole {
    pub id: i32,
    pub name: String,
}
