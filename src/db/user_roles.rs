use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = crate::db::schema::user_roles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserRole {
    pub id: i32,
    pub name: String,
}

#[allow(dead_code)]
pub enum Role {
    Admin,
    User,
    Guest,
    Application,
    Service,
    Organisation,
}

#[allow(dead_code)]
impl Role {
    pub fn val(&self) -> i32 {
        match self {
            Self::Admin => 1,
            Self::User => 2,
            Self::Guest => 3,
            Self::Application => 4,
            Self::Service => 5,
            Self::Organisation => 6,
        }
    }
}
