use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lp_course_engine::Locale;

use super::domain::User;

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub email_verified: bool,
    pub pending_email: Option<String>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
            locale: user.locale,
            created_at: user.created_at,
            email_verified: user.email_verified_at.is_some(),
            pending_email: user.pending_email,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub locale: Option<Locale>,
}
