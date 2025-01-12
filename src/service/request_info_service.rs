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

fn build_request_headers(request: &Request<Body>) -> BTreeMap<String, String> {
    let mut headers_map: BTreeMap<String, String> = BTreeMap::new();

    for (key, value) in request.headers() {
        let key_str = key.as_str();
        let value_str = value.to_str().unwrap_or("[Unknown]");

        match headers_map.get_mut(key_str) {
            None => {
                headers_map.insert(key_str.to_owned(), value_str.to_owned());
            }
            Some(current_value) => {
                current_value.push_str("; ");
                current_value.push_str(value_str);
            }
        };
    }

    headers_map
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
        request_headers: build_request_headers(&request),
    }
}
