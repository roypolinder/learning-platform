mod support;

use reqwest::StatusCode;
use serde_json::json;
use support::{TestApp, step_status};

#[tokio::test]
async fn health_and_catalog_are_public() {
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let health: serde_json::Value = client
        .get(app.url("/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["database"], "ok");
    assert_eq!(health["extensions"], 6);

    let list: serde_json::Value = client
        .get(app.url("/api/v1/courses?q=mcp"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list["total"], 2);
    assert_eq!(list["items"][0]["id"], "mcp-first-connection");
    assert_eq!(list["items"][1]["id"], "build-mcp-server");

    // The catalog is a learning path: easier courses first, then by `order`.
    let all: serde_json::Value = client
        .get(app.url("/api/v1/courses?per_page=100"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let rank = |item: &serde_json::Value| match item["difficulty"].as_str() {
        Some("beginner") => 0,
        Some("intermediate") => 1,
        _ => 2,
    };
    let items = all["items"].as_array().unwrap();
    assert!(
        items
            .windows(2)
            .all(|pair| rank(&pair[0]) <= rank(&pair[1]))
    );
    assert_eq!(items[0]["id"], "mcp-first-connection");

    let detail: serde_json::Value = client
        .get(app.url("/api/v1/courses/mcp-first-connection"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        detail["course"]["help"]
            .as_str()
            .is_some_and(|help| !help.is_empty())
    );
    let text = detail.to_string();
    assert!(!text.contains("\"correct\""), "answer keys leaked");
    assert!(!text.contains("required_tool"), "validation rules leaked");

    let missing = client
        .get(app.url("/api/v1/courses/nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = missing.json().await.unwrap();
    assert_eq!(body["error"]["code"], "not_found");
    app.cleanup().await;
}

#[tokio::test]
async fn learner_progresses_through_generic_steps() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;

    let started = ada
        .post_ok("/api/v1/courses/mcp-first-connection/sessions", json!({}))
        .await;
    let session_id = started["session"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        step_status(&started["progress"], "what-is-mcp"),
        "available"
    );
    assert_eq!(step_status(&started["progress"], "primitives"), "locked");

    // Starting again resumes the same session.
    let resumed = ada
        .post_ok("/api/v1/courses/mcp-first-connection/sessions", json!({}))
        .await;
    assert_eq!(resumed["session"]["id"], session_id.as_str());

    let base = format!("/api/v1/sessions/{session_id}");
    ada.post_ok(&format!("{base}/steps/what-is-mcp/open"), json!({}))
        .await;
    let done = ada
        .post_ok(&format!("{base}/steps/what-is-mcp/complete"), json!({}))
        .await;
    assert_eq!(step_status(&done["progress"], "what-is-mcp"), "completed");
    assert_eq!(step_status(&done["progress"], "primitives"), "available");

    // Quizzes cannot be self-completed.
    ada.post_ok(&format!("{base}/steps/primitives/complete"), json!({}))
        .await;
    let refused = ada
        .post(&format!("{base}/steps/check-basics/complete"), json!({}))
        .await;
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);

    let wrong = ada
        .post_ok(
            &format!("{base}/steps/check-basics/submissions"),
            json!({ "payload": { "answers": { "transport": ["stdio"] } } }),
        )
        .await;
    assert_eq!(wrong["outcome"]["status"], "failed");
    assert_eq!(step_status(&wrong["progress"], "check-basics"), "failed");

    let right = ada
        .post_ok(
            &format!("{base}/steps/check-basics/submissions"),
            json!({ "payload": { "answers": { "transport": ["http"], "control": ["tools"], "roles": ["host"], "authorization": ["challenge"] } } }),
        )
        .await;
    assert_eq!(right["outcome"]["status"], "passed");
    assert_eq!(
        step_status(&right["progress"], "review-primitives"),
        "skipped"
    );
    assert_eq!(
        step_status(&right["progress"], "config-example"),
        "available"
    );

    let invalid = ada
        .post(
            &format!("{base}/steps/config-blanks/submissions"),
            json!({ "payload": { "nope": 1 } }),
        )
        .await;
    assert_eq!(invalid.status(), StatusCode::CONFLICT, "locked step");

    // Progress is persisted and survives a fresh read.
    let progress = ada.json(&format!("{base}/progress")).await;
    assert_eq!(progress["completed_required"], 4);
    assert_eq!(progress["current_step_id"], "config-example");
    let events = ada.json(&format!("{base}/events")).await;
    assert!(
        events
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "step.validated")
    );

    let overview = ada.json("/api/v1/progress/overview").await;
    assert_eq!(overview["stats"]["active_courses"], 1);
    assert_eq!(
        overview["sessions"][0]["current_step"]["id"],
        "config-example"
    );
    app.cleanup().await;
}

#[tokio::test]
async fn sessions_are_isolated_between_users() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let eve = app.user("eve").await;
    let started = ada
        .post_ok("/api/v1/courses/agent-skill-basics/sessions", json!({}))
        .await;
    let session_id = started["session"]["id"].as_str().unwrap();

    for path in ["", "/progress", "/events"] {
        let response = eve
            .get(&format!("/api/v1/sessions/{session_id}{path}"))
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
    let response = eve
        .post(
            &format!("/api/v1/sessions/{session_id}/steps/what-are-skills/complete"),
            json!({}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let listed = eve.json("/api/v1/sessions").await;
    assert_eq!(listed["total"], 0);
    app.cleanup().await;
}

#[tokio::test]
async fn authentication_and_csrf_boundaries() {
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let anonymous = client
        .get(app.url("/api/v1/progress/overview"))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let signed_out = client
        .get(app.url("/api/v1/auth/session"))
        .send()
        .await
        .unwrap();
    assert_eq!(signed_out.status(), StatusCode::OK);
    assert!(signed_out.json::<serde_json::Value>().await.unwrap()["user"].is_null());

    let ada = app.user("ada").await;
    assert_eq!(
        ada.json("/api/v1/auth/session").await["user"]["display_name"],
        "ada"
    );
    let no_header = client
        .post(app.url("/api/v1/courses/mcp-first-connection/sessions"))
        .header("cookie", &ada.cookie)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(no_header.status(), StatusCode::BAD_REQUEST);

    let bad_login = client
        .post(app.url("/api/v1/auth/login"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "email": "ada@example.com", "password": "wrong password!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_login.status(), StatusCode::BAD_REQUEST);

    let duplicate = client
        .post(app.url("/api/v1/auth/register"))
        .header("x-requested-with", "lp-web")
        .json(&json!({ "email": "ADA@example.com", "password": "another long password", "display_name": "Ada" }))
        .send()
        .await
        .unwrap();
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    ada.post("/api/v1/auth/logout", json!({})).await;
    assert_eq!(
        ada.get("/api/v1/users/me").await.status(),
        StatusCode::UNAUTHORIZED
    );
    app.cleanup().await;
}

const AUTHORED_COURSE: &str = r#"
id: authored-demo
version: 1.0.0
title: Authored demo
summary: Created in the app.
description: A course created through the authoring API.
category: Testing
difficulty: beginner
estimated_minutes: 5
sections:
  - id: only
    title: Only section
    steps:
      - id: hello
        type: explanation
        title: Hello
        body: Hi there.
"#;

#[tokio::test]
async fn super_admins_author_courses() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let root = app.user("root").await;
    app.promote("root").await;

    let me = root.json("/api/v1/users/me").await;
    assert_eq!(me["role"], "super_admin");
    assert_eq!(
        ada.get("/api/v1/admin/courses").await.status(),
        StatusCode::FORBIDDEN
    );
    let denied = ada
        .post(
            "/api/v1/admin/courses",
            json!({ "source": AUTHORED_COURSE }),
        )
        .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let report = root
        .post_ok(
            "/api/v1/admin/courses/validate",
            json!({ "source": "id: broken\nversion: 1.0.0" }),
        )
        .await;
    assert_eq!(report["valid"], false);
    assert!(!report["issues"].as_array().unwrap().is_empty());

    let file_course = root
        .post(
            "/api/v1/admin/courses",
            json!({ "source": AUTHORED_COURSE.replace("authored-demo", "mcp-first-connection") }),
        )
        .await;
    assert_eq!(file_course.status(), StatusCode::CONFLICT);

    let created = root
        .post(
            "/api/v1/admin/courses",
            json!({ "source": AUTHORED_COURSE }),
        )
        .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let listed = ada.json("/api/v1/courses?q=authored").await;
    assert_eq!(listed["items"][0]["id"], "authored-demo");

    // Once a learner starts a version, its content is frozen.
    let started = ada
        .post_ok("/api/v1/courses/authored-demo/sessions", json!({}))
        .await;
    let session_id = started["session"]["id"].as_str().unwrap();
    ada.post_ok(
        &format!("/api/v1/sessions/{session_id}/steps/hello/complete"),
        json!({}),
    )
    .await;
    let edited = AUTHORED_COURSE.replace("Hi there.", "Hello again.");
    let frozen = root
        .post("/api/v1/admin/courses", json!({ "source": edited }))
        .await;
    assert_eq!(frozen.status(), StatusCode::CONFLICT);
    let bumped = root
        .post_ok(
            "/api/v1/admin/courses",
            json!({ "source": edited.replace("version: 1.0.0", "version: 1.1.0") }),
        )
        .await;
    assert_eq!(bumped["status"], "new_version");

    let detail = root.json("/api/v1/admin/courses/authored-demo").await;
    assert!(
        detail["definition"]
            .as_str()
            .unwrap()
            .contains("Hello again.")
    );

    let catalog = ada.json("/api/v1/courses").await;
    assert_eq!(catalog["progress"]["authored-demo"]["state"], "completed");
    app.cleanup().await;
}

#[tokio::test]
async fn outdated_sessions_move_to_the_current_version() {
    let app = TestApp::spawn().await;
    let root = app.user("root").await;
    app.promote("root").await;
    let course = |version: &str, second: &str| {
        format!(
            "id: upgrade-demo\nversion: {version}\ntitle: Upgrade demo\nsummary: s\ndescription: d\ncategory: Testing\ndifficulty: beginner\nestimated_minutes: 3\nsections:\n  - id: a\n    title: A\n    steps:\n      - id: one\n        type: explanation\n        title: One\n        body: Same in both versions\n      - id: two\n        type: explanation\n        title: Two\n        body: {second}\n      - id: three\n        type: explanation\n        title: Three\n        body: Three\n"
        )
    };
    root.post_ok(
        "/api/v1/admin/courses",
        json!({ "source": course("1.0.0", "Old text") }),
    )
    .await;
    let started = root
        .post_ok("/api/v1/courses/upgrade-demo/sessions", json!({}))
        .await;
    let old_id = started["session"]["id"].as_str().unwrap().to_owned();
    for step in ["one", "two"] {
        root.post_ok(
            &format!("/api/v1/sessions/{old_id}/steps/{step}/complete"),
            json!({}),
        )
        .await;
    }

    root.post_ok(
        "/api/v1/admin/courses",
        json!({ "source": course("1.1.0", "New text") }),
    )
    .await;
    let detail = root.json(&format!("/api/v1/sessions/{old_id}")).await;
    let new_id = detail["session"]["id"].as_str().unwrap();
    assert_ne!(new_id, old_id);
    assert_eq!(detail["course"]["version"], "1.1.0");
    // Unchanged steps keep their progress; changed steps are done again.
    assert_eq!(step_status(&detail["progress"], "one"), "completed");
    assert_eq!(step_status(&detail["progress"], "two"), "available");

    let resumed = root
        .post_ok("/api/v1/courses/upgrade-demo/sessions", json!({}))
        .await;
    assert_eq!(resumed["session"]["id"], new_id);
    app.cleanup().await;
}
