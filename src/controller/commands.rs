use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use axum_extra::extract::Host;

use std::sync::Arc;

use crate::service::command_service::{CommandID, CommandsService, RunCommandDTO, RunCommandError};

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
    State(commands_service): State<Arc<impl CommandsService>>,
) -> impl IntoResponse {
    let external_request = host_is_external(&host);
    Json(commands_service.all_commands(external_request))
}

pub async fn run_command(
    Host(host): Host,
    Path(id): Path<String>,
    State(commands_service): State<Arc<impl CommandsService>>,
) -> Result<Json<RunCommandDTO>, RunCommandError> {
    debug!(host, id, "run_command");

    let external_request = host_is_external(&host);

    let response = commands_service
        .run_command(external_request, CommandID(id))
        .await?;

    Ok(Json(response))
}
