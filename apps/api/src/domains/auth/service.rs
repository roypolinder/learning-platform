use chrono::{Duration, Utc};
use uuid::Uuid;

use super::dto::{LoginRequest, RegisterRequest};
use super::repository;
use crate::domains::account::service as account;
use crate::domains::users::domain::{User, is_plausible_email, normalize_email};
use crate::domains::users::repository as users;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::security::{
    SESSION_TOKEN_PREFIX, burn_password_check, generate_token, hash_password, hash_token,
    verify_password,
};
use crate::state::AppState;

pub const SESSION_LIFETIME: Duration = Duration::days(14);
const MIN_PASSWORD_LENGTH: usize = 10;

pub struct IssuedSession {
    pub user: User,
    pub token: String,
}

pub async fn register(state: &AppState, request: RegisterRequest) -> ApiResult<IssuedSession> {
    let email = normalize_email(&request.email);
    let display_name = request.display_name.trim().to_owned();
    let mut problems = Vec::new();
    if !is_plausible_email(&email) {
        problems.push(FieldError::new("email", "enter a valid email address"));
    }
    if display_name.is_empty() || display_name.chars().count() > 80 {
        problems.push(FieldError::new(
            "display_name",
            "must be between 1 and 80 characters",
        ));
    }
    problems.extend(password_problem("password", &request.password));
    if !problems.is_empty() {
        return Err(ApiError::Validation(problems));
    }

    let password_hash = hash_password_blocking(request.password).await?;
    let user = users::insert(
        &state.db,
        &email,
        &display_name,
        &password_hash,
        request.locale,
    )
    .await?
    .ok_or_else(|| ApiError::Conflict {
        code: "email_taken",
        message: "An account with this email already exists.".into(),
    })?;
    if let Err(error) = account::send_verification(state, &user).await {
        tracing::warn!(%error, "verification email not queued");
    }
    issue_session(state, user).await
}

/// `None` when the password is acceptable for a new account.
pub fn password_problem(field: &'static str, password: &str) -> Option<FieldError> {
    (password.chars().count() < MIN_PASSWORD_LENGTH || password.len() > 512).then(|| {
        FieldError::new(
            field,
            format!("must be at least {MIN_PASSWORD_LENGTH} characters"),
        )
    })
}

pub async fn hash_password_blocking(password: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)
}

/// Checks the account password again before sensitive changes. Wrong guesses are rate limited
/// per account, so a stolen session cannot be used to brute-force the password.
pub async fn confirm_password(state: &AppState, user_id: Uuid, password: String) -> ApiResult<()> {
    state
        .auth_limiter
        .check(&format!("confirm-password:{user_id}"))?;
    let hash = users::find_password_hash(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let valid = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(ApiError::internal)?;
    if valid {
        Ok(())
    } else {
        Err(ApiError::BadRequest {
            code: "invalid_password",
            message: "The password is incorrect.".into(),
        })
    }
}

pub async fn login(state: &AppState, request: LoginRequest) -> ApiResult<IssuedSession> {
    let email = normalize_email(&request.email);
    let credentials = users::find_credentials_by_email(&state.db, &email).await?;
    let password = request.password;
    let verified = tokio::task::spawn_blocking(move || match credentials {
        Some(credentials) if verify_password(&password, &credentials.password_hash) => {
            Some(credentials.id)
        }
        Some(_) => None,
        None => {
            burn_password_check(&password);
            None
        }
    })
    .await
    .map_err(ApiError::internal)?;

    let user_id = verified.ok_or_else(|| ApiError::BadRequest {
        code: "invalid_credentials",
        message: "Email or password is incorrect.".into(),
    })?;
    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    issue_session(state, user).await
}

async fn issue_session(state: &AppState, user: User) -> ApiResult<IssuedSession> {
    let token = generate_token(SESSION_TOKEN_PREFIX).map_err(ApiError::internal)?;
    repository::insert_session(
        &state.db,
        user.id,
        &hash_token(&token),
        Utc::now() + SESSION_LIFETIME,
    )
    .await?;
    Ok(IssuedSession { user, token })
}
