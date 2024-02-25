use anyhow::Context;

use axum::{
    extract::{connect_info, ConnectInfo},
    middleware::AddExtension,
    Router,
};

use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};

use tokio::net::{UnixListener, UnixStream};

use tower::{Service, ServiceBuilder};

use tower_http::{
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};

use tracing::{debug, info, instrument, warn};

use std::{convert::Infallible, path::PathBuf, sync::Arc};

use crate::{
    config::{self, ServerConfiguration},
    controller,
    service::{
        self,
        connection_service::{ConnectionGuard, ConnectionID, DynConnectionTrackerService},
    },
    utils::request::CounterRequestId,
};

pub async fn run(config_file: String) -> anyhow::Result<()> {
    config::read_configuration(config_file).await?;

    let server_configuration = &config::instance().server_configuration;

    let command_service = service::command_service::new_commands_service();

    let connection_tracker_service = service::connection_service::new_connection_tracker_service();

    let api_routes =
        controller::create_api_routes(command_service, Arc::clone(&connection_tracker_service));

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

    run_server(routes, server_configuration, connection_tracker_service).await
}

async fn create_listener(
    server_configuration: &ServerConfiguration,
) -> anyhow::Result<UnixListener> {
    let path = PathBuf::from(&server_configuration.unix_socket_path);

    let remove_result = tokio::fs::remove_file(&path).await;
    debug!("remove_result = {:?}", remove_result);

    let unix_listener = UnixListener::bind(&path).context("UnixListener::bind error")?;

    info!("listening on uds path: {:?}", path);

    Ok(unix_listener)
}

async fn run_server(
    routes: Router,
    server_configuration: &ServerConfiguration,
    connection_tracker_service: DynConnectionTrackerService,
) -> anyhow::Result<()> {
    let listener = create_listener(server_configuration).await?;

    let mut make_service = routes.into_make_service_with_connect_info::<ConnectionID>();

    loop {
        let connection_tracker_service_clone = Arc::clone(&connection_tracker_service);

        let (socket, _remote_addr) = listener.accept().await.context("listener accept error")?;

        let connection_guard = connection_tracker_service_clone.add_connection().await;

        let tower_service = unwrap_infallible(make_service.call(&connection_guard.id).await);

        tokio::spawn(handle_connection(connection_guard, socket, tower_service));
    }
}

#[instrument(
    name = "conn",
    skip_all,
    fields(
        id = connection_guard.id.as_usize(),
    )
)]
async fn handle_connection(
    connection_guard: ConnectionGuard,
    socket: UnixStream,
    tower_service: AddExtension<Router, ConnectInfo<ConnectionID>>,
) {
    info!("begin handle_connection");

    let socket = TokioIo::new(socket);

    let hyper_service = hyper::service::service_fn(|request| {
        connection_guard.increment_num_requests();
        tower_service.clone().call(request)
    });

    if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
        .serve_connection(socket, hyper_service)
        .await
    {
        warn!("failed to serve connection: {err:#}");
    }

    info!(
        "end handle_connection num requests = {}",
        connection_guard.num_requests(),
    );
}

fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => match err {},
    }
}

impl connect_info::Connected<&ConnectionID> for ConnectionID {
    fn connect_info(id: &ConnectionID) -> Self {
        debug!("in connect_info id = {id:?}");
        *id
    }
}
