use crate::models::error::ApiError;
use actix_web::web;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use crate::models::http::MESSAGE_INTERNAL_SERVER_ERROR;

#[allow(dead_code)]
const MAX_SIZE: usize = 262_144; // max payload size is 256k

#[allow(dead_code)]
pub async fn get_payload<T: DeserializeOwned>(mut payload: web::Payload) -> Result<T, ApiError> {
    // payload is a stream of Bytes objects
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                log::error!("Payload error: {}", e);
                return Err(ApiError::InternalServerError {
                    error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
                })
            }
        };
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            log::error!("Payload exceeded maximum size");
            return Err(ApiError::InternalServerError {
                error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
            });
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    match serde_json::from_slice::<T>(&body) {
        Ok(obj) => Ok(obj),
        Err(e) => Err(ApiError::InternalServerError {
            error_message: e.to_string(),
        }),
    }
}
