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

pub async fn run() {
    let configuration = crate::config::instance();

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

    let addr: SocketAddr = configuration
        .server_configuration
        .bind_address
        .parse()
        .expect("error parsing addr");

    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
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
