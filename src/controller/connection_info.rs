use axum::{extract::State, response::IntoResponse, Json};

use crate::service::connection_service::DynConnectionTrackerService;

pub async fn get_connection_info(
    State(connection_tracker_service): State<DynConnectionTrackerService>,
) -> impl IntoResponse {
    Json(connection_tracker_service.get_state_snapshot_dto().await)
}
