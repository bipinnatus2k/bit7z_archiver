//! Scheduler trait and default implementation.

use std::collections::HashSet;

use crate::job::{Job, JobId};
use crate::resource::ResourceCapacity;

/// Decides which ready jobs should run next.
pub trait Scheduler: Send + Sync {
    fn select_next(&self, ready: &[Job], running: &[Job], capacity: ResourceCapacity)
    -> Vec<JobId>;
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

        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

        let slots = self.max_concurrent.saturating_sub(running.len());
        candidates.into_iter().take(slots).map(|j| j.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{Job, JobId, JobKind, Priority};
    use crate::resource::ResourceCapacity;
    use std::path::PathBuf;

    fn make_job(id: u64, priority: Priority, session_id: Option<u64>) -> Job {
        Job {
            id: JobId(id),
            kind: JobKind::OpenArchive {
                path: PathBuf::from("test.7z"),
                password: None,
            },
            priority,
            session_id,
            descriptor: bit7z_capability::ExecutionDescriptor {
                backend: bit7z_capability::BackendId(1),
                capabilities: vec![],
                resource_claim: bit7z_capability::ResourceClaim::default(),
                policy: bit7z_capability::ExecutionPolicy::Queued,
            },
        }
    }

    #[test]
    fn test_scheduler_respects_concurrency_limit() {
        let scheduler = DefaultScheduler::new(2);
        let ready = vec![
            make_job(1, Priority::User, None),
            make_job(2, Priority::User, None),
            make_job(3, Priority::User, None),
        ];
        let selected = scheduler.select_next(&ready, &[], ResourceCapacity::default());
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_scheduler_respects_session_affinity() {
        let scheduler = DefaultScheduler::new(10);
        let ready = vec![
            make_job(1, Priority::User, Some(1)),
            make_job(2, Priority::User, Some(1)),
            make_job(3, Priority::User, Some(2)),
        ];
        let running = vec![make_job(4, Priority::User, Some(1))];
        let selected = scheduler.select_next(&ready, &running, ResourceCapacity::default());
        // Only the job for session 2 should be selected, because session 1 is already running.
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], JobId(3));
    }

    #[test]
    fn test_scheduler_prioritizes_user_jobs() {
        let scheduler = DefaultScheduler::new(1);
        let ready = vec![
            make_job(1, Priority::Background, None),
            make_job(2, Priority::User, None),
        ];
        let selected = scheduler.select_next(&ready, &[], ResourceCapacity::default());
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], JobId(2));
    }
}
