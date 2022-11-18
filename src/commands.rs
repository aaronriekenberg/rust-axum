mod service;

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use service::{DynCommandsService, RunCommandError, RunCommandResponse};

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
            Self::SemaphoreAcquireError => StatusCode::TOO_MANY_REQUESTS.into_response(),
        }
    }
}

async fn get_all_commands() -> impl IntoResponse {
    Json(crate::config::instance().command_configuration().commands())
}

async fn run_command(
    Path(id): Path<String>,
    Extension(commands_service): Extension<DynCommandsService>,
) -> Result<Json<RunCommandResponse>, RunCommandError> {
    tracing::debug!("in run_command id = {}", id);

    let response = commands_service.run_command(&id).await?;

    Ok(response.into())
}

pub fn router() -> Router {
    let commands_service: DynCommandsService = service::new_commands_service();

    Router::new()
        .route("/commands", get(get_all_commands))
        .route("/commands/:id", get(run_command))
        .layer(Extension(commands_service))
}
