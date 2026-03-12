//! Barrier synchronization for BSP.

use pregel_common::WorkerId;
use std::collections::HashSet;

/// Tracks which workers have reported completion for the current superstep.
///
/// In BSP, no worker advances until all have finished. Each worker sends
/// a "barrier ack" when done. The coordinator checks `all_reported()`.
/// When true, it broadcasts "advance" and calls `reset()` for the next superstep.
#[derive(Debug, Default)]
pub struct Barrier {
    expected_workers: HashSet<WorkerId>,
    reported_workers: HashSet<WorkerId>,
}

impl Barrier {
    pub fn new(expected_workers: impl IntoIterator<Item = WorkerId>) -> Self {
        Self {
            expected_workers: expected_workers.into_iter().collect(),
            reported_workers: HashSet::new(),
        }
    }

    pub fn report(&mut self, worker_id: WorkerId) {
        self.reported_workers.insert(worker_id);
    }

    pub fn all_reported(&self) -> bool {
        self.expected_workers.is_subset(&self.reported_workers)
    }

    pub fn reset(&mut self) {
        self.reported_workers.clear();
    }
}
