use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub email_verified_at: Option<DateTime<Utc>>,
    /// New address of an email change that is waiting for confirmation.
    pub pending_email: Option<String>,
}

pub const ROLE_SUPER_ADMIN: &str = "super_admin";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserCredentials {
    pub id: Uuid,
    pub password_hash: String,
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn is_plausible_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(char::is_whitespace)
        && (3..=254).contains(&email.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_email_shape() {
        assert!(is_plausible_email("ada@example.com"));
        assert!(!is_plausible_email("ada@localhost"));
        assert!(!is_plausible_email("ada example@x.io"));
        assert!(!is_plausible_email("@example.com"));
    }
}
