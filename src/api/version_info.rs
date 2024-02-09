use axum::{body::Body, http::Request, response::IntoResponse, routing::get, Json, Router};

async fn version_info(_request: Request<Body>) -> impl IntoResponse {
    Json(crate::version::get_verison_info())
}

pub fn router() -> Router {
    Router::new().route("/", get(version_info))
}
