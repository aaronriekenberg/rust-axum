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

    async fn run_command(self: Arc<Self>, id: &String) -> Option<RunCommandResponse> {
        let command_info = match self.id_to_command_info.get(id).cloned() {
            Some(command_info) => command_info,
            None => return None,
        };

        async fn execute_command(
            command_info: &crate::config::CommandInfo,
        ) -> Result<std::process::Output, std::io::Error> {
            let output = Command::new(command_info.command())
                .args(command_info.args())
                .output()
                .await?;

            Ok(output)
        }

        let command_start_time = Instant::now();
        let command_result = execute_command(command_info).await;
        let command_duration = command_start_time.elapsed();

        let response = RunCommandResponse {
            now: current_time_string(),
            command_duration_ms: command_duration.as_millis(),
            command_info,
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

        Some(response)
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

    match commands_service.run_command(&id).await {
        Some(result) => Ok(Json(result)),
        None => Err(StatusCode::NOT_FOUND),
    }
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
