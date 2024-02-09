mod service;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use service::{DynCommandsService, RunCommandError, RunCommandResponse};

use tracing::debug;

use crate::api::config;

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
            Self::SemaphoreAcquireError => StatusCode::TOO_MANY_REQUESTS.into_response(),
        }
    }
}

async fn get_all_commands() -> impl IntoResponse {
    Json(&config::instance().command_configuration.commands)
}

async fn run_command(
    Path(id): Path<String>,
    State(commands_service): State<DynCommandsService>,
) -> Result<Json<RunCommandResponse>, RunCommandError> {
    debug!("in run_command id = {}", id);

    let response = commands_service.run_command(&id).await?;

    Ok(response.into())
}

pub fn router() -> Router {
    let commands_service = service::new_commands_service();

    Router::new()
        .route("/", get(get_all_commands))
        .route("/:id", get(run_command))
        .with_state(commands_service)
}
