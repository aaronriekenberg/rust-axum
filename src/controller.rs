mod commands;
mod request_info;
mod version_info;

use axum::{routing::get, Router};

use crate::service::command_service::DynCommandsService;

pub fn create_api_routes(commands_service: DynCommandsService) -> Router {
    let command_routes = Router::new()
        .route("/", get(commands::get_all_commands))
        .route("/:id", get(commands::run_command))
        .with_state(commands_service);

    Router::new()
        .nest("/commands", command_routes)
        .route("/request_info", get(request_info::request_info))
        .route("/version_info", get(version_info::version_info))
}
