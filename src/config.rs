use tracing::info;

use serde::{Deserialize, Serialize};

use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell};

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfiguration {
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandInfo {
    pub id: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandConfiguration {
    pub max_concurrent_commands: usize,
    #[serde(with = "humantime_serde")]
    pub semaphore_acquire_timeout: std::time::Duration,
    pub commands: Vec<CommandInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub server_configuration: ServerConfiguration,
    pub command_configuration: CommandConfiguration,
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

    let file_contents_string =
        String::from_utf8(file_contents).expect("String::from_utf8 error reading config file");

    let configuration: Configuration =
        ::toml::from_str(&file_contents_string).expect("error unmarshalling config file");

    info!("configuration\n{:#?}", configuration);

    CONFIGURATION_INSTANCE
        .set(configuration)
        .expect("CONFIGURATION_INSTANCE.set error");
}

pub fn instance() -> &'static Configuration {
    CONFIGURATION_INSTANCE.get().unwrap()
}
