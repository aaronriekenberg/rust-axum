use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::connect_info;

use tokio::net::UnixStream;

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub connection_id: u64,
}

impl connect_info::Connected<&UnixStream> for ConnectionInfo {
    fn connect_info(_target: &UnixStream) -> Self {
        let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::SeqCst);
        Self { connection_id }
    }
}
