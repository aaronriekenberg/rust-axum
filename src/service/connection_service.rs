mod internal;

use axum::async_trait;

use chrono::prelude::{DateTime, Local, SecondsFormat};

use tokio::{
    sync::RwLock,
    time::{Duration, Instant},
};

use serde::Serialize;

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::SystemTime,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ConnectionID(usize);

impl ConnectionID {
    pub fn as_usize(&self) -> usize {
        self.0
    }
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
        self.num_requests.load(Ordering::Relaxed)
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
        self.num_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn num_requests(&self) -> usize {
        self.num_requests.load(Ordering::Relaxed)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let id = self.id;
        let connection_tracker_service = Arc::clone(&self.connection_tracker_service);

        tokio::task::spawn(async move {
            connection_tracker_service.remove_connection(id).await;
        });
    }
}

#[async_trait]
pub trait ConnectionTrackerService {
    async fn add_connection(self: Arc<Self>) -> ConnectionGuard;
    async fn state(self: Arc<Self>) -> ConnectionTrackerStateDTO;
}

pub type DynConnectionTrackerService = Arc<dyn ConnectionTrackerService + Send + Sync>;

pub fn new_connection_tracker_service() -> DynConnectionTrackerService {
    ConnectionTrackerServiceImpl::new()
}

struct ConnectionTrackerServiceImpl {
    state: RwLock<internal::ConnectionTrackerState>,
}

impl ConnectionTrackerServiceImpl {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(internal::ConnectionTrackerState::new()),
        })
    }

    async fn remove_connection(self: Arc<Self>, connection_id: ConnectionID) {
        let mut state = self.state.write().await;

        state.remove_connection(connection_id);
    }

    async fn connection_tracker_state(self: Arc<Self>) -> ConnectionTrackerState {
        let state = self.state.read().await;

        ConnectionTrackerState {
            max_open_connections: state.max_open_connections(),
            max_connection_age: state.max_connection_age(),
            max_requests_per_connection: state.max_requests_per_connection(),
            open_connections: state.open_connections().cloned().collect(),
        }
    }
}

#[async_trait]
impl ConnectionTrackerService for ConnectionTrackerServiceImpl {
    async fn add_connection(self: Arc<Self>) -> ConnectionGuard {
        let mut state = self.state.write().await;

        state.add_connection(Arc::clone(&self))
    }

    async fn state(self: Arc<Self>) -> ConnectionTrackerStateDTO {
        self.connection_tracker_state().await.into()
    }
}

struct ConnectionTrackerState {
    max_open_connections: usize,
    max_connection_age: Duration,
    max_requests_per_connection: usize,
    open_connections: Vec<Arc<ConnectionInfo>>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionInfoDTO {
    id: usize,
    creation_time: String,
    #[serde(with = "humantime_serde")]
    age: Duration,
    num_requests: usize,
}

impl From<Arc<ConnectionInfo>> for ConnectionInfoDTO {
    fn from(connection_info: Arc<ConnectionInfo>) -> Self {
        // truncate to seconds
        let age = Duration::from_secs(connection_info.age(Instant::now()).as_secs());

        Self {
            id: connection_info.id.as_usize(),
            creation_time: local_date_time_to_string(&LocalDateTime::from(
                connection_info.creation_time,
            )),
            age,
            num_requests: connection_info.num_requests(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConnectionTrackerStateDTO {
    max_open_connections: usize,
    #[serde(with = "humantime_serde")]
    max_connection_lifetime: Duration,
    max_requests_per_connection: usize,
    num_open_connections: usize,
    open_connections: Vec<ConnectionInfoDTO>,
}

impl From<ConnectionTrackerState> for ConnectionTrackerStateDTO {
    fn from(state: ConnectionTrackerState) -> Self {
        let id_to_open_connection: BTreeMap<ConnectionID, Arc<ConnectionInfo>> = state
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
        let max_connection_lifetime = Duration::from_secs(state.max_connection_age.as_secs());

        Self {
            max_open_connections: state.max_open_connections,
            max_connection_lifetime,
            max_requests_per_connection: state.max_requests_per_connection,
            num_open_connections,
            open_connections,
        }
    }
}

type LocalDateTime = DateTime<Local>;

fn local_date_time_to_string(local_date_time: &LocalDateTime) -> String {
    local_date_time.to_rfc3339_opts(SecondsFormat::Millis, false)
}
