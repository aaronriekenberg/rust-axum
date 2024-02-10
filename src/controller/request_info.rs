use axum::{
    body::Body,
    extract::OriginalUri,
    http::{Request, Version},
    response::IntoResponse,
    Json,
};

use serde::Serialize;

use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
struct RequestFields {
    method: String,
    version: &'static str,
    original_uri: String,
}

#[derive(Debug, Serialize)]
struct RequestInfoResponse {
    request_fields: RequestFields,
    request_headers: BTreeMap<String, String>,
}

pub async fn get_request_info(
    OriginalUri(original_uri): OriginalUri,
    request: Request<Body>,
) -> impl IntoResponse {
    let version = match request.version() {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        _ => "[Unknown]",
    };

    let response = RequestInfoResponse {
        request_fields: RequestFields {
            method: request.method().as_str().to_owned(),
            version,
            original_uri: original_uri.to_string(),
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
