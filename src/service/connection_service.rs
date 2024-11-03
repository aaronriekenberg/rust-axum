mod internal;

use axum::async_trait;

use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};

use serde::Serialize;

use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicUsize, Arc},
    time::SystemTime,
};

use crate::utils::time::system_time_to_string;

const CONNECTION_METRICS_ORDERING: std::sync::atomic::Ordering =
    std::sync::atomic::Ordering::Relaxed;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ConnectionID(usize);

impl ConnectionID {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub enum ConnectionCounterMetricName {
    Errors,
    InitialTimeouts,
    FinalTimeouts,
}

#[derive(Debug)]
struct ConnectionInfo {
    id: ConnectionID,
    creation_time: SystemTime,
    creation_instant: Instant,
    num_requests: Arc<AtomicUsize>,
}

impl ConnectionInfo {
    fn new(id: ConnectionID) -> Self {
        Self {
            id,
            creation_time: SystemTime::now(),
            creation_instant: Instant::now(),
            num_requests: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn num_requests(&self) -> usize {
        self.num_requests.load(CONNECTION_METRICS_ORDERING)
    }

    fn age(&self, now: Instant) -> Duration {
        now - self.creation_instant
    }
}

pub struct ConnectionGuard {
    pub id: ConnectionID,
    num_requests: Arc<AtomicUsize>,
    connection_tracker_service: Arc<ConnectionTrackerServiceImpl>,
}

impl ConnectionGuard {
    fn new(
        id: ConnectionID,
        num_requests: Arc<AtomicUsize>,
        connection_tracker_service: Arc<ConnectionTrackerServiceImpl>,
    ) -> Self {
        Self {
            id,
            num_requests,
            connection_tracker_service,
        }
    }

    pub fn increment_num_requests(&self) {
        self.num_requests.fetch_add(1, CONNECTION_METRICS_ORDERING);
    }

    pub fn num_requests(&self) -> usize {
        self.num_requests.load(CONNECTION_METRICS_ORDERING)
    }

    pub fn increment_counter_metric(&self, name: ConnectionCounterMetricName) {
        self.connection_tracker_service
            .increment_counter_metric(name);
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let id = self.id;
        let connection_tracker_service = Arc::clone(&self.connection_tracker_service);

        tokio::spawn(async move {
            connection_tracker_service.remove_connection(id).await;
        });
    }
}

#[async_trait]
pub trait ConnectionTrackerService {
    async fn add_connection(self: Arc<Self>) -> ConnectionGuard;
    async fn state_snapshot_dto(self: Arc<Self>) -> ConnectionTrackerStateSnapshotDTO;
}

pub type DynConnectionTrackerService = Arc<dyn ConnectionTrackerService + Send + Sync>;

pub fn new_connection_tracker_service() -> DynConnectionTrackerService {
    ConnectionTrackerServiceImpl::new()
}

struct ConnectionTrackerServiceImpl {
    state: RwLock<internal::ConnectionTrackerState>,
    counter_metrics: internal::ConnectionCounterMetrics,
}

impl ConnectionTrackerServiceImpl {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(internal::ConnectionTrackerState::default()),
            counter_metrics: internal::ConnectionCounterMetrics::default(),
        })
    }

    async fn remove_connection(self: Arc<Self>, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        state.remove_connection(connection_id);
    }

    async fn connection_tracker_state_snapshot(self: Arc<Self>) -> ConnectionTrackerStateSnapshot {
        let state = self.state.read().await;

        ConnectionTrackerStateSnapshot {
            max_open_connections: state.max_open_connections(),
            min_connection_lifetime: state.min_connection_lifetime(),
            max_connection_lifetime: state.max_connection_lifetime(),
            max_requests_per_connection: state.max_requests_per_connection(),
            connection_errors: self
                .counter_metrics
                .load(ConnectionCounterMetricName::Errors),
            connection_initial_timeouts: self
                .counter_metrics
                .load(ConnectionCounterMetricName::InitialTimeouts),
            connection_final_timeouts: self
                .counter_metrics
                .load(ConnectionCounterMetricName::FinalTimeouts),
            open_connections: state.open_connections().cloned().collect(),
        }
    }

    fn increment_counter_metric(&self, name: ConnectionCounterMetricName) {
        self.counter_metrics.increment(name);
    }
}

#[async_trait]
impl ConnectionTrackerService for ConnectionTrackerServiceImpl {
    async fn add_connection(self: Arc<Self>) -> ConnectionGuard {
        let mut state = self.state.write().await;

        state.add_connection(Arc::clone(&self))
    }

    async fn state_snapshot_dto(self: Arc<Self>) -> ConnectionTrackerStateSnapshotDTO {
        self.connection_tracker_state_snapshot().await.into()
    }
}

struct ConnectionTrackerStateSnapshot {
    max_open_connections: usize,
    min_connection_lifetime: Duration,
    max_connection_lifetime: Duration,
    max_requests_per_connection: usize,
    connection_errors: usize,
    connection_initial_timeouts: usize,
    connection_final_timeouts: usize,
    open_connections: Vec<Arc<ConnectionInfo>>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionInfoSnapshotDTO {
    id: usize,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
    num_requests: usize,
}

impl From<Arc<ConnectionInfo>> for ConnectionInfoSnapshotDTO {
    fn from(connection_info: Arc<ConnectionInfo>) -> Self {
        // truncate to seconds
        let age = Duration::from_secs(connection_info.age(Instant::now()).as_secs());

        Self {
            id: connection_info.id.as_usize(),
            creation_time: system_time_to_string(connection_info.creation_time),
            age,
            num_requests: connection_info.num_requests(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConnectionTrackerStateSnapshotDTO {
    max_open_connections: usize,
    #[serde(with = "humantime_serde")]
    min_connection_lifetime: Duration,
    #[serde(with = "humantime_serde")]
    max_connection_lifetime: Duration,
    max_requests_per_connection: usize,
    connection_errors: usize,
    connection_initial_timeouts: usize,
    connection_final_timeouts: usize,
    num_open_connections: usize,
    open_connections: Vec<ConnectionInfoSnapshotDTO>,
}

impl From<ConnectionTrackerStateSnapshot> for ConnectionTrackerStateSnapshotDTO {
    fn from(state_snapshot: ConnectionTrackerStateSnapshot) -> Self {
        let id_to_open_connection: BTreeMap<ConnectionID, Arc<ConnectionInfo>> = state_snapshot
            .open_connections
            .into_iter()
            .map(|c| (c.id, c))
            .collect();

        let num_open_connections = id_to_open_connection.len();

        // 20 newest connections with descending ids in reverse order
        let open_connections = id_to_open_connection
            .into_iter()
            .rev()
            .take(20)
            .map(|(_, v)| v.into())
            .collect();

        // truncate to seconds
        let min_connection_lifetime =
            Duration::from_secs(state_snapshot.min_connection_lifetime.as_secs());

        // truncate to seconds
        let max_connection_lifetime =
            Duration::from_secs(state_snapshot.max_connection_lifetime.as_secs());

        Self {
            max_open_connections: state_snapshot.max_open_connections,
            min_connection_lifetime,
            max_connection_lifetime,
            max_requests_per_connection: state_snapshot.max_requests_per_connection,
            connection_errors: state_snapshot.connection_errors,
            connection_initial_timeouts: state_snapshot.connection_initial_timeouts,
            connection_final_timeouts: state_snapshot.connection_final_timeouts,
            num_open_connections,
            open_connections,
        }
    }
}
