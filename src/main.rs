mod api;
mod config;
mod server;
mod version;

use anyhow::Context;

use tracing::{error, info};

fn log_version_info() {
    info!("Version Info:");
    for (key, value) in crate::version::get_verison_info() {
        info!("{}: {}", key, value);
    }
}

fn app_name() -> String {
    std::env::args().next().unwrap_or("[UNKNOWN]".to_owned())
}

async fn try_main() -> anyhow::Result<()> {
    log_version_info();

    let config_file = std::env::args().nth(1).with_context(|| {
        format!(
            "config file required as command line argument: {} <config file>",
            app_name(),
        )
    })?;

    crate::config::read_configuration(config_file).await?;

    crate::server::run().await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = try_main().await {
        error!("fatal error in main:\n{:#}", err);
        std::process::exit(1);
    }
}
