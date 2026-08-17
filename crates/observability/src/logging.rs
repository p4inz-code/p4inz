//! Structured logging (`docs/development/implementation_plan.md` section
//! 16: "Structured logs") — every binary calls [`init`] once, at startup,
//! before doing anything else worth logging.

use tracing_subscriber::EnvFilter;

/// Installs a global JSON-formatted `tracing` subscriber, controlled by the
/// standard `RUST_LOG` environment variable (`.env.example`; defaulting to
/// `info` when unset or invalid).
///
/// Safe to call more than once in the same process — only the first call
/// actually installs a subscriber; later calls are a silent no-op rather
/// than the panic `tracing_subscriber::fmt().init()` would produce, which
/// matters for tests that construct a binary's startup path repeatedly.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt().json().with_env_filter(filter).try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_safe_to_call_more_than_once() {
        init();
        init();
    }
}
