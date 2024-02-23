use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::service::command_service::{DynCommandsService, RunCommandDTO, RunCommandError};

use tracing::debug;

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
            Self::SemaphoreAcquireError => StatusCode::TOO_MANY_REQUESTS.into_response(),
        }
    }
}

pub async fn all_commands(State(commands_service): State<DynCommandsService>) -> impl IntoResponse {
    Json(commands_service.all_comamnds())
}

pub async fn run_command(
    Path(id): Path<String>,
    State(commands_service): State<DynCommandsService>,
) -> Result<Json<RunCommandDTO>, RunCommandError> {
    debug!("in run_command id = {}", id);

    let response = commands_service.run_command(&id).await?;

    Ok(Json(response))
}
