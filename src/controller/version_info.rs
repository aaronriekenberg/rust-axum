use axum::{body::Body, http::Request, response::IntoResponse, Json};

use crate::service::version_service;

pub async fn get_version_info(_request: Request<Body>) -> impl IntoResponse {
    Json(version_service::verison_info())
}
