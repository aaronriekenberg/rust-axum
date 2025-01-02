use axum::{extract::State, response::IntoResponse, Json};

use std::sync::Arc;

use crate::service::connection_service::ConnectionTrackerService;

pub async fn connection_info(
    State(connection_tracker_service): State<Arc<impl ConnectionTrackerService>>,
) -> impl IntoResponse {
    Json(connection_tracker_service.state_snapshot_dto().await)
}
