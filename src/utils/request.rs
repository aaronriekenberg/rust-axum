use axum::http::Request;

use tower_http::request_id::{MakeRequestId, RequestId};

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

// A `MakeRequestId` that increments an atomic counter
#[derive(Clone, Default)]
pub struct CounterRequestId {
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
