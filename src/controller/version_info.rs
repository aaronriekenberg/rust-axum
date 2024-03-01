use axum::{response::IntoResponse, Json};

use crate::service::version_service;

pub async fn get_version_info() -> impl IntoResponse {
    Json(version_service::verison_info())
}
