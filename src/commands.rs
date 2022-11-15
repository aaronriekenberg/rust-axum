use axum::{
    async_trait,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use chrono::prelude::{Local, SecondsFormat};

use serde::Serialize;

use std::{collections::HashMap, sync::Arc, time::Instant};

use tokio::process::Command;

fn current_time_string() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

#[derive(Debug, Serialize)]
struct RunCommandResponse {
    now: String,
    command_duration_ms: u128,
    command_info: &'static crate::config::CommandInfo,
    command_output: String,
}

#[derive(Debug)]

enum RunCommandError {
    CommandNotFound,
}

impl IntoResponse for RunCommandError {
    fn into_response(self) -> Response {
        match self {
            Self::CommandNotFound => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

#[async_trait]
trait CommandsService {
    async fn run_command(&self, id: &str) -> Result<RunCommandResponse, RunCommandError>;
}

type DynCommandsService = Arc<dyn CommandsService + Send + Sync>;

struct CommandsServiceImpl {
    id_to_command_info: HashMap<String, &'static crate::config::CommandInfo>,
}

impl CommandsServiceImpl {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            id_to_command_info: crate::config::instance()
                .command_configuration()
                .commands()
                .iter()
                .map(|command_config| (command_config.id().clone(), command_config))
                .collect(),
        })
    }
}

#[async_trait]
impl CommandsService for CommandsServiceImpl {
    async fn run_command(&self, id: &str) -> Result<RunCommandResponse, RunCommandError> {
        let command_info = self
            .id_to_command_info
            .get(id)
            .cloned()
            .ok_or(RunCommandError::CommandNotFound)?;
    }
}

async fn get_all_commands() -> impl IntoResponse {
    Json(crate::config::instance().command_configuration().commands())
}

async fn run_command(
    Path(id): Path<String>,
    Extension(commands_service): Extension<DynCommandsService>,
) -> Result<Json<RunCommandResponse>, RunCommandError> {
    tracing::info!("in run_command id = {}", id);

    let response = commands_service.run_command(&id).await?;

    Ok(response.into())
}

pub fn router() -> Router {
    let commands_service: DynCommandsService = CommandsServiceImpl::new();

    Router::new()
        .route("/commands", get(get_all_commands))
        .route("/commands/:id", get(run_command))
        .layer(Extension(commands_service))
}
