use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use p4inz_common::Secret;
use p4inz_errors::{AppError, AppResult, ErrorKind, IntoAppError};
use p4inz_infrastructure::DiscordIdentity;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// How long an issued session remains valid.
const SESSION_LIFETIME_SECS: u64 = 24 * 60 * 60;

/// The claims carried by a session token — a signed, stateless "secure
/// session" (`docs/security/security-model.md`: "Secure sessions") over
/// the identity a Discord OAuth2 exchange produced. Stateless (no
/// server-side session store) matches the zero-cost self-hosting goal
/// (ADR-002): no database table or external store (Redis, etc.) is
/// needed just to track logged-in sessions.
///
/// The token itself is `base64url(payload_json).base64url(hmac_sha256)` —
/// a minimal hand-rolled scheme rather than a general JWT library:
/// exactly one algorithm (HMAC-SHA256) is ever needed here, so pulling in
/// a full JWT crate (which bundles RSA/ECDSA/EdDSA support this system
/// has no use for) would be exactly the "unnecessary dependency"
/// `docs/development/implementation_plan.md` section 19 warns against —
/// `hmac`/`sha2` (RustCrypto, the same ecosystem a JWT crate would use
/// internally) are minimal, mature, and already do all the real work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionClaims {
    /// The authenticated Discord user id.
    pub sub: String,
    pub username: String,
    /// Expiry, as seconds since the Unix epoch.
    pub exp: u64,
}

fn hmac_for(session_secret: &Secret) -> AppResult<HmacSha256> {
    HmacSha256::new_from_slice(session_secret.expose_secret().as_bytes())
        .into_app_error(ErrorKind::Internal, "failed to initialize session signing key")
}

/// Issues a signed session token for `identity`, valid for
/// [`SESSION_LIFETIME_SECS`].
pub fn issue_session(session_secret: &Secret, identity: &DiscordIdentity) -> AppResult<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .into_app_error(ErrorKind::Internal, "system clock is before the Unix epoch")?
        .as_secs();

    let claims = SessionClaims {
        sub: identity.user_id.clone(),
        username: identity.username.clone(),
        exp: now + SESSION_LIFETIME_SECS,
    };
    let payload = serde_json::to_vec(&claims)
        .into_app_error(ErrorKind::Internal, "failed to encode session claims")?;

    let mut mac = hmac_for(session_secret)?;
    mac.update(&payload);
    let signature = mac.finalize().into_bytes();

    Ok(format!("{}.{}", URL_SAFE_NO_PAD.encode(&payload), URL_SAFE_NO_PAD.encode(signature)))
}

/// Verifies a session token, returning its claims if it is validly
/// signed and not expired.
pub fn verify_session(session_secret: &Secret, token: &str) -> AppResult<SessionClaims> {
    let invalid = || AppError::new(ErrorKind::Unauthorized, "session is invalid or has expired");

    let (payload_b64, signature_b64) = token.split_once('.').ok_or_else(invalid)?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).map_err(|_| invalid())?;
    let signature = URL_SAFE_NO_PAD.decode(signature_b64).map_err(|_| invalid())?;

    let mut mac = hmac_for(session_secret)?;
    mac.update(&payload);
    // `verify_slice` compares in constant time internally.
    mac.verify_slice(&signature).map_err(|_| invalid())?;

    let claims: SessionClaims = serde_json::from_slice(&payload).map_err(|_| invalid())?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .into_app_error(ErrorKind::Internal, "system clock is before the Unix epoch")?
        .as_secs();
    if claims.exp <= now {
        return Err(invalid());
    }

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> DiscordIdentity {
        DiscordIdentity { user_id: "123".to_string(), username: "atharva".to_string() }
    }

    #[test]
    fn issued_session_verifies_successfully() {
        let secret = Secret::new("session-secret");
        let token = issue_session(&secret, &identity()).unwrap();

        let claims = verify_session(&secret, &token).unwrap();

        assert_eq!(claims.sub, "123");
        assert_eq!(claims.username, "atharva");
    }

    #[test]
    fn verifying_with_the_wrong_secret_fails() {
        let token = issue_session(&Secret::new("secret-a"), &identity()).unwrap();

        let err = verify_session(&Secret::new("secret-b"), &token).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Unauthorized);
    }

    #[test]
    fn verifying_garbage_fails() {
        let err = verify_session(&Secret::new("secret"), "not-a-token").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unauthorized);
    }

    #[test]
    fn a_tampered_payload_fails_verification() {
        let secret = Secret::new("session-secret");
        let token = issue_session(&secret, &identity()).unwrap();
        let (_, signature) = token.split_once('.').unwrap();

        let forged_claims = SessionClaims {
            sub: "attacker".to_string(),
            username: "attacker".to_string(),
            exp: u64::MAX,
        };
        let forged_payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged_claims).unwrap());
        let forged_token = format!("{forged_payload}.{signature}");

        let err = verify_session(&secret, &forged_token).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unauthorized);
    }

    #[test]
    fn an_already_expired_token_fails_verification() {
        let secret = Secret::new("session-secret");
        let expired = SessionClaims { sub: "123".to_string(), username: "x".to_string(), exp: 0 };
        let payload = serde_json::to_vec(&expired).unwrap();
        let mut mac = hmac_for(&secret).unwrap();
        mac.update(&payload);
        let signature = mac.finalize().into_bytes();
        let token =
            format!("{}.{}", URL_SAFE_NO_PAD.encode(&payload), URL_SAFE_NO_PAD.encode(signature));

        let err = verify_session(&secret, &token).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Unauthorized);
    }
}
