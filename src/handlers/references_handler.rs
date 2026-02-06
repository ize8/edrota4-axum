use axum::{extract::State, Json};
use std::sync::Arc;

use crate::{models::TimeOffCategory, AppResult, AppState};

/// GET /api/references/time-off-categories
#[utoipa::path(
    get,
    path = "/api/references/time-off-categories",
    responses(
        (status = 200, description = "List of time-off categories", body = Vec<TimeOffCategory>)
    ),
    tag = "references"
)]
pub async fn get_time_off_categories(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<TimeOffCategory>>> {
    let categories = sqlx::query_as::<_, (i32, String, String, String, String)>(
        r#"
        SELECT
            id,
            name,
            short_name,
            font_color,
            bk_color
        FROM "TimeOffCategories"
        ORDER BY id
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let result = categories
        .into_iter()
        .map(|(id, label, short_name, font_color, bk_color)| TimeOffCategory {
            id,
            label,
            short_name,
            font_color,
            bk_color,
        })
        .collect();

    Ok(Json(result))
}
