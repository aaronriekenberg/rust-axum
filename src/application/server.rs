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

use tower::Service;

use tracing::{debug, info, instrument, warn};

use std::{convert::Infallible, path::Path, sync::Arc};

use crate::{
    config::ServerConfiguration,
    service::connection_service::{ConnectionGuard, ConnectionID, DynConnectionTrackerService},
};

pub async fn run(
    routes: Router,
    server_configuration: &ServerConfiguration,
    connection_tracker_service: DynConnectionTrackerService,
) -> anyhow::Result<()> {
    let listener = create_listener(server_configuration).await?;

    let mut make_service = routes.into_make_service_with_connect_info::<ConnectionID>();

    loop {
        let (socket, _remote_addr) = listener.accept().await.context("listener accept error")?;

        let connection_guard = Arc::clone(&connection_tracker_service)
            .add_connection()
            .await;

        let tower_service = unwrap_infallible(make_service.call(&connection_guard.id).await);

        tokio::spawn(handle_connection(connection_guard, socket, tower_service));
    }
}

async fn create_listener(
    server_configuration: &ServerConfiguration,
) -> anyhow::Result<UnixListener> {
    let path = Path::new(&server_configuration.unix_socket_path);

    let remove_result = tokio::fs::remove_file(path).await;
    debug!("remove_result = {:?}", remove_result);

    let unix_listener = UnixListener::bind(path).context("UnixListener::bind error")?;
    info!("listening on unix path: {:?}", path);

    Ok(unix_listener)
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
