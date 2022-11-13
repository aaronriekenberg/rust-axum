use getset::Getters;

use tracing::info;

use serde::{Deserialize, Serialize};

use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell};

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct ServerConfiguration {
    bind_address: String,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CommandInfo {
    id: String,
    description: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CommandConfiguration {
    max_concurrent_commands: usize,
    #[serde(with = "humantime_serde")]
    semaphore_acquire_timeout: std::time::Duration,
    commands: Vec<CommandInfo>,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct Configuration {
    server_configuration: ServerConfiguration,
    command_configuration: CommandConfiguration,
}

static CONFIGURATION_INSTANCE: OnceCell<Configuration> = OnceCell::const_new();

pub async fn read_configuration(config_file: String) {
    info!("reading '{}'", config_file);

    let mut file = File::open(&config_file)
        .await
        .expect("error opening config file");

    let mut file_contents = Vec::new();

    file.read_to_end(&mut file_contents)
        .await
        .expect("error reading config file");

    let configuration: Configuration =
        ::serde_json::from_slice(&file_contents).expect("error unmarshalling config file");

    info!("configuration\n{:#?}", configuration);

    CONFIGURATION_INSTANCE
        .set(configuration)
        .expect("CONFIGURATION_INSTANCE.set error");
}

pub fn instance() -> &'static Configuration {
    CONFIGURATION_INSTANCE.get().unwrap()
}
