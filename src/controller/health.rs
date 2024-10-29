use axum::response::IntoResponse;

pub async fn health() -> impl IntoResponse {
    "all good".into_response()
}
