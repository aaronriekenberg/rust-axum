mod commands;
mod config;
mod request_info;

use axum::{
    error_handling::HandleErrorLayer,
    http::{Request, StatusCode},
    Router,
};

use tower::{BoxError, ServiceBuilder};

use tower_http::trace::TraceLayer;

use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};

use tracing::warn;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_file = std::env::args()
        .nth(1)
        .expect("config file required as command line argument");

    config::read_configuration(config_file).await;

    let api_routes = Router::new()
        .nest("/request_info", request_info::router())
        .nest("/commands", commands::router());

    let app = Router::new()
        .nest("/api/v1", api_routes)
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // set `x-request-id` header on all requests
                .layer(SetRequestIdLayer::x_request_id(MyMakeRequestId::default()))
                // propagate `x-request-id` headers from request to response
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        warn!("got tower::timeout::error::Elapsed error");
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        warn!("got unknown error: {}", error);
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
        .server_configuration
        .bind_address
        .parse()
        .expect("error parsing addr");

    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("TcpListener::bind error");

    axum::serve(listener, app).await.expect("axum::serve error");
}

// A `MakeRequestId` that increments an atomic counter
#[derive(Clone, Default)]
struct MyMakeRequestId {
    counter: Arc<AtomicU64>,
}

impl MakeRequestId for MyMakeRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = self
            .counter
            .fetch_add(1, Ordering::SeqCst)
            .to_string()
            .parse()
            .unwrap();

        Some(RequestId::new(request_id))
    }
}
