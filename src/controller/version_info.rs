use axum::{body::Body, http::Request, response::IntoResponse, Json};

pub async fn get_version_info(_request: Request<Body>) -> impl IntoResponse {
    Json(crate::service::version_service::get_verison_info())
}
