mod config;
mod controller;
mod server;
mod service;

use anyhow::Context;

use tracing::error;

fn app_name() -> String {
    std::env::args().next().unwrap_or("[UNKNOWN]".to_owned())
}

async fn try_main() -> anyhow::Result<()> {
    let config_file = std::env::args().nth(1).with_context(|| {
        format!(
            "config file required as command line argument: {} <config file>",
            app_name(),
        )
    })?;

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
