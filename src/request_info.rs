use axum::{
    body::Body,
    http::{Request, Version},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use serde::Serialize;

use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
struct RequestFields {
    method: String,
    version: String,
    request_uri: String,
}

#[derive(Debug, Serialize)]
struct RequestInfoResponse {
    request_fields: RequestFields,
    request_headers: BTreeMap<String, String>,
}

async fn request_info(request: Request<Body>) -> impl IntoResponse {
    let version = match request.version() {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        _ => "[Unknown]",
    }
    .to_owned();

    let response = RequestInfoResponse {
        request_fields: RequestFields {
            method: request.method().as_str().to_owned(),
            version,
            request_uri: request.uri().to_string(),
        },
        request_headers: request
            .headers()
            .iter()
            .map(|(key, value)| {
                (
                    key.as_str().to_owned(),
                    value.to_str().unwrap_or("[Unknown]").to_owned(),
                )
            })
            .collect(),
    };

    Json(response)
}

pub fn router() -> Router {
    Router::new().route("/", get(request_info))
}
