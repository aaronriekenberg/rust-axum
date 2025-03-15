mod server;

use tower::ServiceBuilder;

use tower_http::{
    ServiceBuilderExt,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use std::sync::Arc;

use crate::{config, controller, service, utils};

pub async fn run(config_file: String) -> anyhow::Result<()> {
    config::read_configuration(config_file).await?;

    let server_configuration = &config::instance().server_configuration;

    let command_service = service::command_service::new_commands_service();

    let connection_tracker_service = service::connection_service::new_connection_tracker_service();

    let routes = controller::create_routes(
        server_configuration,
        command_service,
        Arc::clone(&connection_tracker_service),
    );

    let routes = routes
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // make sure to set request ids before the request reaches `TraceLayer`
                .set_x_request_id(utils::request::CounterRequestId::default())
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

    self::server::run(routes, server_configuration, connection_tracker_service).await
}
