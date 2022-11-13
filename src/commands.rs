use axum::{
    error_handling::HandleErrorLayer,
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

struct CommandsService {}

impl CommandsService {
    fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

async fn get_all_commands() -> impl IntoResponse {
    Json(crate::config::instance().command_configuration().commands())
}

pub fn router() -> Router {
    let commands_service = CommandsService::new();

    Router::new()
        .route("/commands", get(get_all_commands))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(commands_service))
                .into_inner(),
        )
}
