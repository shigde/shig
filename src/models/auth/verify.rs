use crate::db::DbPool;
use crate::models::error::ApiError;

use crate::db::users::update::activate_user_by_verification_token;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Verify {}

impl Verify {
    pub fn user(pool: &web::Data<DbPool>, path: web::Path<String>) -> Result<(), ApiError> {
        let token = path.into_inner();
        let mut conn = pool.get()?;

        let _ = activate_user_by_verification_token(&mut conn, token.as_str())?;
        Ok(())
    }
}
