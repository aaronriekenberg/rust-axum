mod commands;
mod connection_info;
mod health;
mod request_info;
mod version_info;

use axum::{Router, routing::get};

use std::sync::Arc;

use crate::{
    config,
    service::{command_service::CommandsService, connection_service::ConnectionTrackerService},
};

fn create_api_routes(
    commands_service: Arc<impl CommandsService>,
    connection_tracker_service: Arc<impl ConnectionTrackerService>,
) -> Router {
    let command_routes = Router::new()
        .route("/", get(commands::all_commands))
        .route("/{id}", get(commands::run_command))
        .with_state(commands_service);

    let connection_routes = Router::new()
        .route("/", get(connection_info::connection_info))
        .with_state(connection_tracker_service);

    Router::new()
        .nest("/commands", command_routes)
        .nest("/connection_info", connection_routes)
        .route("/request_info", get(request_info::request_info))
        .route("/version_info", get(version_info::version_info))
}

pub fn create_routes(
    server_configuration: &config::ServerConfiguration,
    commands_service: Arc<impl CommandsService>,
    connection_tracker_service: Arc<impl ConnectionTrackerService>,
) -> Router {
    Router::new().route("/health", get(health::health)).nest(
        &server_configuration.context,
        create_api_routes(commands_service, connection_tracker_service),
    )
}

fn host_is_external(host: &str) -> bool {
    config::instance()
        .server_configuration
        .external_hosts
        .iter()
        .any(|external_host| host == external_host)
}
