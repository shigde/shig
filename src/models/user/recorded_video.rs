use crate::models::error::ApiError;
use chrono::NaiveDateTime;
use diesel::QueryableByName;
use serde::{Deserialize, Serialize};

/// A local recorded video. Stream metadata is copied once, never inherited dynamically.
#[derive(Debug, Clone, Serialize, QueryableByName)]
#[serde(rename_all = "camelCase")]
pub struct RecordedVideo {
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub uuid: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    #[serde(rename = "type")]
    pub content_type: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub source_stream_uuid: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    #[serde(skip_serializing)]
    pub user_id: i32,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub owner_uuid: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub channel_uuid: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub title: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub description: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub support: String,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub thumbnail: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub licence: i32,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_public: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
    pub published_at: Option<NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    pub status: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub duration_ms: i64,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub error: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
    pub started_at: Option<NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]
    pub ended_at: Option<NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub created_at: NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub updated_at: NaiveDateTime,
}

impl RecordedVideo {
    pub fn require_owner(&self, user_id: i32) -> Result<(), ApiError> {
        if self.user_id != user_id {
            return Err(ApiError::Forbidden {
                error_message: "Only the video owner can manage this video".into(),
            });
        }
        Ok(())
    }
    pub fn is_visible_to(&self, user_id: Option<i32>) -> bool {
        user_id == Some(self.user_id)
            || (self.is_public && self.published_at.is_some() && self.status == "ready")
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordedVideoUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub support: Option<String>,
    pub thumbnail: Option<String>,
    pub licence: Option<i32>,
    pub is_public: Option<bool>,
    pub published: Option<bool>,
}
impl RecordedVideoUpdate {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self
            .title
            .as_ref()
            .is_some_and(|title| title.trim().is_empty())
        {
            return Err(ApiError::BadRequest {
                error_message: "RecordedVideo title cannot be empty".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_updates_cannot_override_recording_state_or_origin() {
        assert!(serde_json::from_str::<RecordedVideoUpdate>(r#"{"status":"ready"}"#).is_err());
        assert!(
            serde_json::from_str::<RecordedVideoUpdate>(r#"{"sourceStreamUuid":"other"}"#).is_err()
        );
        assert!(
            serde_json::from_str::<RecordedVideoUpdate>(r#"{"archivePrefix":"other"}"#).is_err()
        );
        assert!(RecordedVideoUpdate {
            title: Some(" ".into()),
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(RecordedVideoUpdate {
            title: Some("My video".into()),
            published: Some(false),
            ..Default::default()
        }
        .validate()
        .is_ok());
    }
}
