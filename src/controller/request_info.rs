use axum::{
    body::Body,
    extract::{ConnectInfo, OriginalUri},
    http::Request,
    response::IntoResponse,
    Json,
};

use crate::{connection::ConnectionInfo, service::request_info_service};

pub async fn get_request_info(
    ConnectInfo(connection_info): ConnectInfo<ConnectionInfo>,
    OriginalUri(original_uri): OriginalUri,
    request: Request<Body>,
) -> impl IntoResponse {
    Json(request_info_service::get_request_info(
        connection_info,
        original_uri,
        request,
    ))
}
