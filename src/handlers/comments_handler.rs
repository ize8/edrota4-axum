use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{models::COD, AppResult, AppState};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetCommentsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

/// GET /api/comments?year=&month=&roleId=
#[utoipa::path(
    get,
    path = "/api/comments",
    params(GetCommentsQuery),
    responses(
        (status = 200, description = "List of comments (Consultant on Duty) for specified filters", body = Vec<COD>)
    ),
    tag = "comments"
)]
pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetCommentsQuery>,
) -> AppResult<Json<Vec<COD>>> {
    let mut sql = r#"
        SELECT *
        FROM "COD"
        WHERE 1=1
    "#
    .to_string();

    let mut bindings = vec![];

    if let Some(year) = query.year {
        sql.push_str(&format!(" AND EXTRACT(YEAR FROM date) = ${}", bindings.len() + 1));
        bindings.push(year);
    }

    if let Some(month) = query.month {
        sql.push_str(&format!(" AND EXTRACT(MONTH FROM date) = ${}", bindings.len() + 1));
        bindings.push(month);
    }

    if let Some(role_id) = query.role_id {
        sql.push_str(&format!(" AND role_id = ${}", bindings.len() + 1));
        bindings.push(role_id);
    }

    sql.push_str(" ORDER BY date");

    let mut query_builder = sqlx::query_as::<_, COD>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let comments = query_builder.fetch_all(&state.db).await?;

    Ok(Json(comments))
}
