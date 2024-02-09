use anyhow::Context;
use axum::{http::Request, Router};

use tower::ServiceBuilder;

use tower_http::{
    request_id::{MakeRequestId, RequestId},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};

use tracing::info;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

pub async fn run() -> anyhow::Result<()> {
    let server_configuration = &crate::config::instance().server_configuration;

    let api_routes = Router::new()
        .nest("/commands", crate::commands::router())
        .nest("/request_info", crate::request_info::router())
        .nest("/version_info", crate::version_info::router());

    let app = Router::new()
        .nest("/api/v1", api_routes)
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // make sure to set request ids before the request reaches `TraceLayer`
                .set_x_request_id(MyMakeRequestId::default())
                // log requests and responses
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_response(DefaultOnResponse::new().include_headers(true)),
                )
                // propagate the header to the response before the response reaches `TraceLayer`
                .propagate_x_request_id()
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                .into_inner(),
        );

    let addr: SocketAddr = server_configuration
        .bind_address
        .parse()
        .context("error parsing bind_address")?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("TcpListener::bind error")?;

    info!("listening on {}", addr);

    axum::serve(listener, app)
        .await
        .context("axum::serve error")?;

    anyhow::bail!("axum::serve returned without error");
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
