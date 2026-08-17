//! In-process metrics (`docs/development/implementation_plan.md` section
//! 16: "Metrics", "AI latency/error metrics", "Database health metrics") —
//! a minimal counter registry rendered as Prometheus's plain-text
//! exposition format, with no third-party metrics dependency. This matches
//! the codebase's existing preference for hand-rolling a small surface
//! over pulling in a larger dependency graph for a narrow need (see the
//! session-token module's equivalent decision against a JWT library).
//!
//! Deliberately process-local, not persisted or shared across replicas —
//! adequate for the single-process deployment this project targets
//! ("Observability must work using free/local infrastructure": a `/metrics`
//! endpoint any self-hosted Prometheus can scrape needs no external
//! metrics backend).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Process-wide metrics registry. Every field is an independent atomic
/// counter, so recording never blocks and never contends across metric
/// kinds.
///
/// Recorded through ambiently, like `tracing`'s own macros, rather than
/// dependency-injected: metric recording has no business behavior for a
/// test to substitute or assert exact values against (unlike, say,
/// `p4inz_security::RateLimiter`, whose exact decisions are the thing
/// under test) — see [`global`](Metrics::global).
#[derive(Default)]
pub struct Metrics {
    http_requests_total: AtomicU64,
    http_request_errors_total: AtomicU64,
    http_request_duration_ms_sum: AtomicU64,
    http_request_duration_ms_count: AtomicU64,
    ai_requests_total: AtomicU64,
    ai_request_errors_total: AtomicU64,
    ai_request_duration_ms_sum: AtomicU64,
    ai_request_duration_ms_count: AtomicU64,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

impl Metrics {
    /// The single process-wide instance every caller records into and
    /// renders from.
    pub fn global() -> &'static Metrics {
        METRICS.get_or_init(Metrics::default)
    }

    /// Records one completed HTTP request ("API tracing").
    pub fn record_http_request(&self, status: u16, duration: Duration) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
        if status >= 500 {
            self.http_request_errors_total.fetch_add(1, Ordering::Relaxed);
        }
        self.http_request_duration_ms_sum.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.http_request_duration_ms_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one completed AI provider call ("AI latency/error metrics").
    /// `success` is `false` for both a provider error and an invalid
    /// response that fell back to a deterministic answer — both are the AI
    /// path failing to produce a usable completion.
    pub fn record_ai_request(&self, success: bool, duration: Duration) {
        self.ai_requests_total.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.ai_request_errors_total.fetch_add(1, Ordering::Relaxed);
        }
        self.ai_request_duration_ms_sum.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.ai_request_duration_ms_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Renders every counter, plus caller-supplied point-in-time gauges
    /// (e.g. database pool size), as Prometheus's plain-text exposition
    /// format.
    ///
    /// Gauges are accepted as a parameter rather than tracked as fields on
    /// [`Metrics`] itself so this crate never needs to depend on
    /// `p4inz-database`/`sqlx` to read them — callers sample their own
    /// live state and pass the numbers in as `(name, help, value)`.
    pub fn render_prometheus_text(&self, gauges: &[(&str, &str, f64)]) -> String {
        let mut out = String::new();

        push_counter(
            &mut out,
            "p4inz_http_requests_total",
            "Total HTTP requests handled.",
            self.http_requests_total.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_http_request_errors_total",
            "HTTP requests that completed with a 5xx status.",
            self.http_request_errors_total.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_http_request_duration_ms_sum",
            "Sum of HTTP request durations, in milliseconds.",
            self.http_request_duration_ms_sum.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_http_request_duration_ms_count",
            "Count of HTTP requests contributing to the duration sum.",
            self.http_request_duration_ms_count.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_ai_requests_total",
            "Total AI provider calls attempted.",
            self.ai_requests_total.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_ai_request_errors_total",
            "AI provider calls that errored or fell back to a deterministic answer.",
            self.ai_request_errors_total.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_ai_request_duration_ms_sum",
            "Sum of AI provider call durations, in milliseconds.",
            self.ai_request_duration_ms_sum.load(Ordering::Relaxed),
        );
        push_counter(
            &mut out,
            "p4inz_ai_request_duration_ms_count",
            "Count of AI provider calls contributing to the duration sum.",
            self.ai_request_duration_ms_count.load(Ordering::Relaxed),
        );

        for (name, help, value) in gauges {
            out.push_str(&format!("# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}\n"));
        }

        out
    }
}

fn push_counter(out: &mut String, name: &str, help: &str, value: u64) {
    out.push_str(&format!("# HELP {name} {help}\n# TYPE {name} counter\n{name} {value}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_renders_http_requests() {
        let metrics = Metrics::default();
        metrics.record_http_request(200, Duration::from_millis(50));
        metrics.record_http_request(500, Duration::from_millis(150));

        let text = metrics.render_prometheus_text(&[]);

        assert!(text.contains("p4inz_http_requests_total 2"));
        assert!(text.contains("p4inz_http_request_errors_total 1"));
        assert!(text.contains("p4inz_http_request_duration_ms_sum 200"));
        assert!(text.contains("p4inz_http_request_duration_ms_count 2"));
    }

    #[test]
    fn a_client_error_does_not_count_as_an_http_error() {
        let metrics = Metrics::default();
        metrics.record_http_request(404, Duration::from_millis(10));

        let text = metrics.render_prometheus_text(&[]);

        assert!(text.contains("p4inz_http_request_errors_total 0"));
    }

    #[test]
    fn records_and_renders_ai_requests() {
        let metrics = Metrics::default();
        metrics.record_ai_request(true, Duration::from_millis(300));
        metrics.record_ai_request(false, Duration::from_millis(700));

        let text = metrics.render_prometheus_text(&[]);

        assert!(text.contains("p4inz_ai_requests_total 2"));
        assert!(text.contains("p4inz_ai_request_errors_total 1"));
        assert!(text.contains("p4inz_ai_request_duration_ms_sum 1000"));
    }

    #[test]
    fn renders_supplied_gauges() {
        let metrics = Metrics::default();

        let text = metrics.render_prometheus_text(&[(
            "p4inz_database_pool_connections",
            "Current pool size.",
            5.0,
        )]);

        assert!(text.contains("# TYPE p4inz_database_pool_connections gauge"));
        assert!(text.contains("p4inz_database_pool_connections 5"));
    }

    #[test]
    fn global_returns_the_same_instance_across_calls() {
        Metrics::global().record_http_request(200, Duration::from_millis(1));
        let before = Metrics::global().render_prometheus_text(&[]);
        Metrics::global().record_http_request(200, Duration::from_millis(1));
        let after = Metrics::global().render_prometheus_text(&[]);

        assert_ne!(before, after);
    }
}
