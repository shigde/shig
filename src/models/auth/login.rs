use crate::db::instances::read::find_home_instance;
use crate::db::users::read::find_user_by_email;
use crate::db::DbPool;
use crate::models::auth::jwt::{JWTConfig, JWTResponse, RefreshJWT, JWT};
use crate::models::error::ApiError;
use crate::models::http::MESSAGE_LOGIN_FAILED;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Login {
    pub email: String,
    pub pass: String,
}

impl Login {
    pub fn authenticate(
        pool: &web::Data<DbPool>,
        login: &web::Json<Login>,
        cgf: &web::Data<JWTConfig>,
    ) -> Result<JWTResponse, ApiError> {
        let mut conn = pool.get()?;
        let user = find_user_by_email(&mut conn, login.email.clone())?;
        if !user.verify(login.pass.clone()) {
            return Err(ApiError::Unauthorized {
                error_message: MESSAGE_LOGIN_FAILED.to_string(),
            });
        }

        let instance = find_home_instance(&mut conn)?;
        let jwt = JWT::generate_token(&user, cgf)?;
        let refresh = RefreshJWT::generate_token(&user, instance.domain.clone(), cgf)?;
        Ok(JWTResponse { jwt, refresh })
    }
}
