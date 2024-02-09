use axum::{body::Body, http::Request, response::IntoResponse, routing::get, Json, Router};

use std::collections::BTreeMap;

async fn version_info(_request: Request<Body>) -> impl IntoResponse {
    let mut map = BTreeMap::new();

    map.insert("build_timestamp", env!("VERGEN_BUILD_TIMESTAMP"));

    map.insert("cargo_debug", env!("VERGEN_CARGO_DEBUG"));

    map.insert("cargo_opt_level", env!("VERGEN_CARGO_OPT_LEVEL"));

    map.insert("cargo_pkg_version", env!("CARGO_PKG_VERSION"));

    map.insert("cargo_target_triple", env!("VERGEN_CARGO_TARGET_TRIPLE"));

    map.insert("rustc_channel", env!("VERGEN_RUSTC_CHANNEL"));

    map.insert("rustc_semver", env!("VERGEN_RUSTC_SEMVER"));

    Json(map)
}

pub fn router() -> Router {
    Router::new().route("/", get(version_info))
}
