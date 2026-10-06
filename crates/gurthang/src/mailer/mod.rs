mod email_sender;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use email_sender::EmailSender;

use crate::{app::Context, error::Result};

pub const DEFAULT_FROM_SENDER: &str = "System <system@example.com>";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Email {
    pub from: Option<String>,
    pub to: String,
    pub reply_to: Option<String>,
    pub subject: String,
    pub text: String,
    pub html: String,
    pub bcc: Option<String>,
    pub cc: Option<String>,
}

#[derive(Debug)]
pub struct MailerOpts {
    pub from: String,
    pub reply_to: Option<String>,
}

impl Default for MailerOpts {
    fn default() -> Self {
        Self {
            from: DEFAULT_FROM_SENDER.to_owned(),
            reply_to: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum MailerEnvelope {
    Mailer(Email),
}

pub struct MailerWorker;

impl MailerWorker {
    pub async fn perform_later(ctx: &Context, email: Email) -> Result<()> {
        ctx.jobs.enqueue(MailerEnvelope::Mailer(email)).await?;
        Ok(())
    }
}

#[async_trait]
pub trait Mailer {
    fn opts() -> MailerOpts {
        MailerOpts::default()
    }

    async fn mail(ctx: &Context, email: &Email) -> Result<()> {
        let opts = Self::opts();
        let mut email = email.clone();
        email.from = Some(email.from.unwrap_or_else(|| opts.from.clone()));
        email.reply_to = email.reply_to.or(opts.reply_to);
        MailerWorker::perform_later(ctx, email).await
    }

    async fn deliver_now(_ctx: &Context, email: &Email) -> Result<()> {
        let opts = Self::opts();
        let mut email = email.clone();
        email.from = Some(email.from.unwrap_or_else(|| opts.from.clone()));
        email.reply_to = email.reply_to.or(opts.reply_to);
        deliver(&email).await
    }
}

pub async fn deliver(email: &Email) -> Result<()> {
    match EmailSender::global() {
        Some(sender) => sender.mail(email).await,
        None => Err(crate::error::Error::Message(
            "attempting to send email but no email sender configured".into(),
        )),
    }
}

pub async fn perform_job(email: Email, _database: &sqlx::PgPool) -> gurthang_jobs::Result<()> {
    deliver(&email)
        .await
        .map_err(|error| gurthang_jobs::Error::Config(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMailer;
    impl Mailer for TestMailer {}

    #[test]
    fn mailer_envelope_serializes_as_tagged_job() {
        let json = serde_json::to_value(MailerEnvelope::Mailer(Email {
            to: "user@example.com".into(),
            subject: "Welcome".into(),
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(json["type"], "mailer");
        assert_eq!(json["payload"]["to"], "user@example.com");
        assert_eq!(json["payload"]["subject"], "Welcome");
    }

    #[tokio::test]
    async fn stub_sender_accepts_mail() {
        let sender = EmailSender::stub();
        sender
            .mail(&Email {
                to: "user@example.com".into(),
                subject: "Welcome".into(),
                text: "Welcome".into(),
                html: "<p>Welcome</p>".into(),
                ..Default::default()
            })
            .await
            .unwrap();
    }
}
