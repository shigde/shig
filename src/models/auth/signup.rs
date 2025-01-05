use actix_web::web;
use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::db::user_roles::Role;
use crate::db::users::create::create_new_user;
use crate::db::users::User;
use crate::models::error::ApiError;

#[derive(Serialize, Deserialize)]
pub struct SingUp {
    pub name: String,
    pub email: String,
    pub pass: String,
}

impl SingUp {
    #[allow(dead_code)]
    pub fn user(
        pool: &web::Data<DbPool>,
        sing_up: &web::Json<SingUp>,
    ) -> Result<User, ApiError> {
        let mut conn = pool.get()?;

        let user = create_new_user(
            &mut conn,
            sing_up.name.clone().as_str(),
            sing_up.email.clone().as_str(),
            sing_up.pass.clone().as_str(),
            Role::User,
            false
        )?;


        Ok(user)
    }
}
