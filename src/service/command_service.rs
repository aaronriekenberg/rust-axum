use axum::async_trait;

use serde::Serialize;

use std::{collections::HashMap, process::Stdio, sync::Arc};

use tokio::{
    process::Command,
    sync::{Semaphore, SemaphorePermit},
    time::{Duration, Instant},
};

use tracing::warn;

use crate::{
    config::{self, CommandInfo},
    utils::time::current_timestamp_string,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CommandID(pub String);

#[async_trait]
pub trait CommandsService {
    fn all_commands(&self, external_request: bool) -> Vec<&'static config::CommandInfo>;

    async fn run_command(
        &self,
        external_request: bool,
        command_id: CommandID,
    ) -> Result<RunCommandDTO, RunCommandError>;
}

pub type DynCommandsService = Arc<dyn CommandsService + Send + Sync>;

#[derive(Debug, Serialize)]
pub struct RunCommandDTO {
    now: String,
    command_duration_ms: u128,
    command_info: &'static config::CommandInfo,
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
    all_command_info: Vec<&'static config::CommandInfo>,
    external_command_info: Vec<&'static config::CommandInfo>,
    id_to_command_info: HashMap<CommandID, &'static config::CommandInfo>,
    semapore: Semaphore,
    semapore_acquire_timeout: Duration,
}

impl CommandsServiceImpl {
    fn new() -> Arc<Self> {
        let command_configuration = &config::instance().command_configuration;

        Arc::new(Self {
            all_command_info: command_configuration.commands.iter().collect(),
            external_command_info: command_configuration
                .commands
                .iter()
                .filter(|ci| !ci.internal_only)
                .collect(),
            id_to_command_info: command_configuration
                .commands
                .iter()
                .map(|command_config| (CommandID(command_config.id.clone()), command_config))
                .collect(),
            semapore: Semaphore::new(command_configuration.max_concurrent_commands),
            semapore_acquire_timeout: command_configuration.semaphore_acquire_timeout,
        })
    }

    async fn acquire_semaphore(&self) -> Result<SemaphorePermit<'_>, RunCommandError> {
        let result = tokio::time::timeout(self.semapore_acquire_timeout, self.semapore.acquire())
            .await
            .map_err(|error| {
                warn!(?error, "acquire_semapore timeout error");
                RunCommandError::SemaphoreAcquireError
            })?;

        let permit = result.map_err(|error| {
            warn!(?error, "acquire_semapore acquire error");
            RunCommandError::SemaphoreAcquireError
        })?;

        Ok(permit)
    }

    async fn internal_run_command(
        &self,
        command_info: &'static CommandInfo,
        permit: SemaphorePermit<'_>,
    ) -> RunCommandDTO {
        let command_start_time = Instant::now();
        let command_result = Command::new(&command_info.command)
            .args(&command_info.args)
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .output()
            .await;
        let command_duration = command_start_time.elapsed();

        drop(permit);

        RunCommandDTO {
            now: current_timestamp_string(),
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
        }
    }
}

#[async_trait]
impl CommandsService for CommandsServiceImpl {
    fn all_commands(&self, external_request: bool) -> Vec<&'static config::CommandInfo> {
        if external_request {
            self.external_command_info.clone()
        } else {
            self.all_command_info.clone()
        }
    }

    async fn run_command(
        &self,
        external_request: bool,
        command_id: CommandID,
    ) -> Result<RunCommandDTO, RunCommandError> {
        let command_info = self
            .id_to_command_info
            .get(&command_id)
            .ok_or(RunCommandError::CommandNotFound)?;

        if command_info.internal_only && external_request {
            warn!(
                ?command_id,
                "got external request for internal_only command",
            );
            return Err(RunCommandError::CommandNotFound);
        }

        let permit = self.acquire_semaphore().await?;

        Ok(self.internal_run_command(command_info, permit).await)
    }
}
