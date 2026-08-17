use p4inz_common::Secret;
use p4inz_errors::{AppError, AppResult, ErrorKind, IntoAppError};
use reqwest::Client;
use serde::Deserialize;

const AUTHORIZE_URL: &str = "https://discord.com/api/oauth2/authorize";
const TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const IDENTIFY_URL: &str = "https://discord.com/api/users/@me";

/// A Discord user's identity, as returned by Discord's `/users/@me`
/// endpoint after a successful OAuth2 code exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordIdentity {
    pub user_id: String,
    pub username: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserResponse {
    id: String,
    username: String,
}

/// Web/admin authentication via "Sign in with Discord"
/// (`docs/development/implementation_plan.md` Milestone 40: Authentication;
/// section 12: "Web Authentication -> Identity -> P4inz Permission").
///
/// Discord OAuth2 was chosen over a separate username/password account
/// system: Discord identity is already this product's core identity model
/// (guild membership, role mapping — Discord Permissions, Milestone 14),
/// so web/admin authentication reuses it rather than standing up a
/// parallel account system with its own password storage/reset flow,
/// which `docs/architecture/zero-cost.md`'s self-hosting goal and
/// `docs/development/implementation_plan.md` section 1 ("avoid
/// unnecessary dependencies"/"avoid speculative infrastructure") both
/// argue against absent a concrete requirement for one.
///
/// This only performs the OAuth2 exchange and identity lookup — issuing
/// and verifying a session from the resulting [`DiscordIdentity`] is
/// `p4inz-api`'s concern (an HTTP-adapter detail, not an external
/// integration).
#[derive(Clone)]
pub struct DiscordOAuthClient {
    http: Client,
    client_id: String,
    client_secret: Secret,
    redirect_uri: String,
}

impl DiscordOAuthClient {
    pub fn new(client_id: String, client_secret: Secret, redirect_uri: String) -> AppResult<Self> {
        let http = Client::builder()
            .user_agent(concat!("p4inz/", env!("CARGO_PKG_VERSION")))
            .build()
            .into_app_error(ErrorKind::Internal, "failed to build Discord OAuth HTTP client")?;
        Ok(Self { http, client_id, client_secret, redirect_uri })
    }

    /// The URL to redirect a browser to in order to begin the OAuth2
    /// flow. `state` is an opaque, caller-generated CSRF token that must
    /// be verified unchanged when the callback is later handled.
    pub fn authorize_url(&self, state: &str) -> String {
        let redirect = urlencoding_light(&self.redirect_uri);
        let state = urlencoding_light(state);
        format!(
            "{AUTHORIZE_URL}?client_id={}&redirect_uri={redirect}&response_type=code&scope=identify&state={state}",
            self.client_id
        )
    }

    /// Exchanges an authorization `code` for the authenticated user's
    /// [`DiscordIdentity`].
    pub async fn exchange_code(&self, code: &str) -> AppResult<DiscordIdentity> {
        let token_response = self
            .http
            .post(TOKEN_URL)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.expose_secret()),
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await
            .into_app_error(
                ErrorKind::Unavailable,
                "failed to reach Discord's OAuth token endpoint",
            )?
            .error_for_status()
            .map_err(|_| AppError::unauthorized("Discord rejected the authorization code"))?;

        let token: TokenResponse = token_response
            .json()
            .await
            .into_app_error(ErrorKind::Internal, "failed to parse Discord's token response")?;

        let user_response = self
            .http
            .get(IDENTIFY_URL)
            .bearer_auth(&token.access_token)
            .send()
            .await
            .into_app_error(ErrorKind::Unavailable, "failed to reach Discord's user API")?
            .error_for_status()
            .into_app_error(ErrorKind::Unavailable, "Discord's user API returned an error")?;

        let user: UserResponse = user_response
            .json()
            .await
            .into_app_error(ErrorKind::Internal, "failed to parse Discord's user response")?;

        Ok(DiscordIdentity { user_id: user.id, username: user.username })
    }
}

/// Percent-encodes the handful of characters that matter for a URL query
/// value (space and the URL's own structural delimiters). Not a general
/// RFC 3986 encoder — `p4inz-infrastructure` has no reason to depend on a
/// full URL-encoding crate for the two values (a `redirect_uri` this
/// deployment itself configures, and a locally generated UUID `state`)
/// that ever pass through this.
fn urlencoding_light(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            '?' => "%3F".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            other => other.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> DiscordOAuthClient {
        DiscordOAuthClient::new(
            "123456".to_string(),
            Secret::new("client-secret"),
            "https://p4inz.dev/v1/auth/discord/callback".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn authorize_url_includes_client_id_state_and_encoded_redirect() {
        let url = client().authorize_url("csrf-token");

        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains("client_id=123456"));
        assert!(url.contains("state=csrf-token"));
        assert!(
            url.contains("redirect_uri=https%3A%2F%2Fp4inz.dev%2Fv1%2Fauth%2Fdiscord%2Fcallback")
        );
        assert!(url.contains("scope=identify"));
    }

    #[test]
    fn urlencoding_light_escapes_structural_characters() {
        assert_eq!(urlencoding_light("a b"), "a%20b");
        assert_eq!(
            urlencoding_light("https://x.com/a?b=c&d"),
            "https%3A%2F%2Fx.com%2Fa%3Fb%3Dc%26d"
        );
    }

    /// Makes a real request to discord.com. Not run by default — this
    /// environment should not depend on a third-party service being
    /// reachable during `cargo test`. Run explicitly with `cargo test -p
    /// p4inz-infrastructure -- --ignored`, and only with a genuinely
    /// valid one-time authorization code (codes are single-use).
    #[tokio::test]
    #[ignore = "requires a real, valid, single-use Discord OAuth code; see doc comment"]
    async fn exchange_code_with_an_invalid_code_is_unauthorized() {
        let err = client().exchange_code("not-a-real-code").await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unauthorized);
    }
}
