use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use chrono::prelude::{Local, SecondsFormat};

use serde::Serialize;

use std::{collections::HashMap, sync::Arc, time::Instant};

use tokio::process::Command;

use tower::ServiceBuilder;

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

struct CommandsService {
    id_to_command_info: HashMap<String, &'static crate::config::CommandInfo>,
}

impl CommandsService {
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

    async fn internal_run_command(
        self: Arc<Self>,
        command_info: &crate::config::CommandInfo,
    ) -> Result<std::process::Output, std::io::Error> {
        let output = Command::new(command_info.command())
            .args(command_info.args())
            .output()
            .await?;

        Ok(output)
    }

    async fn run_command(self: Arc<Self>, id: &String) -> Result<RunCommandResponse, StatusCode> {
        let command_info = self
            .id_to_command_info
            .get(id)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?;

        let command_start_time = Instant::now();
        let command_result = self.internal_run_command(&command_info).await;
        let command_duration = command_start_time.elapsed();

        let response = RunCommandResponse {
            now: current_time_string(),
            command_duration_ms: command_duration.as_millis(),
            command_info: &command_info,
            command_output: match command_result {
                Err(err) => {
                    format!("error running command {}", err)
                }
                Ok(command_output) => {
                    let mut combined_output = String::with_capacity(
                        command_output.stderr.len() + command_output.stdout.len(),
                    );
                    combined_output.push_str(&String::from_utf8_lossy(&command_output.stderr));
                    combined_output.push_str(&String::from_utf8_lossy(&command_output.stdout));
                    combined_output
                }
            },
        };

        Ok(response)
    }
}

async fn get_all_commands() -> impl IntoResponse {
    Json(crate::config::instance().command_configuration().commands())
}

async fn run_command(
    Path(id): Path<String>,
    Extension(commands_service): Extension<Arc<CommandsService>>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("in run_command id = {}", id);

    let result = commands_service.run_command(&id).await?;

    Ok(Json(result))
}

pub fn router() -> Router {
    let commands_service = CommandsService::new();

    Router::new()
        .route("/commands", get(get_all_commands))
        .route("/commands/:id", get(run_command))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(commands_service))
                .into_inner(),
        )
}
