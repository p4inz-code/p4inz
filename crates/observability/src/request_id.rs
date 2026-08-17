//! Correlation/request IDs (`docs/development/implementation_plan.md`
//! section 16: "Correlation/request IDs").

/// Generates a fresh correlation id suitable for tagging one request, job,
/// or interaction across every log line and span it produces.
pub fn generate() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_distinct_ids() {
        assert_ne!(generate(), generate());
    }

    #[test]
    fn generates_a_well_formed_uuid() {
        let id = generate();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
    }
}
