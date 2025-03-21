use axum::{
    Json,
    body::Body,
    extract::{ConnectInfo, OriginalUri},
    http::Request,
    response::IntoResponse,
};

use crate::service::{connection_service::ConnectionID, request_info_service};

pub async fn request_info(
    ConnectInfo(connection_id): ConnectInfo<ConnectionID>,
    OriginalUri(original_uri): OriginalUri,
    request: Request<Body>,
) -> impl IntoResponse {
    Json(request_info_service::request_info(
        connection_id,
        original_uri,
        request,
    ))
}
