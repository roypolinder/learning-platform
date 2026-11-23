//! Account management that involves the learner's mailbox: verifying the address, resetting
//! a forgotten password, changing the address, changing the password and deleting the account.
//!
//! Every operation on an existing account acts on the signed-in user only (or on the user a
//! single-use emailed link belongs to). No endpoint here takes a user id.

use chrono::Utc;
use uuid::Uuid;

use super::domain::{ConsumedToken, TokenPurpose};
use super::dto::{
    ChangeEmailRequest, ChangePasswordRequest, DeleteAccountRequest, ResetPasswordRequest,
    TokenResult,
};
use super::repository;
use crate::domains::auth::service::{confirm_password, hash_password_blocking, password_problem};
use crate::domains::users::domain::{ROLE_SUPER_ADMIN, User, is_plausible_email, normalize_email};
use crate::domains::users::repository as users;
use crate::error::{ApiError, ApiResult, FieldError};
use crate::mail::{self, OutgoingEmail, templates};
use crate::observability::AUDIT;
use crate::security::{generate_token, hash_token};
use crate::state::AppState;

const EMAIL_TOKEN_PREFIX: &str = "lp_mail_";

fn invalid_link() -> ApiError {
    ApiError::BadRequest {
        code: "invalid_token",
        message: "This link is invalid, has expired or was already used.".into(),
    }
}

/// Creates a single-use link and returns its URL. The token sits in the fragment so it never
/// reaches server or proxy logs.
async fn issue_link(
    state: &AppState,
    user_id: Uuid,
    purpose: TokenPurpose,
    email: &str,
) -> ApiResult<String> {
    let token = generate_token(EMAIL_TOKEN_PREFIX).map_err(ApiError::internal)?;
    repository::insert_token(
        &state.db,
        user_id,
        purpose,
        email,
        &hash_token(&token),
        Utc::now() + purpose.lifetime(),
    )
    .await?;
    let page = match purpose {
        TokenPurpose::ResetPassword => "reset-password",
        TokenPurpose::VerifyEmail | TokenPurpose::ChangeEmail => "verify-email",
    };
    Ok(format!(
        "{}/{page}#token={token}",
        state.config.web_base_url
    ))
}

/// Queues an email unless this recipient already got too many recently.
fn send_limited(state: &AppState, email: OutgoingEmail) -> ApiResult<()> {
    state
        .mail_limiter
        .check(&format!("mail:{}", email.to.to_lowercase()))?;
    mail::send_in_background(&state.mailer, email);
    Ok(())
}

pub async fn send_verification(state: &AppState, user: &User) -> ApiResult<()> {
    if user.email_verified_at.is_some() {
        return Err(ApiError::Conflict {
            code: "already_verified",
            message: "This email address is already verified.".into(),
        });
    }
    // Only the newest link should work.
    repository::invalidate_tokens(&state.db, user.id, TokenPurpose::VerifyEmail).await?;
    let link = issue_link(state, user.id, TokenPurpose::VerifyEmail, &user.email).await?;
    send_limited(
        state,
        templates::verify_email(&user.locale, &user.email, &user.display_name, &link),
    )
}

pub async fn resend_verification(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    send_verification(state, &user).await
}

/// Always succeeds from the caller's point of view, so it cannot reveal which emails have an
/// account. The lookup and email happen in the background for the same reason (timing).
pub fn forgot_password(state: &AppState, email: &str) {
    let state = state.clone();
    let email = normalize_email(email);
    if !is_plausible_email(&email) {
        return;
    }
    tokio::spawn(async move {
        let result: ApiResult<()> = async {
            let Some(user) = users::find_by_email(&state.db, &email).await? else {
                return Ok(());
            };
            let link =
                issue_link(&state, user.id, TokenPurpose::ResetPassword, &user.email).await?;
            send_limited(
                &state,
                templates::reset_password(&user.locale, &user.email, &user.display_name, &link),
            )
        }
        .await;
        if let Err(error) = result {
            tracing::warn!(%error, "password reset email not sent");
        }
    });
}

pub async fn reset_password(state: &AppState, request: ResetPasswordRequest) -> ApiResult<()> {
    if let Some(problem) = password_problem("password", &request.password) {
        return Err(ApiError::Validation(vec![problem]));
    }
    let password_hash = hash_password_blocking(request.password).await?;
    let mut tx = state.db.begin().await?;
    let token = repository::consume_token(
        &mut *tx,
        &hash_token(request.token.trim()),
        &[TokenPurpose::ResetPassword],
    )
    .await?
    .ok_or_else(invalid_link)?;
    // The link proves control of the mailbox it was sent to, and that must still be the
    // account's address.
    if !users::mark_email_verified(&mut *tx, token.user_id, &token.email).await? {
        return Err(invalid_link());
    }
    users::set_password_hash(&mut *tx, token.user_id, &password_hash).await?;
    repository::invalidate_tokens(&mut *tx, token.user_id, TokenPurpose::ResetPassword).await?;
    repository::revoke_sessions(&mut *tx, token.user_id, None).await?;
    tx.commit().await?;
    tracing::info!(target: AUDIT, category = "account", user_id = %token.user_id, "password reset by email link");
    Ok(())
}

/// Follows a verification or email-change link.
pub async fn confirm_email(state: &AppState, token: &str) -> ApiResult<TokenResult> {
    let ConsumedToken {
        user_id,
        purpose,
        email,
    } = repository::consume_token(
        &state.db,
        &hash_token(token.trim()),
        &[TokenPurpose::VerifyEmail, TokenPurpose::ChangeEmail],
    )
    .await?
    .ok_or_else(invalid_link)?;

    match purpose {
        TokenPurpose::ChangeEmail => {
            users::change_email(&state.db, user_id, &email)
                .await?
                .ok_or_else(|| ApiError::Conflict {
                    code: "email_taken",
                    message: "An account with this email already exists.".into(),
                })?;
            // Links sent to the old address must stop working.
            tracing::info!(target: AUDIT, category = "account", %user_id, new_email = %email, "email address changed");
            for stale in [
                TokenPurpose::ChangeEmail,
                TokenPurpose::VerifyEmail,
                TokenPurpose::ResetPassword,
            ] {
                repository::invalidate_tokens(&state.db, user_id, stale).await?;
            }
        }
        _ => {
            if !users::mark_email_verified(&state.db, user_id, &email).await? {
                return Err(invalid_link());
            }
        }
    }
    Ok(TokenResult { purpose, email })
}

/// Sends a confirmation link to the new address and a notice to the current one. The address
/// only changes once the link is opened.
pub async fn request_email_change(
    state: &AppState,
    user_id: Uuid,
    request: ChangeEmailRequest,
) -> ApiResult<User> {
    let new_email = normalize_email(&request.email);
    if !is_plausible_email(&new_email) {
        return Err(ApiError::Validation(vec![FieldError::new(
            "email",
            "enter a valid email address",
        )]));
    }
    confirm_password(state, user_id, request.password).await?;
    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    if new_email == normalize_email(&user.email) {
        return Err(ApiError::Validation(vec![FieldError::new(
            "email",
            "is already your email address",
        )]));
    }
    if users::email_taken(&state.db, &new_email).await? {
        return Err(ApiError::Conflict {
            code: "email_taken",
            message: "An account with this email already exists.".into(),
        });
    }
    state.mail_limiter.check(&format!("mail:{}", new_email))?;
    repository::invalidate_tokens(&state.db, user.id, TokenPurpose::ChangeEmail).await?;
    let link = issue_link(state, user.id, TokenPurpose::ChangeEmail, &new_email).await?;
    mail::send_in_background(
        &state.mailer,
        templates::confirm_new_email(&user.locale, &new_email, &user.display_name, &link),
    );
    mail::send_in_background(
        &state.mailer,
        templates::email_change_requested(
            &user.locale,
            &user.email,
            &user.display_name,
            &new_email,
        ),
    );
    users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)
}

pub async fn cancel_email_change(state: &AppState, user_id: Uuid) -> ApiResult<User> {
    repository::invalidate_tokens(&state.db, user_id, TokenPurpose::ChangeEmail).await?;
    users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)
}

/// Changes the password and signs out every other browser session.
pub async fn change_password(
    state: &AppState,
    user_id: Uuid,
    current_session: Uuid,
    request: ChangePasswordRequest,
) -> ApiResult<()> {
    if let Some(problem) = password_problem("new_password", &request.new_password) {
        return Err(ApiError::Validation(vec![problem]));
    }
    confirm_password(state, user_id, request.current_password).await?;
    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let password_hash = hash_password_blocking(request.new_password).await?;
    let mut tx = state.db.begin().await?;
    users::set_password_hash(&mut *tx, user_id, &password_hash).await?;
    repository::invalidate_tokens(&mut *tx, user_id, TokenPurpose::ResetPassword).await?;
    repository::revoke_sessions(&mut *tx, user_id, Some(current_session)).await?;
    tx.commit().await?;
    tracing::info!(target: AUDIT, category = "account", %user_id, "password changed");
    mail::send_in_background(
        &state.mailer,
        templates::password_changed(&user.locale, &user.email, &user.display_name),
    );
    Ok(())
}

/// Deletes the signed-in user's own account and everything it owns.
pub async fn delete_account(
    state: &AppState,
    user_id: Uuid,
    request: DeleteAccountRequest,
) -> ApiResult<()> {
    confirm_password(state, user_id, request.password).await?;
    let user = users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let mut tx = state.db.begin().await?;
    // Lock the admin rows so two admins cannot delete themselves at the same time.
    if user.role == ROLE_SUPER_ADMIN && users::count_super_admins(&mut *tx).await? <= 1 {
        return Err(ApiError::Conflict {
            code: "last_super_admin",
            message: "You are the only super admin. Make someone else super admin first.".into(),
        });
    }
    if !users::delete(&mut *tx, user_id).await? {
        return Err(ApiError::Unauthorized);
    }
    tx.commit().await?;
    tracing::info!(target: AUDIT, category = "account", %user_id, email = %user.email, "account deleted by its owner");
    mail::send_in_background(
        &state.mailer,
        templates::account_deleted(&user.locale, &user.email, &user.display_name),
    );
    Ok(())
}
