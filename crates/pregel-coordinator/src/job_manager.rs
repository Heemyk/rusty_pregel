//! Job lifecycle management.

use pregel_common::WorkerId;
use std::collections::HashMap;

/// Metadata about an active job.
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: String,
    pub workers: Vec<WorkerId>,
    pub superstep: u64,
}

/// Registry of active jobs.
pub struct JobManager {
    jobs: HashMap<String, JobInfo>,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    pub fn register(&mut self, job_id: String, workers: Vec<WorkerId>) {
        self.jobs.insert(
            job_id.clone(),
            JobInfo {
                id: job_id,
                workers,
                superstep: 0,
            },
        );
    }

    pub fn get(&self, job_id: &str) -> Option<&JobInfo> {
        self.jobs.get(job_id)
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}
