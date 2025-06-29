use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Deserializer};

pub fn from_iso8601_naive<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let dt = DateTime::parse_from_rfc3339(s)
        .map_err(serde::de::Error::custom)?;
    Ok(dt.naive_utc())
}

pub fn from_option_iso8601_to_naive<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<&str> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(serde::de::Error::custom)?;
            Ok(Some(dt.naive_utc()))
        }
        None => Ok(None),
    }
}