use uuid::Uuid;
use diesel::prelude::*;
use crate::db::schema::{users};
use crate::db::schema::users::dsl::*;
use crate::db::actors::{Actor};
use crate::db::user_roles::{Role, UserRole};
use chrono::{NaiveDateTime, Utc};
use bcrypt::{DEFAULT_COST, hash};

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(belongs_to(UserRole))]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub password: String,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

// impl User {
//     pub fn verify(&self, password: String) -> bool {
//         verify(password, &self.password).unwrap()
//     }
// }

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser<'a> {
    pub user_uuid: &'a str,
    pub name: &'a str,
    pub email: &'a str,
    pub user_role_id: i32,
    pub password: &'a str,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
}

#[allow(dead_code)]
fn insert_new_user(
    conn: &mut SqliteConnection,
    user_name: String,
    user_email: String,
    user_pass: String,
    user_role: Role,
) -> QueryResult<User> {
    let hashed_pass: String = hash(user_pass, DEFAULT_COST).unwrap();
    let uid = Uuid::new_v4().to_string();
    let new_user = NewUser {
        user_uuid: &uid,
        name: &user_name,
        email: &user_email,
        password: &hashed_pass,
        user_role_id: user_role.val(),
        active: true,
        actor_id: 0,
        created_at: Utc::now().naive_utc(),
    };

    diesel::insert_into(users)
        .values(&new_user)
        .execute(conn)?;

    let user = users.first(conn)?;

    Ok(user)
}
