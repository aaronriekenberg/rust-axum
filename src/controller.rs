mod commands;
mod connection_info;
mod request_info;
mod version_info;

use axum::{routing::get, Router};

use crate::service::{
    command_service::DynCommandsService, connection_service::DynConnectionTrackerService,
};

pub fn create_api_routes(
    commands_service: DynCommandsService,
    connection_tracker_service: DynConnectionTrackerService,
) -> Router {
    let command_routes = Router::new()
        .route("/", get(commands::get_all_commands))
        .route("/:id", get(commands::run_command))
        .with_state(commands_service);

    let connection_routes = Router::new()
        .route("/", get(connection_info::get_connection_info))
        .with_state(connection_tracker_service);

    Router::new()
        .nest("/commands", command_routes)
        .nest("/connection_info", connection_routes)
        .route("/request_info", get(request_info::get_request_info))
        .route("/version_info", get(version_info::get_version_info))
}
