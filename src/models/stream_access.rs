use crate::db::DbPool;
use crate::models::auth::jwt::{decode_auth_token, verify_auth_token, JWTConfig};
use crate::models::error::ApiError;
use actix_web::web;
use diesel::prelude::*;
use futures::{future::BoxFuture, stream::FuturesUnordered, StreamExt};
use moq_auth::{Grant, Pattern, Request};
use serde::Deserialize;
use std::time::Duration;

#[cfg(test)]
#[path = "stream_access_test.rs"]
mod tests;

#[derive(Deserialize)]
struct Credential {
    jwt: Option<String>,
}

fn credential(request: &Request) -> Result<Option<String>, ApiError> {
    let refused = || ApiError::Unauthorized {
        error_message: "Invalid relay credentials".into(),
    };
    let query = web::Query::<Credential>::from_query(request.query.as_deref().unwrap_or(""))
        .map_err(|_| refused())?
        .into_inner()
        .jwt
        .filter(|token| !token.is_empty());
    let setup = match &request.token {
        Some(token) if token.kind == moq_auth::Token::OUT_OF_BAND => {
            Some(String::from_utf8(token.value.clone()).map_err(|_| refused())?)
        }
        Some(_) => return Err(refused()),
        None => None,
    };
    if query.is_some() && setup.is_some() && query != setup {
        return Err(refused());
    }
    Ok(setup.or(query))
}

fn decide(request: &Request, pool: &DbPool, jwt: &JWTConfig) -> Result<Grant, ApiError> {
    use crate::db::schema::{channels, streams, users};
    let authenticated = if let Some(token) = credential(request)? {
        let data = decode_auth_token(token, &web::Data::new(jwt.clone())).map_err(|_| {
            ApiError::Unauthorized {
                error_message: "Invalid relay login".into(),
            }
        })?;
        verify_auth_token(&data, &web::Data::new(pool.clone())).map_err(|_| {
            ApiError::Unauthorized {
                error_message: "Invalid relay login".into(),
            }
        })?;
        true
    } else {
        false
    };
    let mut conn = pool.get()?;
    let mut query = streams::table
        .inner_join(channels::table.on(channels::id.eq(streams::channel_id)))
        .inner_join(users::table.on(users::id.eq(streams::user_id)))
        .filter(users::active.eq(true))
        .select((channels::uuid, streams::uuid))
        .into_boxed();
    if !authenticated {
        query = query.filter(streams::is_public.eq(true));
    }
    let root = moq_net::Path::new(&request.path);
    let parts: Vec<_> = root
        .as_str()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if !parts.is_empty() && parts[0] != "live" {
        return Err(ApiError::Unauthorized {
            error_message: "Unknown relay namespace".into(),
        });
    }
    if let Some(channel) = parts.get(1) {
        query = query.filter(channels::uuid.eq(channel.to_string()));
    }
    if let Some(stream) = parts.get(2) {
        query = query.filter(streams::uuid.eq(stream.to_string()));
    }
    let paths = query.load::<(String, String)>(&mut conn)?;
    let subscribe = paths
        .into_iter()
        .filter_map(|(channel, stream)| {
            let path = format!("live/{channel}/{stream}");
            moq_net::Path::new(&path)
                .strip_prefix(&root)
                .and_then(|relative| Pattern::literal(relative.as_str()).ok())
        })
        .collect();
    // No network publishing grant: the authorized SFU publishes directly to the origin.
    let grant = Grant::new(Default::default(), subscribe);
    grant.validate().map_err(|_| ApiError::Unauthorized {
        error_message: "No accessible streams".into(),
    })?;
    Ok(grant)
}

pub async fn authorize_relay(
    mut admissions: moq_relay::auth::Admissions,
    pool: DbPool,
    jwt: JWTConfig,
) -> anyhow::Result<()> {
    let mut leases: FuturesUnordered<BoxFuture<'static, ()>> = FuturesUnordered::new();
    loop {
        let admission = tokio::select! {
            next = admissions.next() => next.ok_or_else(|| anyhow::anyhow!("Relay admission queue closed"))?,
            _ = leases.next(), if !leases.is_empty() => continue,
        };
        let request = admission.request.clone();
        let check_pool = pool.clone();
        let check_jwt = jwt.clone();
        match tokio::task::spawn_blocking(move || decide(&request, &check_pool, &check_jwt)).await {
            Ok(Ok(grant)) => {
                let (lease, consumer) = moq_auth::lease::Producer::new(grant);
                let request = admission.request.clone();
                leases.push(Box::pin(maintain_lease(
                    request,
                    pool.clone(),
                    jwt.clone(),
                    lease,
                )));
                admission.grant(consumer);
            }
            Ok(Err(ApiError::Unauthorized { .. } | ApiError::Forbidden { .. })) => {
                admission.refuse(moq_relay::auth::Error::Refused);
            }
            Ok(Err(error)) => {
                log::error!("Relay authorization database failure: {error}");
                admission.refuse(moq_relay::auth::Error::Unavailable(
                    "Authorization unavailable".into(),
                ));
            }
            Err(error) => {
                log::error!("Relay authorization task failed: {error}");
                admission.refuse(moq_relay::auth::Error::Unavailable(
                    "Authorization unavailable".into(),
                ));
            }
        }
    }
}

// These futures belong to the relay serving loop, not detached per-session tasks.
async fn maintain_lease(
    request: Request,
    pool: DbPool,
    jwt: JWTConfig,
    lease: moq_auth::lease::Producer,
) {
    loop {
        tokio::select! {
            _ = lease.closed() => return,
            _ = tokio::time::sleep(Duration::from_secs(30)) => {}
        }
        let request = request.clone();
        let pool = pool.clone();
        let jwt = jwt.clone();
        match tokio::task::spawn_blocking(move || decide(&request, &pool, &jwt)).await {
            Ok(Ok(grant)) => lease.update(grant),
            _ => {
                lease.revoke(moq_auth::lease::Reason::Refused);
                return;
            }
        }
    }
}
