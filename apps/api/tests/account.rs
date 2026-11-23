mod support;

use reqwest::StatusCode;
use serde_json::{Value, json};
use support::{TestApp, TestUser, sign_in};

const PASSWORD: &str = "a long enough password";

async fn me(user: &TestUser) -> Value {
    user.json("/api/v1/users/me").await
}

#[tokio::test]
async fn registration_sends_a_verification_link() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    assert_eq!(me(&ada).await["email_verified"], false);

    let token = app.emailed_token(&ada.email, "verify-email").await;
    // The link works without a session, e.g. when opened on a phone.
    let anonymous = reqwest::Client::new();
    let confirm = |token: String| {
        anonymous
            .post(app.url("/api/v1/auth/email/confirm"))
            .header("x-requested-with", "lp-web")
            .json(&json!({ "token": token }))
            .send()
    };
    let response = confirm(token.clone()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["purpose"], "verify_email");
    assert_eq!(me(&ada).await["email_verified"], true);

    // Links work once.
    let again = confirm(token).await.unwrap();
    assert_eq!(again.status(), StatusCode::BAD_REQUEST);
    let again: Value = again.json().await.unwrap();
    assert_eq!(again["error"]["code"], "invalid_token");

    let resend = ada
        .post("/api/v1/users/me/email/verification", json!({}))
        .await;
    assert_eq!(resend.status(), StatusCode::CONFLICT);

    // Only the newest verification link works.
    let bob = app.user("bob").await;
    let first = app.emailed_token(&bob.email, "verify-email").await;
    let resend = bob
        .post("/api/v1/users/me/email/verification", json!({}))
        .await;
    assert_eq!(resend.status(), StatusCode::ACCEPTED);
    let mut second = first.clone();
    for _ in 0..50 {
        second = app.emailed_token(&bob.email, "verify-email").await;
        if second != first {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_ne!(first, second);
    assert_eq!(
        confirm(first).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(confirm(second).await.unwrap().status(), StatusCode::OK);
    app.cleanup().await;
}

#[tokio::test]
async fn forgotten_passwords_are_reset_by_email() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let client = reqwest::Client::new();
    let forgot = |email: &str| {
        client
            .post(app.url("/api/v1/auth/password/forgot"))
            .header("x-requested-with", "lp-web")
            .json(&json!({ "email": email }))
            .send()
    };

    // Unknown addresses get the same answer and no email.
    let unknown = forgot("nobody@example.com").await.unwrap();
    assert_eq!(unknown.status(), StatusCode::ACCEPTED);
    let known = forgot("ADA@example.com").await.unwrap();
    assert_eq!(known.status(), StatusCode::ACCEPTED);
    let token = app.emailed_token(&ada.email, "reset-password").await;
    assert_eq!(app.emails_to("nobody@example.com"), 0);

    let reset = |token: &str, password: &str| {
        client
            .post(app.url("/api/v1/auth/password/reset"))
            .header("x-requested-with", "lp-web")
            .json(&json!({ "token": token, "password": password }))
            .send()
    };
    let short = reset(&token, "short").await.unwrap();
    assert_eq!(short.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let wrong = reset("lp_mail_not-a-real-token", "a brand new password")
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);

    let ok = reset(&token, "a brand new password").await.unwrap();
    assert_eq!(ok.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        reset(&token, "yet another password")
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST,
        "reset links work once"
    );

    // Existing sessions are signed out; only the new password works.
    assert_eq!(
        ada.get("/api/v1/users/me").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(sign_in(&app, &ada.email, PASSWORD).await.is_err());
    let cookie = sign_in(&app, &ada.email, "a brand new password")
        .await
        .expect("new password works");
    // Following the link proved the address too.
    let me: Value = client
        .get(app.url("/api/v1/users/me"))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["email_verified"], true);
    app.cleanup().await;
}

#[tokio::test]
async fn email_changes_need_the_password_and_confirmation() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let bob = app.user("bob").await;
    let old_verification = app.emailed_token(&ada.email, "verify-email").await;

    let wrong_password = ada
        .post(
            "/api/v1/users/me/email",
            json!({ "email": "ada.new@example.com", "password": "not my password" }),
        )
        .await;
    assert_eq!(wrong_password.status(), StatusCode::BAD_REQUEST);
    let body: Value = wrong_password.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_password");

    let taken = ada
        .post(
            "/api/v1/users/me/email",
            json!({ "email": bob.email, "password": PASSWORD }),
        )
        .await;
    assert_eq!(taken.status(), StatusCode::CONFLICT);

    let requested = ada
        .post(
            "/api/v1/users/me/email",
            json!({ "email": "Ada.New@example.com", "password": PASSWORD }),
        )
        .await;
    assert_eq!(requested.status(), StatusCode::ACCEPTED);
    let pending: Value = requested.json().await.unwrap();
    assert_eq!(
        pending["email"], "ada@example.com",
        "unchanged until confirmed"
    );
    assert_eq!(pending["pending_email"], "ada.new@example.com");

    let token = app
        .emailed_token("ada.new@example.com", "verify-email")
        .await;
    // The current address is told about the change.
    for _ in 0..50 {
        if app
            .mail
            .sent()
            .iter()
            .any(|mail| mail.to == ada.email && mail.body.contains("ada.new@example.com"))
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        app.mail
            .sent()
            .iter()
            .any(|mail| mail.to == ada.email && mail.body.contains("ada.new@example.com"))
    );

    let confirmed = reqwest::Client::new()
        .post(app.url("/api/v1/auth/email/confirm"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "token": token }))
        .send()
        .await
        .unwrap();
    assert_eq!(confirmed.status(), StatusCode::OK);
    let confirmed: Value = confirmed.json().await.unwrap();
    assert_eq!(confirmed["purpose"], "change_email");

    let profile = me(&ada).await;
    assert_eq!(profile["email"], "ada.new@example.com");
    assert_eq!(profile["email_verified"], true);
    assert!(profile["pending_email"].is_null());
    assert!(sign_in(&app, "ada.new@example.com", PASSWORD).await.is_ok());
    assert!(sign_in(&app, "ada@example.com", PASSWORD).await.is_err());

    // A link sent to the old address no longer works.
    let stale = reqwest::Client::new()
        .post(app.url("/api/v1/auth/email/confirm"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "token": old_verification }))
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::BAD_REQUEST);

    // A pending change can be cancelled.
    bob.post_ok(
        "/api/v1/users/me/email",
        json!({ "email": "bob.new@example.com", "password": PASSWORD }),
    )
    .await;
    let bob_token = app
        .emailed_token("bob.new@example.com", "verify-email")
        .await;
    let cancelled: Value = bob
        .send(
            reqwest::Method::DELETE,
            "/api/v1/users/me/email/pending",
            None,
        )
        .await
        .json()
        .await
        .unwrap();
    assert!(cancelled["pending_email"].is_null());
    let late = reqwest::Client::new()
        .post(app.url("/api/v1/auth/email/confirm"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "token": bob_token }))
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn password_changes_sign_out_other_sessions() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let other_device = sign_in(&app, &ada.email, PASSWORD).await.unwrap();

    let wrong = ada
        .post(
            "/api/v1/users/me/password",
            json!({ "current_password": "wrong password", "new_password": "a brand new password" }),
        )
        .await;
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);

    let changed = ada
        .post(
            "/api/v1/users/me/password",
            json!({ "current_password": PASSWORD, "new_password": "a brand new password" }),
        )
        .await;
    assert_eq!(changed.status(), StatusCode::NO_CONTENT);
    assert_eq!(ada.get("/api/v1/users/me").await.status(), StatusCode::OK);
    let other = reqwest::Client::new()
        .get(app.url("/api/v1/users/me"))
        .header("cookie", other_device)
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::UNAUTHORIZED);
    assert!(
        sign_in(&app, &ada.email, "a brand new password")
            .await
            .is_ok()
    );
    app.cleanup().await;
}

#[tokio::test]
async fn learners_can_only_delete_their_own_account() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let bob = app.user("bob").await;
    let bob_id = me(&bob).await["id"].as_str().unwrap().to_owned();
    ada.post_ok(
        "/api/v1/notes",
        json!({ "course": "mcp-first-connection", "body": "mine" }),
    )
    .await;

    // There is no way to name another account.
    for path in [
        format!("/api/v1/users/{bob_id}/delete"),
        format!("/api/v1/users/{bob_id}"),
    ] {
        let response = ada.post(&path, json!({ "password": PASSWORD })).await;
        assert!(
            matches!(
                response.status(),
                StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
            ),
            "{path}: {}",
            response.status()
        );
        let response = ada
            .send(
                reqwest::Method::DELETE,
                &path,
                Some(json!({ "password": PASSWORD })),
            )
            .await;
        assert!(matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ));
    }
    let unknown_field = ada
        .post(
            "/api/v1/users/me/delete",
            json!({ "password": PASSWORD, "user_id": bob_id }),
        )
        .await;
    assert_eq!(unknown_field.status(), StatusCode::BAD_REQUEST);

    let wrong = ada
        .post(
            "/api/v1/users/me/delete",
            json!({ "password": "not my password" }),
        )
        .await;
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
    assert_eq!(ada.get("/api/v1/users/me").await.status(), StatusCode::OK);

    // Without a session nothing can be deleted.
    let anonymous = reqwest::Client::new()
        .post(app.url("/api/v1/users/me/delete"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "password": PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let deleted = ada
        .post("/api/v1/users/me/delete", json!({ "password": PASSWORD }))
        .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert!(
        deleted
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|cookie| cookie.contains("Max-Age=0"))
    );
    assert_eq!(
        ada.get("/api/v1/users/me").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(sign_in(&app, &ada.email, PASSWORD).await.is_err());

    // Bob is untouched, and the address is free again.
    assert_eq!(me(&bob).await["id"], bob_id.as_str());
    let again = app.user("ada").await;
    let notes = again.json("/api/v1/notes").await;
    assert_eq!(notes["items"].as_array().map(Vec::len), Some(0));
    app.cleanup().await;
}

#[tokio::test]
async fn the_last_super_admin_cannot_delete_themselves() {
    let app = TestApp::spawn().await;
    let root = app.user("root").await;
    app.promote("root").await;
    let blocked = root
        .post("/api/v1/users/me/delete", json!({ "password": PASSWORD }))
        .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);

    let second = app.user("second").await;
    app.promote("second").await;
    let allowed = root
        .post("/api/v1/users/me/delete", json!({ "password": PASSWORD }))
        .await;
    assert_eq!(allowed.status(), StatusCode::NO_CONTENT);
    let blocked = second
        .post("/api/v1/users/me/delete", json!({ "password": PASSWORD }))
        .await;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
    app.cleanup().await;
}

#[tokio::test]
async fn super_admins_see_who_signed_up() {
    let app = TestApp::spawn().await;
    let admin = app.user("admin").await;
    app.promote("admin").await;
    let ada = app.user("ada").await;
    app.user("bot_123").await;

    assert_eq!(
        ada.get("/api/v1/admin/users").await.status(),
        StatusCode::FORBIDDEN
    );

    let all = admin.json("/api/v1/admin/users").await;
    assert_eq!(all["summary"]["total"], 3);
    assert_eq!(all["summary"]["last_24_hours"], 3);
    assert_eq!(all["summary"]["verified"], 0);
    assert_eq!(all["users"]["total"], 3);
    // Newest first.
    assert_eq!(all["users"]["items"][0]["email"], "bot_123@example.com");
    assert_eq!(all["users"]["items"][0]["sign_ins"], 1);
    assert_eq!(
        all["summary"]["signups_per_day"][0]["count"]
            .as_i64()
            .unwrap(),
        3
    );

    // `_` is matched literally, not as a wildcard.
    let search = admin.json("/api/v1/admin/users?q=t_1").await;
    assert_eq!(search["users"]["total"], 1);
    assert_eq!(search["users"]["items"][0]["display_name"], "bot_123");

    let token = app.emailed_token(&ada.email, "verify-email").await;
    ada.post_ok("/api/v1/auth/email/confirm", json!({ "token": token }))
        .await;
    let verified = admin.json("/api/v1/admin/users?status=verified").await;
    assert_eq!(verified["users"]["total"], 1);
    assert_eq!(verified["users"]["items"][0]["email"], "ada@example.com");
    let unverified = admin
        .json("/api/v1/admin/users?status=unverified&per_page=1")
        .await;
    assert_eq!(unverified["users"]["total"], 2);
    assert_eq!(unverified["users"]["items"].as_array().unwrap().len(), 1);
    app.cleanup().await;
}
