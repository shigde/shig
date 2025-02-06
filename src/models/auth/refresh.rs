use crate::db::DbPool;
use crate::models::auth::jwt::{
    decode_refresh_token, verify_refresh_token, JWTConfig, JWTResponse, JWT,
};

use crate::models::error::ApiError;
use crate::models::http::MESSAGE_NOT_ACCEPTABLE;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Refresh {
    pub token: String,
}

impl Refresh {
    pub fn generate_token(
        pool: &web::Data<DbPool>,
        payload: &web::Json<Refresh>,
        cgf: &web::Data<JWTConfig>,
    ) -> Result<JWTResponse, ApiError> {
        let refresh_token = payload.token.clone();

        if let Ok(token_data) = decode_refresh_token(refresh_token.clone(), cgf) {
            let user_result = verify_refresh_token(pool, &token_data);

            if user_result.is_ok() {
                let user = user_result.unwrap();
                return match JWT::generate_token(&user, cgf) {
                    Ok(jwt_token) => Ok(JWTResponse {
                        jwt: jwt_token,
                        refresh: refresh_token.clone(),
                    }),
                    // 406
                    Err(_) => Err(ApiError::NotAcceptable {
                        error_message: MESSAGE_NOT_ACCEPTABLE.to_string(),
                    }),
                };
            }
        }
        // 406
        Err(ApiError::NotAcceptable {
            error_message: MESSAGE_NOT_ACCEPTABLE.to_string(),
        })
    }
}
