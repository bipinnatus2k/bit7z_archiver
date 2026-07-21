//! Scheduler trait and default implementation.

use std::collections::HashSet;

use crate::job::{Job, JobId};
use crate::resource::ResourceCapacity;

/// Decides which ready jobs should run next.
pub trait Scheduler: Send + Sync {
    fn select_next(
        &self,
        ready: &[Job],
        running: &[Job],
        capacity: ResourceCapacity,
    ) -> Vec<JobId>;
}

/// A simple scheduler that respects priority, concurrency, and session affinity.
#[derive(Debug, Default)]
pub struct DefaultScheduler {
    max_concurrent: usize,
}

impl DefaultScheduler {
    pub fn new(max_concurrent: usize) -> Self {
        Self { max_concurrent }
    }
}

impl Scheduler for DefaultScheduler {
    fn select_next(
        &self,
        ready: &[Job],
        running: &[Job],
        _capacity: ResourceCapacity,
    ) -> Vec<JobId> {
        if running.len() >= self.max_concurrent {
            return vec![];
        }

        let running_sessions: HashSet<_> = running.iter().filter_map(|j| j.session_id).collect();
        let mut candidates: Vec<&Job> = ready
            .iter()
            .filter(|j| {
                j.session_id
                    .map(|sid| !running_sessions.contains(&sid))
                    .unwrap_or(true)
            })
            .collect();

        candidates.sort_by(|a, b| a.priority.cmp(&b.priority));

        let slots = self.max_concurrent.saturating_sub(running.len());
        candidates.into_iter().take(slots).map(|j| j.id).collect()
    }
}
