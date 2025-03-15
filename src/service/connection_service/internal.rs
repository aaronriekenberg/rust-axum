use tokio::time::{Duration, Instant};

use tracing::debug;

use std::{
    cmp,
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
};

use super::{
    CONNECTION_METRICS_ORDERING, ConnectionCounterMetricName, ConnectionGuard, ConnectionID,
    ConnectionInfo, ConnectionTrackerServiceImpl,
};

#[derive(Default)]
struct ConnectionTrackerMetrics {
    max_open_connections: usize,
    past_min_connection_age: Option<Duration>,
    past_max_connection_age: Duration,
    past_max_requests_per_connection: usize,
}

impl ConnectionTrackerMetrics {
    fn update_for_new_connection(&mut self, new_num_connections: usize) {
        self.max_open_connections = cmp::max(self.max_open_connections, new_num_connections);
    }

    fn update_for_removed_connection(&mut self, removed_connection_info: &ConnectionInfo) {
        let removed_connection_age = removed_connection_info.age(Instant::now());

        self.past_min_connection_age = Some(cmp::min(
            self.past_min_connection_age.unwrap_or(Duration::MAX),
            removed_connection_age,
        ));

        self.past_max_connection_age =
            cmp::max(self.past_max_connection_age, removed_connection_age);

        self.past_max_requests_per_connection = cmp::max(
            self.past_max_requests_per_connection,
            removed_connection_info.num_requests(),
        );
    }
}
#[derive(Default)]
pub struct ConnectionCounterMetrics {
    connection_errors: AtomicUsize,
    connection_initial_timeouts: AtomicUsize,
    connection_final_timeouts: AtomicUsize,
}

impl ConnectionCounterMetrics {
    fn metric(&self, name: ConnectionCounterMetricName) -> &AtomicUsize {
        match name {
            ConnectionCounterMetricName::Errors => &self.connection_errors,
            ConnectionCounterMetricName::InitialTimeouts => &self.connection_initial_timeouts,
            ConnectionCounterMetricName::FinalTimeouts => &self.connection_final_timeouts,
        }
    }
    pub fn increment(&self, name: ConnectionCounterMetricName) {
        self.metric(name).fetch_add(1, CONNECTION_METRICS_ORDERING);
    }

    pub fn load(&self, name: ConnectionCounterMetricName) -> usize {
        self.metric(name).load(CONNECTION_METRICS_ORDERING)
    }
}

#[derive(Default)]
pub struct ConnectionTrackerState {
    previous_connection_id: usize,
    id_to_connection_info: HashMap<ConnectionID, Arc<ConnectionInfo>>,
    metrics: ConnectionTrackerMetrics,
}

impl ConnectionTrackerState {
    fn next_connection_id(&mut self) -> ConnectionID {
        let connection_id = self.previous_connection_id + 1;
        self.previous_connection_id = connection_id;
        ConnectionID(connection_id)
    }

    pub fn add_connection(
        &mut self,
        connection_tracker_service: Arc<ConnectionTrackerServiceImpl>,
    ) -> ConnectionGuard {
        let connection_id = self.next_connection_id();

        let connection_info = Arc::new(ConnectionInfo::new(connection_id));

        let num_requests = Arc::clone(&connection_info.num_requests);

        self.id_to_connection_info
            .insert(connection_id, connection_info);

        let new_num_connections = self.id_to_connection_info.len();

        self.metrics.update_for_new_connection(new_num_connections);

        debug!(new_num_connections, "add_connection");

        ConnectionGuard::new(connection_id, num_requests, connection_tracker_service)
    }

    pub fn remove_connection(&mut self, connection_id: ConnectionID) {
        if let Some(connection_info) = self.id_to_connection_info.remove(&connection_id) {
            self.metrics.update_for_removed_connection(&connection_info);
        }

        debug!(
            id_to_connection_info.len = self.id_to_connection_info.len(),
            "remove_connection"
        );
    }

    pub fn max_open_connections(&self) -> usize {
        self.metrics.max_open_connections
    }

    pub fn min_connection_lifetime(&self) -> Duration {
        match self.metrics.past_min_connection_age {
            Some(past_min_connection_age) => past_min_connection_age,
            None => {
                let now = Instant::now();
                self.id_to_connection_info
                    .values()
                    .map(|c| c.age(now))
                    .min()
                    .unwrap_or_default()
            }
        }
    }

    pub fn max_connection_lifetime(&self) -> Duration {
        let now = Instant::now();
        cmp::max(
            self.metrics.past_max_connection_age,
            self.id_to_connection_info
                .values()
                .map(|c| c.age(now))
                .max()
                .unwrap_or_default(),
        )
    }

    pub fn max_requests_per_connection(&self) -> usize {
        cmp::max(
            self.metrics.past_max_requests_per_connection,
            self.id_to_connection_info
                .values()
                .map(|c| c.num_requests())
                .max()
                .unwrap_or_default(),
        )
    }

    pub fn open_connections(&self) -> impl Iterator<Item = &Arc<ConnectionInfo>> {
        self.id_to_connection_info.values()
    }
}
