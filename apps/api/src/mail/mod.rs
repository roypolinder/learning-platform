//! Account emails (verification, password reset, email changes). Production sends them
//! through any SMTP provider; development without `SMTP_URL` writes them to the log so the
//! links can be opened from the terminal.

pub mod templates;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingEmail {
    pub to: String,
    pub subject: String,
    /// Plain text. Transactional mail needs no HTML, and plain text is never flagged for it.
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail configuration is invalid: {0}")]
    Config(String),
    #[error("could not send email: {0}")]
    Send(String),
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, email: OutgoingEmail) -> Result<(), MailError>;
}

/// Picks SMTP when configured, otherwise the log.
pub fn from_config(config: &AppConfig) -> Result<Arc<dyn Mailer>, MailError> {
    match &config.smtp_url {
        Some(url) => Ok(Arc::new(SmtpMailer::new(url.expose(), &config.mail_from)?)),
        None => {
            if config.web_dist_dir.is_some() {
                tracing::warn!(
                    "SMTP_URL is not set: account emails are only written to the log, so \
                     learners cannot verify their email or reset their password"
                );
            }
            Ok(Arc::new(LogMailer))
        }
    }
}

/// Sends without making the request wait for the mail server. Failures are logged; the
/// learner can ask for a new email.
pub fn send_in_background(mailer: &Arc<dyn Mailer>, email: OutgoingEmail) {
    let mailer = Arc::clone(mailer);
    tokio::spawn(async move {
        let subject = email.subject.clone();
        if let Err(error) = mailer.send(email).await {
            tracing::error!(%error, %subject, "account email not sent");
        }
    });
}

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    /// `url` follows lettre's format: `smtps://user:pass@host:465` (implicit TLS) or
    /// `smtp://user:pass@host:587?tls=required` (STARTTLS).
    pub fn new(url: &str, from: &str) -> Result<Self, MailError> {
        check_smtp_url(url)?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            .map_err(|error| MailError::Config(format!("SMTP_URL: {error}")))?
            .build();
        let from = from
            .parse()
            .map_err(|error| MailError::Config(format!("MAIL_FROM: {error}")))?;
        Ok(Self { transport, from })
    }
}

/// Catches common mistakes with a clear message. Never echoes the URL: it holds the password.
fn check_smtp_url(url: &str) -> Result<(), MailError> {
    let problem = if !(url.starts_with("smtps://") || url.starts_with("smtp://")) {
        let start: String = url.chars().take_while(|c| *c != ':' && *c != '/').collect();
        Some(format!(
            "SMTP_URL must start with smtps:// or smtp://, but starts with `{start}`. \
             Check for a duplicated `SMTP_URL=` or quotes in the configuration file"
        ))
    } else if !url.contains('@') {
        Some("SMTP_URL needs credentials: smtps://USER:PASSWORD@HOST:465".into())
    } else if url.matches('@').count() > 1 {
        Some("SMTP_URL contains more than one `@`; write the `@` in the user name as %40".into())
    } else {
        None
    };
    problem.map_or(Ok(()), |problem| Err(MailError::Config(problem)))
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, email: OutgoingEmail) -> Result<(), MailError> {
        let to: Mailbox = email
            .to
            .parse()
            .map_err(|error| MailError::Send(format!("recipient: {error}")))?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(email.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(email.body)
            .map_err(|error| MailError::Send(error.to_string()))?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| MailError::Send(error.to_string()))
    }
}

/// Development fallback: the email, including its link, goes to the API log.
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, email: OutgoingEmail) -> Result<(), MailError> {
        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "email not sent (SMTP_URL is not set):\n{}",
            email.body
        );
        Ok(())
    }
}

/// Keeps emails in memory. Used by the integration tests to follow links.
#[derive(Clone, Default)]
pub struct MemoryMailer {
    sent: Arc<Mutex<Vec<OutgoingEmail>>>,
}

impl MemoryMailer {
    pub fn sent(&self) -> Vec<OutgoingEmail> {
        self.sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, email: OutgoingEmail) -> Result<(), MailError> {
        self.sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(email);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn explains_broken_smtp_urls_without_leaking_the_password() {
        let error = |url: &str| check_smtp_url(url).unwrap_err().to_string();
        let duplicated = error("SMTP_URL=smtps://a%40b.nl:secret@smtp.x:465");
        assert!(
            duplicated.contains("starts with `SMTP_URL=smtps`"),
            "{duplicated}"
        );
        assert!(!duplicated.contains("secret"));
        assert!(error("smtps://a@b.nl:secret@smtp.x:465").contains("%40"));
        assert!(error("smtps://smtp.x:465").contains("credentials"));
        assert!(check_smtp_url("smtps://a%40b.nl:p%24ss@smtp.transip.email:465").is_ok());
        assert!(
            SmtpMailer::new("smtps://a%40b.nl:pw@smtp.transip.email:465", "LP <a@b.nl>").is_ok()
        );
    }
}
