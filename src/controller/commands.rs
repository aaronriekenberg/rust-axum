use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::service::command_service::{DynCommandsService, RunCommandError, RunCommandResponse};

use tracing::debug;

use crate::config;

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
            Self::SemaphoreAcquireError => StatusCode::TOO_MANY_REQUESTS.into_response(),
        }
    }
}

pub async fn get_all_commands() -> impl IntoResponse {
    Json(&config::instance().command_configuration.commands)
}

pub async fn run_command(
    Path(id): Path<String>,
    State(commands_service): State<DynCommandsService>,
) -> Result<Json<RunCommandResponse>, RunCommandError> {
    debug!("in run_command id = {}", id);

    let response = commands_service.run_command(&id).await?;

    Ok(response.into())
}
