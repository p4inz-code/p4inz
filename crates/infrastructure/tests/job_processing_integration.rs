//! Cross-layer integration test (`docs/development/implementation_plan.md`
//! Milestone 59: Integration Tests — "Cross-layer behavior") proving
//! `p4inz_jobs`' worker orchestration (`process_next`) correctly drives a
//! real `p4inz_infrastructure::jobs::GitHubSyncHandler` end-to-end:
//! enqueue via `trigger_github_sync` -> `process_next` claims and executes
//! it -> the handler decodes the job payload, synchronizes through
//! `p4inz_knowledge::synchronize_from_source`, and persists via a
//! `KnowledgeRepository` -> the job is marked succeeded (or failed, for
//! the failure-path test).
//!
//! Everything above is real production code from three different crates
//! (`p4inz-jobs`, `p4inz-infrastructure`, `p4inz-knowledge`) wired
//! together as a Cargo integration test (a separate binary compiled
//! against each crate's public API, not an in-crate `#[cfg(test)]`
//! module) — proving the seam between the job system's generic
//! orchestration and one concrete `JobHandler` actually connects, which
//! no existing unit test does: `crates/jobs/src/execute.rs`'s tests drive
//! `process_next` against trivial stand-in handlers, and
//! `crates/infrastructure/src/jobs/github_sync.rs`'s tests call
//! `GitHubSyncHandler::handle` directly, bypassing `process_next`
//! entirely.
//!
//! Only the two genuine I/O boundaries are faked: the GitHub source
//! adapter (no live GitHub API call) and job/knowledge persistence (no
//! live PostgreSQL — that level of integration is Milestone 60: Database
//! Tests).

use std::sync::Mutex;
use std::time::SystemTime;

use p4inz_errors::{AppError, AppResult};
use p4inz_infrastructure::jobs::{GITHUB_SYNC_JOB_KIND, GitHubSyncHandler, trigger_github_sync};
use p4inz_jobs::{Job, JobHandlerRegistry, JobId, JobRepository, JobStatus, process_next};
use p4inz_knowledge::{
    KnowledgeCategory, KnowledgeItem, KnowledgeItemId, KnowledgeRepository, RawDocument,
    SourceAdapter,
};

#[derive(Default)]
struct InMemoryJobRepository {
    jobs: Mutex<Vec<Job>>,
}

impl JobRepository for InMemoryJobRepository {
    async fn enqueue(&self, job: &Job) -> AppResult<()> {
        self.jobs.lock().unwrap().push(job.clone());
        Ok(())
    }

    async fn claim_next(&self, now: SystemTime) -> AppResult<Option<Job>> {
        let mut jobs = self.jobs.lock().unwrap();
        let due = jobs.iter_mut().find(|j| j.status() == JobStatus::Pending && j.run_at() <= now);
        match due {
            Some(job) => {
                *job = Job::from_parts(
                    job.id(),
                    job.kind().clone(),
                    job.payload().to_string(),
                    JobStatus::Running,
                    job.run_at(),
                    job.attempts(),
                    job.max_attempts(),
                    job.last_error().map(str::to_string),
                    job.created_at(),
                    now,
                );
                Ok(Some(job.clone()))
            }
            None => Ok(None),
        }
    }

    async fn mark_succeeded(&self, id: JobId, now: SystemTime) -> AppResult<()> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
        *job = Job::from_parts(
            job.id(),
            job.kind().clone(),
            job.payload().to_string(),
            JobStatus::Succeeded,
            job.run_at(),
            job.attempts(),
            job.max_attempts(),
            job.last_error().map(str::to_string),
            job.created_at(),
            now,
        );
        Ok(())
    }

    async fn mark_failed(&self, id: JobId, error: &str, now: SystemTime) -> AppResult<()> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
        *job = Job::from_parts(
            job.id(),
            job.kind().clone(),
            job.payload().to_string(),
            JobStatus::Failed,
            job.run_at(),
            job.attempts() + 1,
            job.max_attempts(),
            Some(error.to_string()),
            job.created_at(),
            now,
        );
        Ok(())
    }

    async fn reschedule(
        &self,
        id: JobId,
        run_at: SystemTime,
        error: &str,
        now: SystemTime,
    ) -> AppResult<()> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.iter_mut().find(|j| j.id() == id).unwrap();
        *job = Job::from_parts(
            job.id(),
            job.kind().clone(),
            job.payload().to_string(),
            JobStatus::Pending,
            run_at,
            job.attempts() + 1,
            job.max_attempts(),
            Some(error.to_string()),
            job.created_at(),
            now,
        );
        Ok(())
    }

    async fn find_by_id(&self, id: JobId) -> AppResult<Option<Job>> {
        Ok(self.jobs.lock().unwrap().iter().find(|j| j.id() == id).cloned())
    }

    async fn count_by_status(&self, status: JobStatus) -> AppResult<u64> {
        Ok(self.jobs.lock().unwrap().iter().filter(|j| j.status() == status).count() as u64)
    }
}

struct FixedAdapter(RawDocument);

impl SourceAdapter for FixedAdapter {
    async fn fetch(&self, _reference: &str) -> AppResult<RawDocument> {
        Ok(self.0.clone())
    }
}

struct FailingAdapter;

impl SourceAdapter for FailingAdapter {
    async fn fetch(&self, _reference: &str) -> AppResult<RawDocument> {
        Err(AppError::unavailable("GitHub unreachable"))
    }
}

#[derive(Default, Clone)]
struct InMemoryKnowledgeRepository {
    items: std::sync::Arc<Mutex<Vec<KnowledgeItem>>>,
}

impl KnowledgeRepository for InMemoryKnowledgeRepository {
    async fn save(&self, item: &KnowledgeItem) -> AppResult<()> {
        let mut items = self.items.lock().unwrap();
        items.retain(|existing| existing.id() != item.id());
        items.push(item.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: KnowledgeItemId) -> AppResult<Option<KnowledgeItem>> {
        Ok(self.items.lock().unwrap().iter().find(|i| i.id() == id).cloned())
    }

    async fn find_by_source_reference(&self, reference: &str) -> AppResult<Option<KnowledgeItem>> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.source().reference().map(|link| link.as_str()) == Some(reference))
            .cloned())
    }
}

#[tokio::test]
async fn a_triggered_sync_job_is_claimed_processed_and_persisted() {
    let job_repository = InMemoryJobRepository::default();
    let knowledge_repository = InMemoryKnowledgeRepository::default();
    let items_handle = knowledge_repository.items.clone();

    let mut registry = JobHandlerRegistry::new();
    let adapter = FixedAdapter(RawDocument {
        title: "p4inz-code/p4inz".to_string(),
        body: "A Discord bot for Northbyte Studios.".to_string(),
        fetched_at: SystemTime::now(),
    });
    registry.insert(GITHUB_SYNC_JOB_KIND, GitHubSyncHandler::new(adapter, knowledge_repository));

    let now = SystemTime::now();
    trigger_github_sync(&job_repository, "p4inz-code/p4inz", KnowledgeCategory::Projects, now)
        .await
        .unwrap();
    assert_eq!(job_repository.count_by_status(JobStatus::Pending).await.unwrap(), 1);

    let processed = process_next(&job_repository, &registry, now).await.unwrap();

    assert!(processed, "process_next should have claimed the enqueued sync job");
    assert_eq!(job_repository.count_by_status(JobStatus::Succeeded).await.unwrap(), 1);
    assert_eq!(job_repository.count_by_status(JobStatus::Pending).await.unwrap(), 0);

    let items = items_handle.lock().unwrap().clone();
    assert_eq!(items.len(), 1, "the handler should have persisted the synchronized item");
    assert_eq!(items[0].title().as_str(), "p4inz-code/p4inz");
}

#[tokio::test]
async fn a_failing_source_adapter_leaves_the_job_pending_for_retry() {
    let job_repository = InMemoryJobRepository::default();

    let mut registry = JobHandlerRegistry::new();
    registry.insert(
        GITHUB_SYNC_JOB_KIND,
        GitHubSyncHandler::new(FailingAdapter, InMemoryKnowledgeRepository::default()),
    );

    let now = SystemTime::now();
    trigger_github_sync(&job_repository, "p4inz-code/p4inz", KnowledgeCategory::Projects, now)
        .await
        .unwrap();

    let processed = process_next(&job_repository, &registry, now).await.unwrap();

    assert!(processed);
    // The default retry budget (Milestone 34: Retry System) is more than
    // one attempt, so a single failure reschedules rather than fails
    // permanently — proving `process_next`'s retry decision correctly
    // reacts to a real handler's real error, not a stand-in.
    assert_eq!(job_repository.count_by_status(JobStatus::Pending).await.unwrap(), 1);
    assert_eq!(job_repository.count_by_status(JobStatus::Failed).await.unwrap(), 0);

    let job = job_repository.jobs.lock().unwrap()[0].clone();
    assert_eq!(job.attempts(), 1);
    assert!(job.last_error().unwrap().contains("GitHub unreachable"));
}
