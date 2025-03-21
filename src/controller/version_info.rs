use axum::{Json, response::IntoResponse};

use crate::service::version_service;

pub async fn version_info() -> impl IntoResponse {
    Json(version_service::verison_info())
}
