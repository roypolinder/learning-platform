//! Account emails (verification, password reset, email changes). Production sends them
//! through any SMTP provider; development without `SMTP_URL` writes them to the log so the
//! links can be opened from the terminal.

pub mod templates;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::message::MultiPart;
use lettre::message::header::ContentType;
use lettre::transport::smtp::extension::ClientId;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::AppConfig;
use crate::observability::AUDIT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingEmail {
    pub to: String,
    pub subject: String,
    /// Plain text; always sent, for clients that don't show HTML.
    pub body: String,
    /// Optional HTML version of the same content.
    pub html: Option<String>,
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
    /// Returns the receiving server's reply, e.g. `250 2.0.0 Ok: queued as 4Xyz`.
    async fn send(&self, email: OutgoingEmail) -> Result<String, MailError>;
}

/// Picks SMTP when configured, otherwise the log.
pub fn from_config(config: &AppConfig) -> Result<Arc<dyn Mailer>, MailError> {
    match &config.smtp_url {
        Some(url) => {
            let host = url::Url::parse(&config.public_base_url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned));
            Ok(Arc::new(SmtpMailer::new(
                url.expose(),
                &config.mail_from,
                host.as_deref(),
            )?))
        }
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

/// Sends without making the request wait for the mail server. The outcome, including the
/// server's reply, lands in the admin log; the learner can ask for a new email.
pub fn send_in_background(mailer: &Arc<dyn Mailer>, email: OutgoingEmail) {
    let mailer = Arc::clone(mailer);
    tokio::spawn(async move {
        let _ = send_logged(mailer.as_ref(), email).await;
    });
}

/// Sends now and records the result in the admin log.
pub async fn send_logged(mailer: &dyn Mailer, email: OutgoingEmail) -> Result<String, MailError> {
    let (to, subject) = (email.to.clone(), email.subject.clone());
    let result = mailer.send(email).await;
    match &result {
        Ok(response) => tracing::info!(
            target: AUDIT,
            category = "mail",
            %to,
            %subject,
            %response,
            "email sent"
        ),
        Err(error) => tracing::error!(
            target: AUDIT,
            category = "mail",
            %to,
            %subject,
            %error,
            "email not sent"
        ),
    }
    result
}

pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

/// Spam filters penalise a short host name such as `learn` in the SMTP greeting, so greet with
/// the platform's public host name when it is a real domain.
fn hello_name(public_host: Option<&str>) -> Option<ClientId> {
    public_host
        .filter(|host| host.contains('.') && host.parse::<std::net::IpAddr>().is_err())
        .map(|host| ClientId::Domain(host.to_owned()))
}

impl SmtpMailer {
    /// `url` follows lettre's format: `smtps://user:pass@host:465` (implicit TLS) or
    /// `smtp://user:pass@host:587?tls=required` (STARTTLS).
    pub fn new(url: &str, from: &str, public_host: Option<&str>) -> Result<Self, MailError> {
        check_smtp_url(url)?;
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            .map_err(|error| MailError::Config(format!("SMTP_URL: {error}")))?;
        if let Some(name) = hello_name(public_host) {
            builder = builder.hello_name(name);
        }
        let transport = builder.build();
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
    async fn send(&self, email: OutgoingEmail) -> Result<String, MailError> {
        let to: Mailbox = email
            .to
            .parse()
            .map_err(|error| MailError::Send(format!("recipient: {error}")))?;
        // A Message-ID is expected on every email; lettre leaves it out unless asked, and its
        // default would use the server's short host name.
        let message_id = format!(
            "<{}@{}>",
            uuid::Uuid::now_v7().simple(),
            self.from.email.domain()
        );
        let builder = Message::builder()
            .message_id(Some(message_id))
            .from(self.from.clone())
            .to(to)
            .subject(email.subject);
        let message = match email.html {
            Some(html) => builder.multipart(MultiPart::alternative_plain_html(email.body, html)),
            None => builder.header(ContentType::TEXT_PLAIN).body(email.body),
        }
        .map_err(|error| MailError::Send(error.to_string()))?;
        let response = self
            .transport
            .send(message)
            .await
            .map_err(|error| MailError::Send(error.to_string()))?;
        let text: Vec<&str> = response.message().collect();
        Ok(format!("{} {}", response.code(), text.join(" "))
            .trim()
            .to_owned())
    }
}

/// Development fallback: the email, including its link, goes to the API log.
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, email: OutgoingEmail) -> Result<String, MailError> {
        tracing::info!(
            to = %email.to,
            subject = %email.subject,
            "email not sent (SMTP_URL is not set):\n{}",
            email.body
        );
        Ok("not sent: SMTP_URL is not set, the email is in the server log".into())
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
    async fn send(&self, email: OutgoingEmail) -> Result<String, MailError> {
        self.sent
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(email);
        Ok("kept in memory".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_with_a_real_domain_only() {
        assert!(matches!(
            hello_name(Some("learn.polinderit.nl")),
            Some(ClientId::Domain(name)) if name == "learn.polinderit.nl"
        ));
        assert!(hello_name(Some("localhost")).is_none());
        assert!(hello_name(Some("203.0.113.7")).is_none());
        assert!(hello_name(None).is_none());
    }

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
            SmtpMailer::new(
                "smtps://a%40b.nl:pw@smtp.transip.email:465",
                "LP <a@b.nl>",
                Some("learn.polinderit.nl")
            )
            .is_ok()
        );
    }
}
