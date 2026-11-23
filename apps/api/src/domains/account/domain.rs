use chrono::Duration;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPurpose {
    VerifyEmail,
    ResetPassword,
    ChangeEmail,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VerifyEmail => "verify_email",
            Self::ResetPassword => "reset_password",
            Self::ChangeEmail => "change_email",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "verify_email" => Some(Self::VerifyEmail),
            "reset_password" => Some(Self::ResetPassword),
            "change_email" => Some(Self::ChangeEmail),
            _ => None,
        }
    }

    /// Reset links are short-lived because they grant access to the account.
    pub fn lifetime(self) -> Duration {
        match self {
            Self::VerifyEmail => Duration::hours(48),
            Self::ResetPassword => Duration::hours(1),
            Self::ChangeEmail => Duration::hours(24),
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for TokenPurpose {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for TokenPurpose {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let text = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Self::parse(text).ok_or_else(|| format!("unknown token purpose `{text}`").into())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConsumedToken {
    pub user_id: Uuid,
    pub purpose: TokenPurpose,
    /// The address the link was sent to.
    pub email: String,
}
