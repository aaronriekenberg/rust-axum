mod commands;
mod config;
mod request_info;
mod server;
mod version_info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_file = std::env::args()
        .nth(1)
        .expect("config file required as command line argument");

    config::read_configuration(config_file).await;

    server::run().await;
}
