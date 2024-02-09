mod commands;
mod config;
mod request_info;
mod server;
mod version_info;

use anyhow::Context;

use tracing::error;

async fn try_main() -> anyhow::Result<()> {
    let config_file = std::env::args()
        .nth(1)
        .context("config file required as command line argument")?;

    config::read_configuration(config_file).await?;

    server::run().await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if let Err(err) = try_main().await {
        error!("fatal error in main:\n{:#}", err);
        std::process::exit(1);
    }
}
