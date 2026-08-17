use std::future::Future;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use p4inz_domain::Link;
use p4inz_errors::{AppResult, ErrorKind, IntoAppError};
use p4inz_jobs::{Job, JobHandler, JobKind, JobRepository};
use p4inz_knowledge::{
    KnowledgeCategory, KnowledgeRepository, Source, SourceAdapter, SourceKind,
    synchronize_from_source,
};
use serde::{Deserialize, Serialize};

/// The job kind registered for GitHub synchronization
/// (`docs/development/implementation_plan.md` Milestone 35: GitHub Jobs).
pub const GITHUB_SYNC_JOB_KIND: &str = "knowledge:github_sync";

/// A GitHub sync job's payload: which repository, and which
/// [`KnowledgeCategory`] to file the resulting knowledge item under.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubSyncPayload {
    reference: String,
    category: String,
}

/// Enqueues a one-off GitHub synchronization job for `reference` (an
/// `"owner/repo"` string) to run as soon as a worker is free — the
/// "manual synchronization trigger" requirement
/// (`docs/development/implementation_plan.md` section 15).
///
/// [`run_scheduler`] is the automatic counterpart; both enqueue the exact
/// same job kind and payload shape, so [`GitHubSyncHandler`] handles a
/// manually triggered sync identically to a scheduled one — there is no
/// separate "manual" code path to keep in sync. No caller (a Discord
/// admin command, an API endpoint) wires this up yet — neither exists at
/// this point in the roadmap (Discord Commands' concrete commands and the
/// API are later phases) — but the capability itself is what this
/// milestone requires; exposing it is whichever later surface needs it.
pub async fn trigger_github_sync(
    job_repository: &impl JobRepository,
    reference: &str,
    category: KnowledgeCategory,
    now: SystemTime,
) -> AppResult<()> {
    enqueue(job_repository, reference, category, now, now).await
}

async fn enqueue(
    job_repository: &impl JobRepository,
    reference: &str,
    category: KnowledgeCategory,
    run_at: SystemTime,
    now: SystemTime,
) -> AppResult<()> {
    let payload = GitHubSyncPayload {
        reference: reference.to_string(),
        category: category.as_str().to_string(),
    };
    let payload = serde_json::to_string(&payload)
        .into_app_error(ErrorKind::Internal, "failed to encode GitHub sync job payload")?;
    let kind =
        JobKind::parse(GITHUB_SYNC_JOB_KIND).expect("GITHUB_SYNC_JOB_KIND is a valid job kind");
    let job = Job::new(kind, payload, run_at, now);
    job_repository.enqueue(&job).await
}

/// Executes a GitHub sync job (`docs/development/implementation_plan.md`
/// Milestone 35), decoding the job's [`GitHubSyncPayload`] and delegating
/// to `p4inz_knowledge::synchronize_from_source` — the same, already
/// idempotent/incremental synchronization operation Knowledge
/// Synchronization (Milestone 20) established; this only adds the
/// job-system plumbing around it.
pub struct GitHubSyncHandler<A: SourceAdapter, R: KnowledgeRepository> {
    adapter: A,
    knowledge_repository: R,
}

impl<A: SourceAdapter, R: KnowledgeRepository> GitHubSyncHandler<A, R> {
    pub fn new(adapter: A, knowledge_repository: R) -> Self {
        Self { adapter, knowledge_repository }
    }
}

#[async_trait]
impl<A: SourceAdapter + Sync, R: KnowledgeRepository + Sync> JobHandler
    for GitHubSyncHandler<A, R>
{
    async fn handle(&self, payload: &str) -> AppResult<()> {
        let parsed: GitHubSyncPayload = serde_json::from_str(payload)
            .into_app_error(ErrorKind::Validation, "invalid GitHub sync job payload")?;
        let category = KnowledgeCategory::parse(&parsed.category)
            .into_app_error(ErrorKind::Validation, "invalid knowledge category in job payload")?;
        let link = Link::parse(format!("https://github.com/{}", parsed.reference))
            .into_app_error(ErrorKind::Validation, "invalid GitHub reference in job payload")?;
        let source = Source::new(SourceKind::Repository, Some(link));

        synchronize_from_source(
            &self.adapter,
            &self.knowledge_repository,
            &parsed.reference,
            category,
            source,
            SystemTime::now(),
        )
        .await?;

        Ok(())
    }
}

/// Runs the periodic GitHub-synchronization scheduler until `shutdown`
/// resolves — the "Scheduled GitHub synchronization" requirement
/// (`docs/development/implementation_plan.md` section 15).
///
/// On startup and then every `tick_interval`, enqueues a sync job (via the
/// same [`enqueue`] path [`trigger_github_sync`] uses) for every entry in
/// `repositories`. Deliberately does not try to skip repositories that
/// haven't changed since their last sync — `synchronize_from_source`
/// itself already only writes when content actually changed
/// ([`p4inz_knowledge::SyncOutcome::Unchanged`] is a no-op write), so an
/// "unnecessary" scheduled sync costs one cheap GitHub API read, not a
/// database write; adding staleness tracking on top would be speculative
/// complexity for the repository counts this system targets ("a handful
/// of repositories", not hundreds).
///
/// Like [`p4inz_jobs::run_worker_with`], `shutdown` is only ever raced
/// against the idle wait between ticks, never against an in-flight
/// enqueue — the same cancellation-safety reasoning applies.
pub async fn run_scheduler<J, F>(
    job_repository: &J,
    repositories: &[String],
    category: KnowledgeCategory,
    tick_interval: Duration,
    shutdown: F,
) -> std::io::Result<()>
where
    J: JobRepository + Sync,
    F: Future<Output = std::io::Result<()>>,
{
    tracing::info!(repository_count = repositories.len(), "GitHub sync scheduler started");
    tokio::pin!(shutdown);

    loop {
        for reference in repositories {
            let now = SystemTime::now();
            if let Err(error) = enqueue(job_repository, reference, category, now, now).await {
                tracing::error!(reference, %error, "failed to enqueue scheduled GitHub sync");
            }
        }

        tokio::select! {
            biased;
            result = &mut shutdown => {
                result?;
                break;
            }
            () = tokio::time::sleep(tick_interval) => {}
        }
    }

    tracing::info!("GitHub sync scheduler shutting down");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use p4inz_errors::AppError;
    use p4inz_knowledge::{Body, KnowledgeItem, KnowledgeItemId, RawDocument, Title};

    use super::*;

    #[derive(Default)]
    struct InMemoryJobRepository {
        jobs: Mutex<Vec<Job>>,
    }

    impl JobRepository for InMemoryJobRepository {
        async fn enqueue(&self, job: &Job) -> AppResult<()> {
            self.jobs.lock().unwrap().push(job.clone());
            Ok(())
        }

        async fn claim_next(&self, _now: SystemTime) -> AppResult<Option<Job>> {
            unimplemented!("not exercised by these tests")
        }

        async fn mark_succeeded(&self, _id: p4inz_jobs::JobId, _now: SystemTime) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn mark_failed(
            &self,
            _id: p4inz_jobs::JobId,
            _error: &str,
            _now: SystemTime,
        ) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn reschedule(
            &self,
            _id: p4inz_jobs::JobId,
            _run_at: SystemTime,
            _error: &str,
            _now: SystemTime,
        ) -> AppResult<()> {
            unimplemented!("not exercised by these tests")
        }

        async fn find_by_id(&self, _id: p4inz_jobs::JobId) -> AppResult<Option<Job>> {
            unimplemented!("not exercised by these tests")
        }

        async fn count_by_status(&self, _status: p4inz_jobs::JobStatus) -> AppResult<u64> {
            unimplemented!("not exercised by these tests")
        }
    }

    #[tokio::test]
    async fn trigger_enqueues_a_job_of_the_registered_kind() {
        let repository = InMemoryJobRepository::default();
        let now = SystemTime::now();

        trigger_github_sync(&repository, "p4inz-code/p4inz", KnowledgeCategory::Projects, now)
            .await
            .unwrap();

        let jobs = repository.jobs.lock().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind().as_str(), GITHUB_SYNC_JOB_KIND);
        assert!(jobs[0].payload().contains("p4inz-code/p4inz"));
        assert!(jobs[0].payload().contains("projects"));
    }

    #[tokio::test]
    async fn scheduler_enqueues_one_job_per_configured_repository_on_startup() {
        let repository = InMemoryJobRepository::default();
        let repositories = vec!["p4inz-code/p4inz".to_string(), "p4inz-code/website".to_string()];

        run_scheduler(
            &repository,
            &repositories,
            KnowledgeCategory::Projects,
            Duration::from_secs(3600),
            async { Ok(()) },
        )
        .await
        .unwrap();

        assert_eq!(repository.jobs.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn scheduler_enqueues_nothing_for_an_empty_repository_list() {
        let repository = InMemoryJobRepository::default();

        run_scheduler(
            &repository,
            &[],
            KnowledgeCategory::Projects,
            Duration::from_secs(3600),
            async { Ok(()) },
        )
        .await
        .unwrap();

        assert!(repository.jobs.lock().unwrap().is_empty());
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

        async fn find_by_source_reference(
            &self,
            reference: &str,
        ) -> AppResult<Option<KnowledgeItem>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|i| i.source().reference().map(|link| link.as_str()) == Some(reference))
                .cloned())
        }
    }

    fn valid_payload(reference: &str) -> String {
        serde_json::to_string(&GitHubSyncPayload {
            reference: reference.to_string(),
            category: KnowledgeCategory::Projects.as_str().to_string(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn handle_synchronizes_and_persists_a_new_item() {
        let adapter = FixedAdapter(RawDocument {
            title: "p4inz-code/p4inz".to_string(),
            body: "A Discord bot.".to_string(),
            fetched_at: SystemTime::now(),
        });
        let knowledge_repository = InMemoryKnowledgeRepository::default();
        let items_handle = knowledge_repository.items.clone();
        let handler = GitHubSyncHandler::new(adapter, knowledge_repository);

        handler.handle(&valid_payload("p4inz-code/p4inz")).await.unwrap();

        let items = items_handle.lock().unwrap().clone();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title(), &Title::parse("p4inz-code/p4inz").unwrap());
        assert_eq!(items[0].body(), &Body::parse("A Discord bot.").unwrap());
    }

    #[tokio::test]
    async fn handle_propagates_adapter_failure() {
        let handler =
            GitHubSyncHandler::new(FailingAdapter, InMemoryKnowledgeRepository::default());

        let err = handler.handle(&valid_payload("p4inz-code/p4inz")).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn handle_rejects_malformed_payload() {
        let handler = GitHubSyncHandler::new(
            FixedAdapter(RawDocument {
                title: "x".to_string(),
                body: "y".to_string(),
                fetched_at: SystemTime::now(),
            }),
            InMemoryKnowledgeRepository::default(),
        );

        let err = handler.handle("not json").await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }

    #[tokio::test]
    async fn handle_rejects_an_unknown_category() {
        let payload = serde_json::to_string(&GitHubSyncPayload {
            reference: "p4inz-code/p4inz".to_string(),
            category: "not-a-real-category".to_string(),
        })
        .unwrap();
        let handler = GitHubSyncHandler::new(
            FixedAdapter(RawDocument {
                title: "x".to_string(),
                body: "y".to_string(),
                fetched_at: SystemTime::now(),
            }),
            InMemoryKnowledgeRepository::default(),
        );

        let err = handler.handle(&payload).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }
}
