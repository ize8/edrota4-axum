use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct COD {
    pub id: i64,  // INT8 in database (bigserial)
    pub role_id: i64,  // INT8 in database
    pub date: NaiveDate,
    pub created_by: i32,  // INT4 - references Users.user_profile_id
    pub comment: Option<String>,
    #[serde(serialize_with = "serialize_datetime_with_millis")]
    pub created_at: Option<DateTime<Utc>>,  // TIMESTAMPTZ in database
}

fn serialize_datetime_with_millis<S>(
    dt: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use chrono::SecondsFormat;
    match dt {
        Some(dt) => {
            let formatted = dt.to_rfc3339_opts(SecondsFormat::Millis, true);
            serializer.serialize_str(&formatted)
        }
        None => serializer.serialize_none(),
    }
}

