use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use utoipa::ToSchema;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DiaryEntry {
    pub id: i32,
    pub role_id: i32,
    pub date: NaiveDate,
    pub entry: Option<String>,
    pub al: bool,
    pub sl: bool,
    pub pl: bool,
    #[serde(serialize_with = "serialize_naive_as_utc")]
    pub created_at: NaiveDateTime,
    pub user_profile_id: Option<i32>,
    pub created_by: i32,
    pub deleted: bool,
    #[sqlx(default)]
    pub short_name: Option<String>,  // From LEFT JOIN with Users table
}
fn serialize_naive_as_utc<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use chrono::SecondsFormat;
    let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(*dt, Utc);
    utc_dt.to_rfc3339_opts(SecondsFormat::Millis, true).serialize(serializer)
}
