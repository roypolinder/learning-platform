mod support;

use chrono::Utc;
use lp_api::observability::{LogRecord, insert};
use reqwest::StatusCode;
use serde_json::{Map, json};
use support::TestApp;

fn record(level: &'static str, category: &str, message: &str, to: &str) -> LogRecord {
    let mut fields = Map::new();
    fields.insert("to".into(), json!(to));
    LogRecord {
        created_at: Utc::now(),
        level,
        target: "audit".into(),
        category: Some(category.into()),
        message: message.into(),
        fields,
    }
}

#[tokio::test]
async fn super_admins_read_the_log_and_test_email() {
    let app = TestApp::spawn().await;
    let admin = app.user("admin").await;
    app.promote("admin").await;
    let ada = app.user("ada").await;

    let pool = app.pool().await;
    insert(
        &pool,
        &record("info", "mail", "email sent", "ada@example.com"),
    )
    .await
    .unwrap();
    insert(
        &pool,
        &record("error", "mail", "email not sent", "bob_x@example.com"),
    )
    .await
    .unwrap();
    insert(&pool, &record("warn", "server", "slow request", "-"))
        .await
        .unwrap();

    assert_eq!(
        ada.get("/api/v1/admin/logs").await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        ada.post("/api/v1/admin/mail/test", json!({}))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );

    let all = admin.json("/api/v1/admin/logs").await;
    assert_eq!(all["logs"]["total"], 3);
    assert_eq!(all["counts"]["errors_24h"], 1);
    assert_eq!(all["counts"]["warnings_24h"], 1);
    assert_eq!(
        all["logs"]["items"][0]["message"], "slow request",
        "newest first"
    );

    let errors = admin.json("/api/v1/admin/logs?level=error").await;
    assert_eq!(errors["logs"]["total"], 1);
    assert_eq!(
        errors["logs"]["items"][0]["fields"]["to"],
        "bob_x@example.com"
    );
    // Search looks inside the fields, with `_` taken literally.
    let found = admin.json("/api/v1/admin/logs?q=bob_x").await;
    assert_eq!(found["logs"]["total"], 1);
    let mail = admin.json("/api/v1/admin/logs?category=mail").await;
    assert_eq!(mail["logs"]["total"], 2);

    let sent = admin.post_ok("/api/v1/admin/mail/test", json!({})).await;
    assert_eq!(sent["to"], "admin@example.com");
    assert_eq!(sent["response"], "kept in memory");
    assert!(
        app.mail
            .sent()
            .iter()
            .any(|email| email.to == "admin@example.com" && email.subject.contains("email check"))
    );
    app.cleanup().await;
}
