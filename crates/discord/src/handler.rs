use p4inz_application::{Question, QuestionError, QuestionHandler};
use p4inz_audit::AuditActor;
use p4inz_errors::AppError;
use p4inz_security::RateLimiter;
use serenity::all::{
    Context, CreateInteractionResponse, CreateInteractionResponseMessage, Interaction, Message,
    Ready,
};
use serenity::async_trait;
use serenity::client::EventHandler;
use tracing::Instrument;

use crate::error_ux::describe;
use crate::permissions::GuildRoleMapping;
use crate::registry::CommandRegistry;

/// Gateway lifecycle, slash-command dispatch, and natural-language message
/// handler.
///
/// Connection-lifecycle events (`ready`, `resume`), slash-command dispatch
/// (`interaction_create`) and natural-language questions (`message`) are
/// handled here, with failures rendered consistently via
/// [`crate::error_ux::describe`]. Component/autocomplete/modal
/// interactions are out of scope until a command needs them.
pub struct Handler<Q: QuestionHandler> {
    registry: CommandRegistry,
    question_handler: Q,
    question_rate_limiter: RateLimiter,
    guild_role_mapping: GuildRoleMapping,
}

impl<Q: QuestionHandler> Handler<Q> {
    pub fn new(
        registry: CommandRegistry,
        question_handler: Q,
        question_rate_limiter: RateLimiter,
        guild_role_mapping: GuildRoleMapping,
    ) -> Self {
        Self { registry, question_handler, question_rate_limiter, guild_role_mapping }
    }
}

/// Strips a single leading `<@id>`/`<@!id>` user-mention token (and the
/// whitespace after it) from `content`, if present.
///
/// Mentions are stripped structurally rather than by looking up the bot's
/// own user id, so this needs no cache lookup — Discord always renders a
/// leading mention as `<@` followed by digits and a closing `>`.
fn strip_leading_mention(content: &str) -> &str {
    let trimmed = content.trim_start();
    let Some(rest) = trimmed.strip_prefix("<@") else {
        return trimmed;
    };
    let rest = rest.strip_prefix('!').unwrap_or(rest);
    let Some(end) = rest.find('>') else {
        return trimmed;
    };
    let (id, after) = rest.split_at(end);
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return trimmed;
    }
    after[1..].trim_start()
}

#[async_trait]
impl<Q: QuestionHandler + Send + Sync + 'static> EventHandler for Handler<Q> {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!(user = %ready.user.name, "Discord gateway connected");
    }

    async fn resume(&self, _ctx: Context, _: serenity::model::event::ResumedEvent) {
        tracing::info!("Discord gateway session resumed");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command_interaction) = interaction else {
            return;
        };

        let name = command_interaction.data.name.clone();
        // Discord interaction tracing (`docs/development/
        // implementation_plan.md` section 16): correlates every log line
        // produced while handling this interaction under Discord's own
        // snowflake id, the same pattern job execution
        // (`p4inz_jobs::execute::process_next`) and API requests
        // (`p4inz_api::request_tracing::trace_requests`) already use.
        let span = tracing::info_span!(
            "discord_interaction",
            interaction_id = %command_interaction.id,
            command = %name,
        );

        async move {
            let Some(command) = self.registry.get(&name) else {
                tracing::warn!("received unknown slash command");
                return;
            };

            if let Err(error) = command.execute(&ctx, &command_interaction).await {
                tracing::error!(%error, "slash command handler failed");

                let response = CreateInteractionResponseMessage::new()
                    .content(describe(&error))
                    .ephemeral(true);
                // Best-effort: if the command already sent its own response
                // before failing, this call itself fails and we've already
                // logged above, so there's nothing further to recover.
                let _ = command_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Message(response))
                    .await;
            }
        }
        .instrument(span)
        .await
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let span = tracing::info_span!(
            "discord_message",
            message_id = %msg.id,
            author = %msg.author.id,
        );

        async move {
            let is_direct_message = msg.guild_id.is_none();
            let directed_at_bot =
                is_direct_message || msg.mentions_me(&ctx.http).await.unwrap_or(false);
            if !directed_at_bot {
                return;
            }

            let text = if is_direct_message {
                msg.content.as_str()
            } else {
                strip_leading_mention(&msg.content)
            };

            let question = match Question::parse(text) {
                Ok(question) => question,
                Err(QuestionError::Empty) => return,
                Err(QuestionError::TooLong { max }) => {
                    let error =
                        AppError::validation(format!("question must be at most {max} characters"));
                    let _ = msg.reply(&ctx.http, describe(&error)).await;
                    return;
                }
            };

            let rate_limit_key = format!("question:{}", msg.author.id);
            if let Err(error) = self.question_rate_limiter.check(&rate_limit_key) {
                let _ = msg.reply(&ctx.http, describe(&error)).await;
                return;
            }

            // No guild role assignments have a way to be persisted/configured
            // yet (Discord Permissions, Milestone 14, is the mapping
            // mechanism only), so this resolves to an empty `PermissionSet`
            // for every guild member until an administrator-configured
            // mapping is injected here — fail closed, per the security
            // model, rather than granting access by default.
            let member_role_ids =
                msg.member.as_ref().map(|member| member.roles.as_slice()).unwrap_or(&[]);
            let granted = self.guild_role_mapping.resolve(member_role_ids);
            let actor = AuditActor::User(format!("discord:{}", msg.author.id));

            match self.question_handler.answer(&question, &granted, actor).await {
                Ok(answer) => {
                    let _ = msg.reply(&ctx.http, answer).await;
                }
                Err(error) => {
                    tracing::error!(%error, "question handler failed");
                    let _ = msg.reply(&ctx.http, describe(&error)).await;
                }
            }
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_plain_mention() {
        assert_eq!(strip_leading_mention("<@123456789012345678> what is p4inz?"), "what is p4inz?");
    }

    #[test]
    fn strips_nickname_mention() {
        assert_eq!(
            strip_leading_mention("<@!123456789012345678> what is p4inz?"),
            "what is p4inz?"
        );
    }

    #[test]
    fn leaves_content_without_a_mention_unchanged() {
        assert_eq!(strip_leading_mention("what is p4inz?"), "what is p4inz?");
    }

    #[test]
    fn leaves_malformed_mention_unchanged() {
        assert_eq!(strip_leading_mention("<@notanid> hello"), "<@notanid> hello");
    }

    #[test]
    fn handles_mention_with_no_trailing_text() {
        assert_eq!(strip_leading_mention("<@123456789012345678>"), "");
    }
}
