mod application;
mod config;
mod controller;
mod service;
mod utils;

use anyhow::Context;

use tracing::{error, info};

fn log_version_info() {
    for (key, value) in crate::service::version_service::verison_info() {
        info!(key, value, "version info");
    }
}

fn process_name() -> String {
    std::env::args().next().unwrap_or("[UNKNOWN]".to_owned())
}

async fn try_main() -> anyhow::Result<()> {
    log_version_info();

    let config_file = std::env::args().nth(1).with_context(|| {
        format!(
            "config file required as command line argument: {} <config file>",
            process_name(),
        )
    })?;

    application::run(config_file).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(error) = try_main().await {
        error!(?error, "fatal error in main");
        std::process::exit(1);
    }
}
