use axum::async_trait;

use chrono::prelude::{Local, SecondsFormat};

use serde::Serialize;

use std::{collections::HashMap, sync::Arc};

use tokio::{
    process::Command,
    sync::{Semaphore, SemaphorePermit},
    time::{Duration, Instant},
};

#[async_trait]
pub trait CommandsService {
    async fn run_command(&self, command_id: &str) -> Result<RunCommandResponse, RunCommandError>;
}

pub type DynCommandsService = Arc<dyn CommandsService + Send + Sync>;

#[derive(Debug, Serialize)]
pub struct RunCommandResponse {
    now: String,
    command_duration_ms: u128,
    command_info: &'static crate::config::CommandInfo,
    command_output: String,
}

#[derive(Debug)]
pub enum RunCommandError {
    CommandNotFound,
    SemaphoreAcquireError,
}

pub fn new_commands_service() -> DynCommandsService {
    CommandsServiceImpl::new()
}

struct CommandsServiceImpl {
    id_to_command_info: HashMap<String, &'static crate::config::CommandInfo>,
    semapore: Semaphore,
    semapore_acquire_timeout: Duration,
}

impl CommandsServiceImpl {
    fn new() -> Arc<Self> {
        let command_configuration = crate::config::instance().command_configuration();

        Arc::new(Self {
            id_to_command_info: command_configuration
                .commands()
                .iter()
                .map(|command_config| (command_config.id().clone(), command_config))
                .collect(),
            semapore: Semaphore::new(*command_configuration.max_concurrent_commands()),
            semapore_acquire_timeout: *command_configuration.semaphore_acquire_timeout(),
        })
    }

    async fn acquire_semaphore(&self) -> Result<SemaphorePermit<'_>, RunCommandError> {
        let result = tokio::time::timeout(self.semapore_acquire_timeout, self.semapore.acquire())
            .await
            .map_err(|_| RunCommandError::SemaphoreAcquireError)?;

        let permit = result.map_err(|_| RunCommandError::SemaphoreAcquireError)?;

        Ok(permit)
    }
}

#[async_trait]
impl CommandsService for CommandsServiceImpl {
    async fn run_command(&self, command_id: &str) -> Result<RunCommandResponse, RunCommandError> {
        let command_info = *self
            .id_to_command_info
            .get(command_id)
            .ok_or(RunCommandError::CommandNotFound)?;

        let permit = self.acquire_semaphore().await?;

        let command_start_time = Instant::now();
        let command_result = Command::new(command_info.command())
            .args(command_info.args())
            .output()
            .await;
        let command_duration = command_start_time.elapsed();

        drop(permit);

        Ok(RunCommandResponse {
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
        })
    }
}

fn current_time_string() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
