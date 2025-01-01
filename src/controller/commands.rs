use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use axum_extra::extract::Host;

use crate::service::command_service::{
    CommandID, DynCommandsService, RunCommandDTO, RunCommandError,
};

use tracing::debug;

use super::host_is_external;

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
    let external_request = host_is_external(&host);
    Json(commands_service.all_commands(external_request))
}

pub async fn run_command<'a>(
    Host(host): Host,
    Path(id): Path<String>,
    State(commands_service): State<DynCommandsService>,
) -> Result<Json<RunCommandDTO>, RunCommandError> {
    debug!(host, id, "run_command");

    let external_request = host_is_external(&host);

    let response = commands_service
        .run_command(external_request, CommandID(id))
        .await?;

    Ok(Json(response))
}
