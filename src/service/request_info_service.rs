use axum::{
    body::Body,
    http::{Request, Uri, Version},
};

use serde::Serialize;

use std::collections::BTreeMap;

use crate::service::connection_service::ConnectionID;

#[derive(Debug, Serialize)]
struct RequestFieldsDTO {
    connection_id: usize,
    method: String,
    version: &'static str,
    original_uri: String,
}

#[derive(Debug, Serialize)]
pub struct RequestInfoDTO {
    request_fields: RequestFieldsDTO,
    request_headers: BTreeMap<String, String>,
}

pub fn request_info(
    connection_id: ConnectionID,
    original_uri: Uri,
    request: Request<Body>,
) -> RequestInfoDTO {
    let version = match request.version() {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        _ => "[Unknown]",
    };

    RequestInfoDTO {
        request_fields: RequestFieldsDTO {
            connection_id: connection_id.as_usize(),
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
    }
}
