use axum::{
    extract::{Host, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::service::command_service::{
    CommandID, DynCommandsService, RunCommandDTO, RunCommandError,
};

use tracing::debug;

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
            Self::SemaphoreAcquireError => StatusCode::TOO_MANY_REQUESTS.into_response(),
        }
    }
}

pub async fn all_commands(
    Host(host): Host,
    State(commands_service): State<DynCommandsService>,
) -> impl IntoResponse {
    Json(commands_service.all_comamnds(&host))
}

pub async fn run_command(
    Host(host): Host,
    Path(id): Path<String>,
    State(commands_service): State<DynCommandsService>,
) -> Result<Json<RunCommandDTO>, RunCommandError> {
    debug!(id, "run_command");

    let response = commands_service.run_command(&host, CommandID(id)).await?;

    Ok(Json(response))
}
