use serde::{Deserialize, Serialize};

use lp_course_engine::Locale;

use crate::domains::users::dto::UserDto;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub locale: Locale,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserDto,
}
