use std::{sync::{atomic::{AtomicBool, AtomicI32, Ordering}, Condvar, Mutex}, time::Duration};

pub(crate) struct ProofShared {
    pub(crate) view: AtomicI32,
    pub(crate) busy: AtomicBool,
    results: Mutex<Vec<(String, i32, i64)>>,
    ready: Condvar,
}
impl Default for ProofShared {
    fn default() -> Self { Self { view: AtomicI32::new(-1), busy: AtomicBool::new(false), results: Mutex::new(Vec::new()), ready: Condvar::new() } }
}
impl ProofShared {
    pub(crate) fn push_result(&self, op: impl Into<String>, code: i32, value: i64) {
        self.results.lock().unwrap().push((op.into(), code, value));
        self.ready.notify_all();
    }
    pub(crate) fn wait_result(&self, op: &str, timeout: Duration) -> Option<(i32, i64)> {
        let end = std::time::Instant::now() + timeout;
        let mut entries = self.results.lock().unwrap();
        loop {
            if let Some(i) = entries.iter().position(|(name, _, _)| name == op) { let (_, code, value) = entries.remove(i); return Some((code, value)); }
            let remaining = end.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() { return None; }
            let (next, wait) = self.ready.wait_timeout(entries, remaining).unwrap();
            entries = next;
            if wait.timed_out() && !entries.iter().any(|(name, _, _)| name == op) { return None; }
        }
    }
    pub(crate) fn observe_view(&self, value: i32) { self.view.store(value, Ordering::SeqCst); }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn wait_result_returns_matching_operation_or_times_out() {
        let shared = ProofShared::default();
        shared.push_result("other", 9, 1);
        shared.push_result("wanted", 1, 42);
        assert_eq!(shared.wait_result("wanted", Duration::from_millis(10)), Some((1,42)));
        assert_eq!(shared.wait_result("missing", Duration::from_millis(1)), None);
    }
}
