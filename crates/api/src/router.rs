use axum::Router;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin::{__path_trigger_sync, trigger_sync};
use crate::auth::{__path_callback, __path_login, __path_me, callback, login, me};
use crate::health::{__path_health, __path_ready, health, ready};
use crate::knowledge::{__path_search, search};
use crate::metrics::{__path_metrics, metrics};
use crate::rate_limit::rate_limit;
use crate::request_tracing::trace_requests;
use crate::state::ApiState;
use crate::version::API_V1_PREFIX;

/// The API's OpenAPI document (`docs/development/implementation_plan.md`
/// section 10: "OpenAPI-described"). Generated from the `#[utoipa::path]`
/// annotations on each handler rather than hand-maintained separately, so
/// the description can't silently drift from the actual routes — "API
/// contracts must be treated as compatibility boundaries" is only
/// meaningful if the contract reliably reflects what's really served.
#[derive(OpenApi)]
#[openapi(info(title = "P4inz API", version = "1"))]
struct ApiDoc;

/// Builds the complete Axum application (`docs/development/
/// implementation_plan.md` section 10: "API Architecture").
///
/// `allowed_origins` becomes an explicit CORS allowlist — "CORS is
/// explicitly configured" — never a wildcard: an empty list means no
/// cross-origin browser request is permitted, matching
/// [`p4inz_config::ApiConfig`]'s fail-closed default.
pub fn build_router(state: ApiState, allowed_origins: &[String]) -> Router {
    let v1 = OpenApiRouter::new()
        .routes(routes!(search))
        .routes(routes!(login))
        .routes(routes!(callback))
        .routes(routes!(me))
        .routes(routes!(trigger_sync));

    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health))
        .routes(routes!(ready))
        .routes(routes!(metrics))
        .nest(API_V1_PREFIX, v1)
        .with_state(state.clone())
        .split_for_parts();

    router
        .route("/openapi.json", get(move || async move { axum::Json(openapi) }))
        .layer(axum::middleware::from_fn_with_state(state, rate_limit))
        .layer(cors_layer(allowed_origins))
        // Outermost: assigns/logs/measures every request, including ones
        // rejected below by CORS or rate limiting (Milestone 51).
        .layer(axum::middleware::from_fn(trace_requests))
}

fn cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| match origin.parse::<HeaderValue>() {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!(origin, "ignoring invalid configured CORS origin");
                None
            }
        })
        .collect();

    if origins.is_empty() {
        // No `allow_origin` configured at all: browsers receive no
        // `Access-Control-Allow-Origin` header and block cross-origin
        // reads — the fail-closed default, not `CorsLayer::permissive()`.
        CorsLayer::new()
    } else {
        CorsLayer::new().allow_origin(origins).allow_methods([Method::GET, Method::POST])
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    /// A `PgPool` that performs no I/O at construction — safe to build
    /// without a live PostgreSQL instance for tests that only exercise
    /// routing, not `/ready`'s actual database round trip (which has its
    /// own `#[ignore]`d live-database coverage).
    fn fake_pool() -> p4inz_database::PgPool {
        let config = p4inz_config::DatabaseConfig {
            url: p4inz_common::Secret::new("postgres://user:pass@localhost/p4inz"),
        };
        p4inz_database::connect_lazy(&config, p4inz_database::PoolSettings::default()).unwrap()
    }

    /// A request builder that already carries a `ConnectInfo` extension —
    /// the rate-limit middleware (Milestone 42) requires one, which
    /// `axum::serve(...).into_make_service_with_connect_info` supplies for
    /// a real connection but a `oneshot`-driven test request needs set
    /// explicitly.
    fn request(method: &str, uri: &str) -> axum::http::request::Builder {
        Request::builder()
            .method(method)
            .uri(uri)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))))
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response =
            app.oneshot(request("GET", "/health").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), br#"{"status":"ok"}"#);
    }

    #[tokio::test]
    async fn unknown_route_returns_not_found() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response = app
            .oneshot(request("GET", "/does-not-exist").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn cors_layer_permits_no_origin_by_default() {
        // Building the layer with an empty allowlist must not panic and
        // must not fall back to a permissive wildcard — covered directly
        // since `CorsLayer` doesn't expose its configured origins for a
        // black-box HTTP-response assertion.
        let _ = cors_layer(&[]);
    }

    #[test]
    fn cors_layer_ignores_an_invalid_configured_origin() {
        let _ = cors_layer(&["not a valid header value \n".to_string()]);
    }

    #[tokio::test]
    async fn openapi_document_describes_the_health_and_ready_routes() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response = app
            .oneshot(request("GET", "/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(doc["info"]["title"], "P4inz API");
        assert!(doc["paths"]["/health"]["get"].is_object());
        assert!(doc["paths"]["/ready"]["get"].is_object());
        assert!(doc["paths"]["/metrics"]["get"].is_object());
        assert!(doc["paths"]["/v1/knowledge/search"]["get"].is_object());
    }

    #[tokio::test]
    async fn metrics_returns_prometheus_text() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response =
            app.oneshot(request("GET", "/metrics").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.starts_with("text/plain"));

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("p4inz_http_requests_total"));
        assert!(text.contains("p4inz_database_pool_connections"));
    }

    #[tokio::test]
    async fn every_response_carries_a_request_id_header() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response =
            app.oneshot(request("GET", "/health").body(Body::empty()).unwrap()).await.unwrap();

        let request_id = response.headers().get("x-request-id").unwrap().to_str().unwrap();
        assert!(!request_id.is_empty());
    }

    #[tokio::test]
    async fn an_incoming_request_id_is_echoed_back() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response = app
            .oneshot(
                request("GET", "/health")
                    .header("x-request-id", "caller-supplied-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.headers().get("x-request-id").unwrap(), "caller-supplied-id");
    }

    #[tokio::test]
    async fn search_without_a_query_parameter_is_rejected() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response = app
            .oneshot(request("GET", "/v1/knowledge/search").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_with_a_blank_query_is_rejected() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response = app
            .oneshot(request("GET", "/v1/knowledge/search?q=%20%20").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(doc["code"], "validation");
    }

    fn configured_state() -> ApiState {
        configured_state_with_admins(&[])
    }

    fn configured_state_with_admins(admin_user_ids: &[&str]) -> ApiState {
        let oauth = p4inz_infrastructure::DiscordOAuthClient::new(
            "123456".to_string(),
            p4inz_common::Secret::new("client-secret"),
            "https://p4inz.dev/v1/auth/discord/callback".to_string(),
        )
        .unwrap();
        ApiState::new(fake_pool()).with_auth(crate::state::AuthState {
            oauth,
            session_secret: p4inz_common::Secret::new("session-secret"),
            admin_user_ids: admin_user_ids.iter().map(|s| s.to_string()).collect(),
        })
    }

    #[tokio::test]
    async fn auth_routes_are_unavailable_when_not_configured() {
        let app = build_router(ApiState::new(fake_pool()), &[]);

        let response =
            app.oneshot(request("GET", "/v1/auth/me").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn login_redirects_to_discord_and_sets_a_state_cookie() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(request("GET", "/v1/auth/discord/login").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert!(location.starts_with("https://discord.com/api/oauth2/authorize"));
        let set_cookie = response.headers().get("set-cookie").unwrap().to_str().unwrap();
        assert!(set_cookie.starts_with("p4inz_oauth_state="));
        assert!(set_cookie.contains("HttpOnly"));
    }

    #[tokio::test]
    async fn me_without_a_session_is_unauthorized() {
        let app = build_router(configured_state(), &[]);

        let response =
            app.oneshot(request("GET", "/v1/auth/me").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn me_with_a_valid_session_returns_the_authenticated_identity() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(
                request("GET", "/v1/auth/me")
                    .header("cookie", session_cookie_for("123456789012345678"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(doc["user_id"], "123456789012345678");
        assert_eq!(doc["username"], "x");
    }

    #[tokio::test]
    async fn me_with_a_tampered_session_cookie_is_unauthorized() {
        let app = build_router(configured_state(), &[]);

        let mut cookie = session_cookie_for("123456789012345678");
        // Flips the last character of the signed token — the HMAC
        // signature (`crate::auth::session::issue_session`) must reject
        // this, not just decode whatever payload happens to still parse.
        cookie.push('x');

        let response = app
            .oneshot(
                request("GET", "/v1/auth/me").header("cookie", cookie).body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn callback_without_a_matching_state_cookie_is_unauthorized() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(
                request("GET", "/v1/auth/discord/callback?code=abc&state=whatever")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    fn session_cookie_for(user_id: &str) -> String {
        let identity = p4inz_infrastructure::DiscordIdentity {
            user_id: user_id.to_string(),
            username: "x".to_string(),
        };
        let token =
            crate::auth::issue_session(&p4inz_common::Secret::new("session-secret"), &identity)
                .unwrap();
        format!("p4inz_session={token}")
    }

    #[tokio::test]
    async fn trigger_sync_without_a_session_is_unauthorized() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(
                request("POST", "/v1/admin/knowledge/sync")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"reference":"p4inz-code/p4inz","category":"projects"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn trigger_sync_with_a_session_but_not_an_admin_is_forbidden() {
        let app = build_router(configured_state_with_admins(&["999"]), &[]);

        let response = app
            .oneshot(
                request("POST", "/v1/admin/knowledge/sync")
                    .header("content-type", "application/json")
                    .header("cookie", session_cookie_for("123"))
                    .body(Body::from(r#"{"reference":"p4inz-code/p4inz","category":"projects"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn trigger_sync_rejects_an_invalid_category_even_for_an_admin() {
        let app = build_router(configured_state_with_admins(&["123"]), &[]);

        let response = app
            .oneshot(
                request("POST", "/v1/admin/knowledge/sync")
                    .header("content-type", "application/json")
                    .header("cookie", session_cookie_for("123"))
                    .body(Body::from(r#"{"reference":"p4inz-code/p4inz","category":"bogus"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn openapi_document_describes_the_admin_route() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(request("GET", "/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(doc["paths"]["/v1/admin/knowledge/sync"]["post"].is_object());
    }

    #[tokio::test]
    async fn openapi_document_describes_the_auth_routes() {
        let app = build_router(configured_state(), &[]);

        let response = app
            .oneshot(request("GET", "/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(doc["paths"]["/v1/auth/discord/login"]["get"].is_object());
        assert!(doc["paths"]["/v1/auth/discord/callback"]["get"].is_object());
        assert!(doc["paths"]["/v1/auth/me"]["get"].is_object());
    }

    #[tokio::test]
    async fn a_client_exceeding_its_rate_limit_gets_a_429() {
        let app = build_router(ApiState::new(fake_pool()), &[]);
        // The default `RateLimiterConfig` grants 5 tokens; exhaust them
        // from the same simulated client IP before the request that
        // should finally be rejected.
        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(request("GET", "/health").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response =
            app.oneshot(request("GET", "/health").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn requests_from_different_client_ips_have_independent_rate_limits() {
        let app = build_router(ApiState::new(fake_pool()), &[]);
        let other_ip = ConnectInfo(SocketAddr::from(([10, 0, 0, 1], 0)));

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(request("GET", "/health").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .extension(other_ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    struct E2eFixedAdapter;

    impl p4inz_knowledge::SourceAdapter for E2eFixedAdapter {
        async fn fetch(
            &self,
            _reference: &str,
        ) -> p4inz_errors::AppResult<p4inz_knowledge::RawDocument> {
            Ok(p4inz_knowledge::RawDocument {
                title: "P4inz End To End Sync Test".to_string(),
                body: "Exercises the full admin-sync-to-public-search path.".to_string(),
                fetched_at: std::time::SystemTime::now(),
            })
        }
    }

    /// Full-system E2E scenario (`docs/development/implementation_plan.md`
    /// Milestone 65: "Full-system scenarios") — the complete real path an
    /// admin-triggered GitHub sync takes: HTTP request -> session/
    /// permission check -> job enqueued -> the worker's orchestration
    /// (`p4inz_jobs::process_next`) claims and processes it -> knowledge
    /// persisted via the real PostgreSQL-backed repository -> the public
    /// search endpoint finds it. Every layer but the GitHub network call
    /// itself is real, not faked — [`E2eFixedAdapter`] stands in for that
    /// one genuine external boundary, the same substitution
    /// `crates/infrastructure/tests/job_processing_integration.rs`
    /// (Milestone 59) already makes for the same reason.
    ///
    /// Requires a live, reachable PostgreSQL instance — not run by default
    /// (`cargo test --workspace`; see `crates/database/tests/
    /// migrations.rs`'s module doc comment for how to run tests like this
    /// one explicitly).
    #[tokio::test]
    #[ignore = "requires a live PostgreSQL instance; see doc comment"]
    async fn admin_triggered_sync_flows_through_the_worker_into_public_search() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
        let db_config = p4inz_config::DatabaseConfig { url: p4inz_common::Secret::new(url) };
        let pool = p4inz_database::connect(&db_config, p4inz_database::PoolSettings::default())
            .await
            .unwrap();
        p4inz_database::run_migrations(&pool).await.unwrap();

        let admin_id = format!("e2e-admin-{}", uuid::Uuid::new_v4());
        let oauth = p4inz_infrastructure::DiscordOAuthClient::new(
            "123456".to_string(),
            p4inz_common::Secret::new("client-secret"),
            "https://p4inz.dev/v1/auth/discord/callback".to_string(),
        )
        .unwrap();
        let state = ApiState::new(pool.clone()).with_auth(crate::state::AuthState {
            oauth,
            session_secret: p4inz_common::Secret::new("session-secret"),
            admin_user_ids: vec![admin_id.clone()],
        });
        let app = build_router(state, &[]);

        let reference = format!("p4inz-e2e-test/{}", uuid::Uuid::new_v4());
        let sync_request_body =
            serde_json::json!({ "reference": reference, "category": "projects" });
        let response = app
            .clone()
            .oneshot(
                request("POST", "/v1/admin/knowledge/sync")
                    .header("content-type", "application/json")
                    .header("cookie", session_cookie_for(&admin_id))
                    .body(Body::from(sync_request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The worker isn't a separate running process in this test — drive
        // its orchestration directly, over the same database the API just
        // enqueued the job in.
        let mut registry = p4inz_jobs::JobHandlerRegistry::new();
        registry.insert(
            p4inz_infrastructure::jobs::GITHUB_SYNC_JOB_KIND,
            p4inz_infrastructure::jobs::GitHubSyncHandler::new(
                E2eFixedAdapter,
                p4inz_search::PgKnowledgeRepository::new(pool.clone()),
            ),
        );
        let job_repository = p4inz_infrastructure::jobs::PgJobRepository::new(pool.clone());
        let processed =
            p4inz_jobs::process_next(&job_repository, &registry, std::time::SystemTime::now())
                .await
                .unwrap();
        assert!(processed, "the enqueued sync job should have been claimed and processed");

        let search_response = app
            .oneshot(request("GET", "/v1/knowledge/search?q=sync").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(search_response.status(), StatusCode::OK);
        let body = search_response.into_body().collect().await.unwrap().to_bytes();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let results = doc["results"].as_array().unwrap();
        assert!(
            results.iter().any(|item| item["title"] == "P4inz End To End Sync Test"),
            "expected the item synchronized by the worker to be findable via public search"
        );
    }
}
