use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};
use crate::db::users::User;

pub mod create;
pub mod read;
pub mod update;
pub mod delete;

pub const FORGOTTEN_PASSWORD_VERIFICATION_TOKEN: &str = "forgotten_password";
pub const SIGN_UP_VERIFICATION_TOKEN: &str = "sign_up";

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(table_name = crate::db::schema::verification_tokens)]
pub struct VerificationToken {
    pub id: i32,
    pub user_id: i32,
    pub kind: String,
    pub token: String,
    pub verified: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
