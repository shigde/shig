use crate::db::users::User;
use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::MESSAGE_INTERNAL_SERVER_ERROR;
use actix_web::http::header::HeaderValue;
use actix_web::web;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct JWTConfig {
    pub key: String,
    #[allow(dead_code)]
    pub live_time: i64,
}

#[derive(Serialize, Deserialize)]
pub struct JWT {
    // issued at
    pub iat: i64,
    // expiration
    pub exp: i64,
    // data
    pub user: String,
    pub uuid: String,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct JWTResponse {
    pub jwt: String,
}

impl JWT {
    #[allow(dead_code)]
    pub fn generate_token(user: &User, cgf: &web::Data<JWTConfig>) -> Result<String, ApiError> {
        let now = Utc::now().timestamp(); // nanosecond -> second
        let payload = JWT {
            iat: now,
            exp: now + cgf.live_time,
            user: user.name.clone(),
            uuid: user.user_uuid.clone(),
        };

        jsonwebtoken::encode(
            &Header::default(),
            &payload,
            &EncodingKey::from_secret(cgf.key.as_bytes()),
        )
        .map_err(|_| ApiError::InternalServerError {
            error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
        })
    }
}

pub fn decode_token(
    token: String,
    cgf: &web::Data<JWTConfig>,
) -> jsonwebtoken::errors::Result<TokenData<JWT>> {
    jsonwebtoken::decode::<JWT>(
        &token,
        &DecodingKey::from_secret(cgf.key.as_bytes()),
        &Validation::default(),
    )
}

pub fn verify_token(
    token_data: &TokenData<JWT>,
    pool: &web::Data<DbPool>,
) -> Result<Session, String> {
    let mut conn = pool.get().map_err(|_| "Failed to get db connection")?;
    let user =
        User::from_uuid(&mut conn, token_data.claims.uuid.clone()).map_err(|_| "Invalid token")?;
    Ok(Session::create(user))
}

#[allow(dead_code)]
pub fn is_auth_header_valid(authen_header: &HeaderValue) -> bool {
    if let Ok(authen_str) = authen_header.to_str() {
        return authen_str.starts_with("bearer") || authen_str.starts_with("Bearer");
    }
    false
}
