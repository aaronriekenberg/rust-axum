use anyhow::Context;

use axum::{
    extract::{connect_info, ConnectInfo},
    middleware::AddExtension,
    Router,
};

use hyper_util::{
    rt::{TokioExecutor, TokioIo, TokioTimer},
    server,
};

use tokio::net::{TcpListener, TcpStream};

use tower::Service;

use tracing::{debug, info, instrument, warn};

use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    config::ServerConfiguration,
    service::connection_service::{
        ConnectionCounterMetricName, ConnectionGuard, ConnectionID, DynConnectionTrackerService,
    },
};

pub async fn run(
    routes: Router,
    server_configuration: &ServerConfiguration,
    connection_tracker_service: DynConnectionTrackerService,
) -> anyhow::Result<()> {
    let listener = create_listener(server_configuration).await?;

    let mut make_service = routes.into_make_service_with_connect_info::<ConnectionID>();

    let connection_timeout_durations = [
        server_configuration.connection.max_lifetime,
        server_configuration.connection.graceful_shutdown_timeout,
    ];

    debug!(
        "connection_timeout_durations = {:?} tcp_nodelay = {}",
        connection_timeout_durations, server_configuration.connection.tcp_nodelay
    );

    loop {
        let (tcp_stream, remote_addr) = listener.accept().await.context("listener accept error")?;

        if server_configuration.connection.tcp_nodelay {
            debug!("calling tcp_stream.set_nodelay(true)");
            if let Err(e) = tcp_stream.set_nodelay(true) {
                warn!("error setting tcp no delay {:?}", e);
                continue;
            };
        } else {
            debug!("NOT calling tcp_stream.set_nodelay");
        }

        let connection_guard = Arc::clone(&connection_tracker_service)
            .add_connection()
            .await;

        let tower_service = unwrap_infallible(make_service.call(connection_guard.id).await);

        let connection = Connection {
            connection_guard,
            tcp_stream,
            remote_addr,
            connection_timeout_durations,
            tower_service,
        };

        tokio::spawn(connection.run());
    }
}

async fn create_listener(
    server_configuration: &ServerConfiguration,
) -> anyhow::Result<TcpListener> {
    let tcp_listener = TcpListener::bind(&server_configuration.bind_address)
        .await
        .with_context(|| {
            format!(
                "TCP server bind error address = {:?}",
                server_configuration.bind_address
            )
        })?;

    let local_addr = tcp_listener.local_addr().with_context(|| {
        format!(
            "TCP server local_addr error address = {:?}",
            server_configuration.bind_address
        )
    })?;

    info!("listening on tcp {:?}", local_addr);

    Ok(tcp_listener)
}

struct Connection {
    connection_guard: ConnectionGuard,
    tcp_stream: TcpStream,
    remote_addr: SocketAddr,
    connection_timeout_durations: [Duration; 2],
    tower_service: AddExtension<Router, ConnectInfo<ConnectionID>>,
}

impl Connection {
    #[instrument(
        name = "conn",
        skip_all,
        fields(
            id = self.connection_guard.id.as_usize(),
        )
    )]
    async fn run(self) {
        debug!("begin Connection::run remote_addr = {:?}", self.remote_addr);

        let socket = TokioIo::new(self.tcp_stream);

        let hyper_service = hyper::service::service_fn(|request| {
            self.connection_guard.increment_num_requests();
            self.tower_service.clone().call(request)
        });

        let mut builder = server::conn::auto::Builder::new(TokioExecutor::new());

        builder.http1().timer(TokioTimer::new());
        builder.http2().timer(TokioTimer::new());

        let hyper_conn = builder.serve_connection(socket, hyper_service);
        tokio::pin!(hyper_conn);

        for (iter, sleep_duration) in self.connection_timeout_durations.iter().enumerate() {
            debug!("iter = {} sleep_duration = {:?}", iter, sleep_duration);
            tokio::select! {
                res = hyper_conn.as_mut() => {
                    match res {
                        Ok(()) => debug!("after polling conn, no error"),
                        Err(e) => {
                            warn!("error serving connection: {:?}", e);
                            self.connection_guard.increment_counter_metric(ConnectionCounterMetricName::Errors);
                        },
                    };
                    break;
                }
                _ = tokio::time::sleep(*sleep_duration) => {
                    debug!("iter = {} got timeout_interval, calling conn.graceful_shutdown", iter);
                    hyper_conn.as_mut().graceful_shutdown();
                    if iter == 0 {
                        self.connection_guard.increment_counter_metric(ConnectionCounterMetricName::InitialTimeouts);
                    } else {
                        self.connection_guard.increment_counter_metric(ConnectionCounterMetricName::FinalTimeouts);
                    }
                }
            }
        }

        debug!(
            "end Connection::run num requests = {}",
            self.connection_guard.num_requests(),
        );
    }
}

fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    let Ok(result) = result;
    result
}

impl connect_info::Connected<ConnectionID> for ConnectionID {
    fn connect_info(id: ConnectionID) -> Self {
        debug!("in connect_info id = {id:?}");
        id
    }
}
