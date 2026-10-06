use std::sync::OnceLock;

use lettre::{
    AsyncTransport, Message, Tokio1Executor, Transport, message::MultiPart,
    transport::smtp::authentication::Credentials,
};
use tracing::info;

use super::{DEFAULT_FROM_SENDER, Email};
use crate::{
    config::{MailerConfig, MailerTls, SmtpMailer},
    error::{Error, Result},
};

static SENDER: OnceLock<EmailSender> = OnceLock::new();

#[derive(Clone, Debug)]
enum EmailTransport {
    Smtp(lettre::AsyncSmtpTransport<Tokio1Executor>),
    Stub(lettre::transport::stub::StubTransport),
}

#[derive(Clone, Debug)]
pub struct EmailSender {
    transport: EmailTransport,
}

impl EmailSender {
    pub fn from_config(config: &MailerConfig) -> Result<Option<Self>> {
        if config.stub {
            return Ok(Some(Self::stub()));
        }
        if let Some(smtp) = &config.smtp
            && smtp.enable
        {
            return Ok(Some(Self::smtp(smtp)?));
        }
        Ok(None)
    }

    pub fn smtp(config: &SmtpMailer) -> Result<Self> {
        let mut builder = match config.tls_mode() {
            MailerTls::Starttls => {
                lettre::AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                    .map_err(|error| Error::Message(format!("smtp: {error}")))?
                    .port(config.port)
            }
            MailerTls::Implicit => {
                lettre::AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                    .map_err(|error| Error::Message(format!("smtp: {error}")))?
                    .port(config.port)
            }
            MailerTls::None => {
                lettre::AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                    .port(config.port)
            }
        };
        if let Some(auth) = &config.auth {
            builder =
                builder.credentials(Credentials::new(auth.user.clone(), auth.password.clone()));
        }
        Ok(Self {
            transport: EmailTransport::Smtp(builder.build()),
        })
    }

    pub fn stub() -> Self {
        Self {
            transport: EmailTransport::Stub(lettre::transport::stub::StubTransport::new_ok()),
        }
    }

    pub fn install(self) {
        let _ = SENDER.set(self);
    }

    pub fn global() -> Option<&'static Self> {
        SENDER.get()
    }

    pub async fn mail(&self, email: &Email) -> Result<()> {
        if matches!(self.transport, EmailTransport::Stub(_)) {
            info!(
                to = %email.to,
                subject = %email.subject,
                "mailer stub: email not sent"
            );
        }
        let content = MultiPart::alternative_plain_html(email.text.clone(), email.html.clone());
        let mut builder = Message::builder()
            .from(
                email
                    .from
                    .clone()
                    .unwrap_or_else(|| DEFAULT_FROM_SENDER.to_owned())
                    .parse()
                    .map_err(|error| Error::Message(format!("mailer from: {error}")))?,
            )
            .to(email
                .to
                .parse()
                .map_err(|error| Error::Message(format!("mailer to: {error}")))?);
        if let Some(bcc) = &email.bcc {
            builder = builder.bcc(
                bcc.parse()
                    .map_err(|error| Error::Message(format!("mailer bcc: {error}")))?,
            );
        }
        if let Some(cc) = &email.cc {
            builder = builder.cc(cc
                .parse()
                .map_err(|error| Error::Message(format!("mailer cc: {error}")))?);
        }
        if let Some(reply_to) = &email.reply_to {
            builder = builder.reply_to(
                reply_to
                    .parse()
                    .map_err(|error| Error::Message(format!("mailer reply-to: {error}")))?,
            );
        }
        let message = builder
            .subject(email.subject.clone())
            .multipart(content)
            .map_err(|error| Error::Message(format!("mailer message: {error}")))?;
        match &self.transport {
            EmailTransport::Smtp(transport) => {
                transport
                    .send(message)
                    .await
                    .map_err(|error| Error::Message(format!("smtp send: {error}")))?;
            }
            EmailTransport::Stub(transport) => {
                transport
                    .send(&message)
                    .map_err(|error| Error::Message(format!("stub send: {error}")))?;
            }
        }
        Ok(())
    }
}
