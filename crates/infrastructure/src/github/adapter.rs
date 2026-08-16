use std::time::SystemTime;

use p4inz_common::Secret;
use p4inz_errors::{AppError, ErrorKind, IntoAppError};
use p4inz_knowledge::{RawDocument, SourceAdapter};
use reqwest::{Client, StatusCode};

use crate::github::response::{ReadmeResponse, RepositoryResponse, decode_readme};

const GITHUB_API_BASE: &str = "https://api.github.com";

/// Fetches a GitHub repository's description and README as a
/// [`RawDocument`] (`docs/PROJECT_SPEC.md` section 5: "GitHub may be used
/// for frequently changing project information").
///
/// `reference` is an `"owner/repo"` string. Uses GitHub's REST API
/// directly via `reqwest` rather than an SDK crate — the surface needed
/// (two read-only GET requests) doesn't justify an additional dependency.
pub struct GitHubSourceAdapter {
    http: Client,
    token: Option<Secret>,
}

impl GitHubSourceAdapter {
    pub fn new(token: Option<Secret>) -> Result<Self, AppError> {
        let http = Client::builder()
            .user_agent(concat!("p4inz/", env!("CARGO_PKG_VERSION")))
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build GitHub HTTP client")?;
        Ok(Self { http, token })
    }

    fn authorized(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(token) => builder.bearer_auth(token.expose_secret()),
            None => builder,
        }
    }
}

impl SourceAdapter for GitHubSourceAdapter {
    async fn fetch(&self, reference: &str) -> p4inz_errors::AppResult<RawDocument> {
        let repo_response = self
            .authorized(self.http.get(format!("{GITHUB_API_BASE}/repos/{reference}")))
            .send()
            .await
            .into_app_error(ErrorKind::Unavailable, "failed to reach the GitHub API")?;

        if repo_response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::not_found(format!(
                "GitHub repository '{reference}' was not found"
            )));
        }
        let repo_response = repo_response
            .error_for_status()
            .into_app_error(ErrorKind::Unavailable, "the GitHub API returned an error")?;
        let repository: RepositoryResponse = repo_response.json().await.into_app_error(
            ErrorKind::Internal,
            "failed to parse the GitHub repository response",
        )?;

        let readme_body = self.fetch_readme(reference).await;

        Ok(build_raw_document(&repository, readme_body, SystemTime::now()))
    }
}

impl GitHubSourceAdapter {
    /// Best-effort README fetch: a repository legitimately may not have
    /// one, in which case `fetch` falls back to the repository's
    /// description rather than failing outright.
    async fn fetch_readme(&self, reference: &str) -> Option<String> {
        let response = self
            .authorized(self.http.get(format!("{GITHUB_API_BASE}/repos/{reference}/readme")))
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let readme: ReadmeResponse = response.json().await.ok()?;
        decode_readme(&readme).ok()
    }
}

/// Combines a repository's metadata and (optional) decoded README into a
/// [`RawDocument`]: the README when present and non-blank, otherwise the
/// repository's description, otherwise an empty body (left for the
/// synchronization step to reject — `p4inz-infrastructure` doesn't decide
/// what counts as insufficient content).
fn build_raw_document(
    repository: &RepositoryResponse,
    readme_body: Option<String>,
    fetched_at: SystemTime,
) -> RawDocument {
    let body = readme_body
        .filter(|body| !body.trim().is_empty())
        .or_else(|| repository.description.clone())
        .unwrap_or_default();

    RawDocument { title: repository.full_name.clone(), body, fetched_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(full_name: &str, description: Option<&str>) -> RepositoryResponse {
        RepositoryResponse {
            full_name: full_name.to_string(),
            description: description.map(str::to_string),
        }
    }

    #[test]
    fn prefers_readme_body_when_present() {
        let doc = build_raw_document(
            &repo("p4inz-code/p4inz", Some("desc")),
            Some("# Readme".to_string()),
            SystemTime::now(),
        );
        assert_eq!(doc.title, "p4inz-code/p4inz");
        assert_eq!(doc.body, "# Readme");
    }

    #[test]
    fn falls_back_to_description_when_readme_missing() {
        let doc =
            build_raw_document(&repo("p4inz-code/p4inz", Some("desc")), None, SystemTime::now());
        assert_eq!(doc.body, "desc");
    }

    #[test]
    fn falls_back_to_description_when_readme_blank() {
        let doc = build_raw_document(
            &repo("p4inz-code/p4inz", Some("desc")),
            Some("   ".to_string()),
            SystemTime::now(),
        );
        assert_eq!(doc.body, "desc");
    }

    #[test]
    fn empty_when_neither_readme_nor_description_available() {
        let doc = build_raw_document(&repo("p4inz-code/p4inz", None), None, SystemTime::now());
        assert_eq!(doc.body, "");
    }

    /// Makes a real request to api.github.com. Not run by default — CI and
    /// this environment should not depend on a third-party service being
    /// reachable during `cargo test`. Run explicitly with
    /// `cargo test -p p4inz-infrastructure -- --ignored`.
    #[tokio::test]
    #[ignore = "makes a real network request to api.github.com; see doc comment"]
    async fn fetches_the_real_p4inz_repository() {
        let adapter = GitHubSourceAdapter::new(None).unwrap();
        let document = adapter.fetch("p4inz-code/p4inz").await.unwrap();
        assert_eq!(document.title, "p4inz-code/p4inz");
        assert!(!document.body.is_empty());
    }

    #[tokio::test]
    #[ignore = "makes a real network request to api.github.com; see doc comment"]
    async fn not_found_repository_returns_not_found() {
        use p4inz_errors::ErrorKind;

        let adapter = GitHubSourceAdapter::new(None).unwrap();
        let error = adapter.fetch("p4inz-code/this-repo-does-not-exist-xyz").await.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotFound);
    }
}
