/// Compares two byte slices in constant time with respect to their
/// contents, to avoid timing side-channels when checking a secret against
/// an attacker-controlled value (e.g. a webhook HMAC signature or a bearer
/// token) — `docs/security/security-model.md` requires webhook
/// verification and secret isolation as mandatory controls, both of which
/// depend on this being done safely.
///
/// A length mismatch returns `false` immediately without scanning either
/// slice; this leaks the fact that lengths differ (not their contents),
/// which is the standard, accepted behavior for this kind of comparison —
/// callers comparing against a fixed-length secret (e.g. a 32-byte HMAC)
/// are not exposed to anything they didn't already know.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_slices_are_equal() {
        assert!(constant_time_eq(b"secret-token", b"secret-token"));
    }

    #[test]
    fn different_content_is_not_equal() {
        assert!(!constant_time_eq(b"secret-token", b"secret-tokeN"));
    }

    #[test]
    fn different_length_is_not_equal() {
        assert!(!constant_time_eq(b"short", b"a-much-longer-value"));
    }

    #[test]
    fn empty_slices_are_equal() {
        assert!(constant_time_eq(b"", b""));
    }
}
