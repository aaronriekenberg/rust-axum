use axum::{body::Body, extract::OriginalUri, http::Request, response::IntoResponse, Json};

use crate::service::request_info_service;

pub async fn get_request_info(
    OriginalUri(original_uri): OriginalUri,
    request: Request<Body>,
) -> impl IntoResponse {
    Json(request_info_service::get_request_info(
        original_uri,
        request,
    ))
}
