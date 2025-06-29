use crate::db::instances::read::find_home_instance;
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
use crate::db::active_users::read::find_active_user_by_uuid;

const AUTH_SESSION_TIMEOUT: i64 = 600; // 10min;

#[derive(Deserialize, Clone)]
pub struct JWTConfig {
    pub auth_token_key: String,
    pub refresh_token_key: String,
    pub session_live_time: i64,
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

#[derive(Serialize, Deserialize)]
pub struct RefreshJWT {
    // issued at
    pub iat: i64,
    // expiration:
    // number of seconds from 1970-01-01T00:00:00Z UTC until the specified UTC
    pub exp: i64,
    // data
    pub domain: String,
    pub uuid: String,
}

#[derive(Serialize, Deserialize)]
pub struct JWTResponse {
    pub jwt: String,
    pub refresh: String,
}

impl JWT {
    pub fn generate_token(user: &User, cgf: &web::Data<JWTConfig>) -> Result<String, ApiError> {
        let now = Utc::now().timestamp(); // nanosecond -> second
        let payload = JWT {
            iat: now,
            exp: now + AUTH_SESSION_TIMEOUT, // 10min in seconds
            user: user.name.clone(),
            uuid: user.user_uuid.clone(),
        };

        generate_token(payload, cgf.auth_token_key.clone())
    }
}

impl RefreshJWT {
    pub fn generate_token(
        user: &User,
        domain: String,
        cgf: &web::Data<JWTConfig>,
    ) -> Result<String, ApiError> {
        let now = Utc::now().timestamp(); // nanosecond -> second
        let payload = RefreshJWT {
            iat: now,
            exp: now + cgf.session_live_time,
            domain: domain.clone(),
            uuid: user.user_uuid.clone(),
        };

        generate_token(payload, cgf.refresh_token_key.clone())
    }
}

fn generate_token<T: Serialize>(payload: T, key: String) -> Result<String, ApiError> {
    jsonwebtoken::encode(
        &Header::default(),
        &payload,
        &EncodingKey::from_secret(key.as_bytes()),
    )
    .map_err(|_| ApiError::InternalServerError {
        error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
    })
}

pub fn decode_auth_token(
    token: String,
    cgf: &web::Data<JWTConfig>,
) -> jsonwebtoken::errors::Result<TokenData<JWT>> {
    jsonwebtoken::decode::<JWT>(
        &token,
        &DecodingKey::from_secret(cgf.auth_token_key.as_bytes()),
        &Validation::default(),
    )
}

pub fn decode_refresh_token(
    token: String,
    cgf: &web::Data<JWTConfig>,
) -> jsonwebtoken::errors::Result<TokenData<RefreshJWT>> {
    jsonwebtoken::decode::<RefreshJWT>(
        &token,
        &DecodingKey::from_secret(cgf.refresh_token_key.as_bytes()),
        &Validation::default(),
    )
}

pub fn verify_auth_token(
    token_data: &TokenData<JWT>,
    pool: &web::Data<DbPool>,
) -> Result<Session, String> {
    let mut conn = pool.get().map_err(|_| "Failed to get db connection")?;
    let user = find_active_user_by_uuid(&mut conn, token_data.claims.uuid.as_str()).map_err(|_| "Invalid token")?;
    Ok(Session::create(user))
}

pub fn verify_refresh_token(
    pool: &web::Data<DbPool>,
    token_data: &TokenData<RefreshJWT>,
) -> Result<User, String> {
    let mut conn = pool.get().map_err(|_| "Failed to get db connection")?;
    let inst = find_home_instance(&mut conn).map_err(|_| "Invalid token")?;
    if inst.domain != token_data.claims.domain {
        return Err("Invalid token".to_string());
    }
    let user =
        User::from_uuid(&mut conn, token_data.claims.uuid.clone()).map_err(|_| "Invalid token")?;
    Ok(user)
}

#[allow(dead_code)]
pub fn is_auth_header_valid(authen_header: &HeaderValue) -> bool {
    if let Ok(authen_str) = authen_header.to_str() {
        return authen_str.starts_with("bearer") || authen_str.starts_with("Bearer");
    }
    false
}
