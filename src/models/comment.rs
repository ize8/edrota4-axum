use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct COD {
    pub id: i32,
    pub role_id: i32,
    pub date: NaiveDate,
    pub created_by: i32,
    pub comment: Option<String>,
    #[serde(serialize_with = "serialize_option_naive_as_utc")]
    pub created_at: Option<NaiveDateTime>,
}

fn serialize_option_naive_as_utc<S>(
    dt: &Option<NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match dt {
        Some(dt) => {
            let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(*dt, Utc);
            utc_dt.to_rfc3339().serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}
