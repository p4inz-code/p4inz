use std::collections::HashMap;

use crate::handler::JobHandler;

/// Maps [`crate::JobKind`] names to the [`JobHandler`] that executes them.
///
/// Mirrors `p4inz_discord::CommandRegistry`'s shape (name → boxed
/// trait-object handler) for the same reason: dispatch across a set of
/// heterogeneous implementations known only at composition-root wiring
/// time.
#[derive(Default)]
pub struct JobHandlerRegistry {
    handlers: HashMap<String, Box<dyn JobHandler + Send + Sync>>,
}

impl JobHandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `handler` for `kind`. Returns the previously registered
    /// handler for the same kind, if any — callers should treat that as a
    /// programming error (duplicate job kind registration).
    pub fn insert(
        &mut self,
        kind: impl Into<String>,
        handler: impl JobHandler + Send + Sync + 'static,
    ) -> Option<Box<dyn JobHandler + Send + Sync>> {
        self.handlers.insert(kind.into(), Box::new(handler))
    }

    pub fn get(&self, kind: &str) -> Option<&(dyn JobHandler + Send + Sync)> {
        self.handlers.get(kind).map(|h| h.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use p4inz_errors::AppResult;

    use super::*;

    struct Noop;

    #[async_trait]
    impl JobHandler for Noop {
        async fn handle(&self, _payload: &str) -> AppResult<()> {
            Ok(())
        }
    }

    #[test]
    fn starts_empty() {
        let registry = JobHandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn insert_and_get_round_trip_by_kind() {
        let mut registry = JobHandlerRegistry::new();
        registry.insert("test:noop", Noop);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test:noop").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn inserting_same_kind_replaces_and_returns_previous() {
        let mut registry = JobHandlerRegistry::new();
        assert!(registry.insert("test:noop", Noop).is_none());
        assert!(registry.insert("test:noop", Noop).is_some());
        assert_eq!(registry.len(), 1);
    }
}
