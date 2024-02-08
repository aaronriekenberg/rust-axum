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

use tracing::{info, warn};

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
