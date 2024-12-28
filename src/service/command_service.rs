use axum::async_trait;

use itertools::Itertools;

use serde::Serialize;

use std::{collections::HashMap, process::Stdio, sync::Arc};

use tokio::{
    process::Command,
    sync::{Semaphore, SemaphorePermit},
    time::{Duration, Instant},
};

use tracing::warn;

use crate::{config, utils::time::current_timestamp_string};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CommandID(pub String);

#[async_trait]
pub trait CommandsService {
    fn all_commands(&self, external_request: bool) -> Vec<CommandInfoDTO>;

    async fn run_command(
        &self,
        external_request: bool,
        command_id: CommandID,
    ) -> Result<RunCommandDTO, RunCommandError>;
}

pub type DynCommandsService = Arc<dyn CommandsService + Send + Sync>;

#[derive(Clone, Debug, Serialize)]
pub struct CommandInfoDTO {
    pub id: &'static String,
    pub description: &'static String,
    pub command: &'static String,
    pub args: &'static Vec<String>,
}

impl From<&'static config::CommandInfo> for CommandInfoDTO {
    fn from(command_info: &'static config::CommandInfo) -> Self {
        Self {
            id: &command_info.id,
            description: &command_info.description,
            command: &command_info.command,
            args: &command_info.args,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RunCommandDTO {
    now: String,
    command_duration_ms: u128,
    command_info: CommandInfoDTO,
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
    all_command_info_dtos: Vec<CommandInfoDTO>,
    external_command_info_dtos: Vec<CommandInfoDTO>,
    id_to_command_info: HashMap<CommandID, &'static config::CommandInfo>,
    semapore: Semaphore,
    semapore_acquire_timeout: Duration,
}

impl CommandsServiceImpl {
    fn new() -> Arc<Self> {
        let command_configuration = &config::instance().command_configuration;

        Arc::new(Self {
            all_command_info_dtos: command_configuration.commands.iter().map_into().collect(),
            external_command_info_dtos: command_configuration
                .commands
                .iter()
                .filter(|ci| !ci.internal_only)
                .map_into()
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

    async fn internal_run_command<'a>(
        &self,
        command_info: &'static config::CommandInfo,
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
            command_info: command_info.into(),
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
    fn all_commands(&self, external_request: bool) -> Vec<CommandInfoDTO> {
        if external_request {
            self.external_command_info_dtos.clone()
        } else {
            self.all_command_info_dtos.clone()
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
