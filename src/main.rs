mod commands;
mod config;
mod request_info;

use axum::{error_handling::HandleErrorLayer, http::StatusCode, Router};

use std::{net::SocketAddr, time::Duration};
use tower::{BoxError, ServiceBuilder};

use tower_http::trace::TraceLayer;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_file = std::env::args()
        .nth(1)
        .expect("config file required as command line argument");

    config::read_configuration(config_file).await;

    // Compose the routes
    let app = Router::new()
        .merge(request_info::router())
        .merge(commands::router())
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        );

    let addr: SocketAddr = config::instance()
        .server_configuration()
        .bind_address()
        .parse()
        .expect("error parsing addr");

    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Server.serve error");
}
