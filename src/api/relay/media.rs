// use crate::relay::state::RelayState;
// use actix_web::error::{
//     ErrorGatewayTimeout, ErrorInternalServerError, ErrorNotFound, ErrorUnauthorized,
// };
// use actix_web::{get, web, Error, HttpResponse};
// use async_stream::try_stream;
// use bytes::Bytes;
// use futures_util::stream::Stream;
// use moq_relay::{AuthError, AuthParams, AuthToken};
// use serde::Deserialize;
// use crate::api::relay::auth::AuthQuery;
//
// struct ServeGroup {
//     group: Option<moq_lite::GroupConsumer>,
//     frame: Option<moq_lite::FrameConsumer>,
//     deadline: tokio::time::Instant,
// }
//
// #[derive(Deserialize)]
// struct FetchPath {
//     pub channel_uuid: String,
//     pub stream_uuid: String,
//     pub track_uuid: String,
// }
//
// #[derive(Debug, serde::Deserialize)]
// struct FetchParams {
//     #[serde(flatten)]
//     auth: AuthQuery,
//
//     #[serde(default)]
//     group: FetchGroup,
//
//     #[serde(default)]
//     frame: FetchFrame,
// }
//
// #[derive(Debug, Default)]
// enum FetchGroup {
//     // Return the group at the given sequence number.
//     Num(u64),
//
//     // Return the latest group.
//     #[default]
//     Latest,
// }
//
// impl<'de> serde::Deserialize<'de> for FetchGroup {
//     fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
//         let s = String::deserialize(deserializer)?;
//         if let Ok(num) = s.parse::<u64>() {
//             Ok(FetchGroup::Num(num))
//         } else if s == "latest" {
//             Ok(FetchGroup::Latest)
//         } else {
//             Err(serde::de::Error::custom(format!(
//                 "invalid group value: {s}"
//             )))
//         }
//     }
// }
//
// #[derive(Debug, Default)]
// enum FetchFrame {
//     Num(usize),
//     #[default]
//     Chunked,
// }
//
// impl<'de> serde::Deserialize<'de> for FetchFrame {
//     fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
//         let s = String::deserialize(deserializer)?;
//         if let Ok(num) = s.parse::<usize>() {
//             Ok(FetchFrame::Num(num))
//         } else if s == "chunked" {
//             Ok(FetchFrame::Chunked)
//         } else {
//             Err(serde::de::Error::custom(format!(
//                 "invalid frame value: {s}"
//             )))
//         }
//     }
// }
//
// // ?group=42
// // ?group=latest
// // ?frame=7
// // ?frame=chunked
// // ?jwt=abc&register=yes&group=latest&frame=10
// #[get("/fetch/{channel_uuid}/stream/{stream_uuid}/track/{track_uuid}")]
// pub async fn fetch(
//     state: web::Data<RelayState>,
//     path: web::Path<FetchPath>,
//     query: web::Query<FetchParams>,
// ) -> actix_web::Result<HttpResponse> {
//     let FetchPath {
//         channel_uuid,
//         stream_uuid,
//         track_uuid,
//     } = path.into_inner();
//
//     let fetch_path = format!(
//         "/fetch/{}/stream/{}/track/{}",
//         channel_uuid, stream_uuid, track_uuid
//     );
//
//     let auth = AuthParams {
//         path: fetch_path.clone(),
//         jwt: query.auth.jwt.clone(),
//     };
//
//     let token: AuthToken = state
//         .auth
//         .verify(&auth)
//         .await
//         .map_err(|_: AuthError| ErrorUnauthorized("invalid token"))?;
//
//     let Some(origin) = state.cluster.subscriber(&token) else {
//         return Err(ErrorUnauthorized("unauthorized"));
//     };
//
//     log::info!("fetching {}", fetch_path);
//
//     let track = moq_lite::Track {
//         name: track_uuid,
//         priority: 0,
//     };
//
//     // NOTE: The auth token is already scoped to the broadcast.
//     let broadcast = origin
//         //.consume_broadcast("")
//         //.ok_or(ErrorNotFound("broadcast not found"))?;
//
//     let mut track = broadcast.subscribe_track(&track).map_err(|err| match err {
//         moq_lite::Error::NotFound => ErrorNotFound("track producer not found"),
//         _ => ErrorNotFound("unknown broadcast"),
//     })?;
//
//     let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
//
//     let result = tokio::time::timeout_at(deadline, async {
//         let group = match query.group {
//             FetchGroup::Latest => match track.latest() {
//                 Some(sequence) => track.get_group(sequence).await,
//                 None => track.next_group().await,
//             },
//             FetchGroup::Num(sequence) => track.get_group(sequence).await,
//         };
//
//         let group = match group {
//             Ok(Some(group)) => group,
//             Ok(None) => return Err(ErrorNotFound("group not found")),
//             Err(_) => return Err(ErrorInternalServerError("find group failed")),
//         };
//
//         tracing::info!(track = %track.info.name, group = %group.info.sequence, "serving group");
//
//         match query.frame {
//             FetchFrame::Num(index) => match group.get_frame(index).await {
//                 Ok(Some(frame)) => Ok(ServeGroup {
//                     group: None,
//                     frame: Some(frame),
//                     deadline,
//                 }),
//                 Ok(None) => return Err(ErrorNotFound("frame not found")),
//                 Err(_) => return Err(ErrorInternalServerError("find frame failed")),
//             },
//             FetchFrame::Chunked => Ok(ServeGroup {
//                 group: Some(group),
//                 frame: None,
//                 deadline,
//             }),
//         }
//     })
//     .await;
//
//     match result {
//         Ok(Ok(serve)) => {
//             let stream = serve_group_stream(serve);
//             Ok(HttpResponse::Ok()
//                 .content_type("application/octet-stream")
//                 .streaming(stream))
//         }
//         Ok(Err(err)) => Err(err),
//         Err(_) => Err(ErrorGatewayTimeout("fetch timeout")),
//     }
// }
//
// fn serve_group_stream(mut serve: ServeGroup) -> impl Stream<Item = Result<Bytes, Error>> {
//     try_stream! {
//         while serve.group.is_some() || serve.frame.is_some() {
//             if let Some(mut frame) = serve.frame.take() {
//                 let bytes = tokio::time::timeout_at(serve.deadline, frame.read_all())
//                     .await
//                     .map_err(|_| ErrorGatewayTimeout("frame timeout"))?
//                     .map_err(ErrorInternalServerError)?;
//
//                 yield bytes;
//                 continue;
//             }
//
//             if let Some(group) = serve.group.as_mut() {
//                 serve.frame = tokio::time::timeout_at(serve.deadline, group.next_frame())
//                     .await
//                     .map_err(|_| ErrorGatewayTimeout("frame timeout"))?
//                     .map_err(ErrorInternalServerError)?;
//
//                 if serve.frame.is_none() {
//                     serve.group.take();
//                 }
//             }
//         }
//     }
// }
