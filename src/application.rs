use anyhow::Context;

use axum::{http::Request, Router};

use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};

use tokio::{net::UnixListener, task::JoinHandle};

use tower::{Service, ServiceBuilder};

use tower_http::{
    request_id::{MakeRequestId, RequestId},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};

use tracing::{debug, info, warn};

use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use crate::{config, controller, service};

pub async fn run(config_file: String) -> anyhow::Result<()> {
    config::read_configuration(config_file).await?;

    let server_configuration = &config::instance().server_configuration;

    let command_service = service::command_service::new_commands_service();

    let api_routes = controller::create_api_routes(command_service);

    let routes = Router::new()
        .nest("/api/v1", api_routes)
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // make sure to set request ids before the request reaches `TraceLayer`
                .set_x_request_id(CounterRequestId::default())
                // log requests and responses
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_response(DefaultOnResponse::new().include_headers(true)),
                )
                // propagate the header to the response before the response reaches `TraceLayer`
                .propagate_x_request_id()
                .layer(TimeoutLayer::new(server_configuration.request_timeout))
                .into_inner(),
        );

    let path = PathBuf::from(&server_configuration.unix_socket_path);

    let remove_result = tokio::fs::remove_file(&path).await;
    debug!("remove_result = {:?}", remove_result);

    let uds = UnixListener::bind(&path).context("UnixListener::bind error")?;

    info!("listening on uds path: {:?}", path);

    let event_loop_task_result: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let mut make_service = routes.into_make_service();

        loop {
            let (socket, _remote_addr) = uds.accept().await.context("uds accept error")?;

            let tower_service = unwrap_infallible(make_service.call(&socket).await);

            tokio::spawn(async move {
                info!("accepted socket");

                let socket = TokioIo::new(socket);

                let hyper_service =
                    hyper::service::service_fn(move |request| tower_service.clone().call(request));

                if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                    .serve_connection(socket, hyper_service)
                    .await
                {
                    warn!("failed to serve connection: {err:#}");
                }

                info!("ending socket task");
            });
        }
    });

    let result = event_loop_task_result
        .await
        .context("event_loop_task_result JoinError")?;

    result.context("event loop returned error")?;

    anyhow::bail!("event_loop_task_result returned without error");
}

// A `MakeRequestId` that increments an atomic counter
#[derive(Clone, Default)]
struct CounterRequestId {
    counter: Arc<AtomicU64>,
}

impl MakeRequestId for CounterRequestId {
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

fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => match err {},
    }
}
