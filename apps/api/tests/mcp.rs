mod support;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, ClientConfig};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use support::{TestApp, TestUser, step_status};

async fn connect(
    app: &TestApp,
    token: &str,
) -> rmcp::service::RunningService<rmcp::RoleClient, ClientConfig> {
    let config = StreamableHttpClientTransportConfig::with_uri(format!(
        "http://localhost:{}/mcp",
        app.address.port()
    ))
    .auth_header(token.to_owned());
    ClientConfig::default()
        .serve(StreamableHttpClientTransport::from_config(config))
        .await
        .expect("MCP handshake")
}

async fn call(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientConfig>,
    tool: &str,
    args: Value,
) -> (bool, Value) {
    let arguments = args.as_object().cloned().unwrap_or_default();
    let result = client
        .call_tool(CallToolRequestParams::new(tool.to_owned()).with_arguments(arguments))
        .await
        .unwrap();
    (
        result.is_error == Some(true),
        result.structured_content.unwrap_or(Value::Null),
    )
}

async fn token(user: &TestUser) -> String {
    let created = user
        .post_ok("/api/v1/integrations/mcp", json!({ "label": "VS Code" }))
        .await;
    created["token"].as_str().unwrap().to_owned()
}

/// Chooses a project and finishes the first course of the track, which unlocks the next.
async fn finish_project_setup(user: &TestUser) {
    user.post_ok(
        "/api/v1/project",
        json!({ "theme_id": "bike-rental", "stack": "javascript" }),
    )
    .await;
    let started = user
        .post_ok("/api/v1/courses/project-setup/sessions", json!({}))
        .await;
    let base = format!(
        "/api/v1/sessions/{}",
        started["session"]["id"].as_str().unwrap()
    );
    assert_eq!(step_status(&started["progress"], "choose"), "locked");
    for step in ["how-it-works", "big-picture"] {
        user.post_ok(&format!("{base}/steps/{step}/complete"), json!({}))
            .await;
    }
    // The project already exists, so the choice step resolves itself.
    let refreshed = user.post_ok(&format!("{base}/refresh"), json!({})).await;
    assert_eq!(step_status(&refreshed, "choose"), "completed");
    for step in ["your-choice", "tools", "create-project", "project-ready"] {
        user.post_ok(&format!("{base}/steps/{step}/complete"), json!({}))
            .await;
    }
    let quiz = user
        .post_ok(
            &format!("{base}/steps/check/submissions"),
            json!({ "payload": { "answers": { "checks": ["browser"], "env": ["secrets"], "seed": ["checks"] } } }),
        )
        .await;
    assert_eq!(quiz["progress"]["status"], "completed", "{quiz}");
}

#[tokio::test]
async fn agent_connects_completes_steps_and_stays_isolated() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    finish_project_setup(&ada).await;
    let started = ada
        .post_ok("/api/v1/courses/mcp-first-connection/sessions", json!({}))
        .await;
    let session_id = started["session"]["id"].as_str().unwrap().to_owned();
    let base = format!("/api/v1/sessions/{session_id}");
    for step in ["what-is-mcp", "roles", "primitives"] {
        ada.post_ok(&format!("{base}/steps/{step}/complete"), json!({}))
            .await;
    }
    ada.post_ok(
        &format!("{base}/steps/place-roles/submissions"),
        json!({ "payload": { "slots": { "host": "host", "client": "client", "server": "server" } } }),
    )
    .await;
    ada.post_ok(
        &format!("{base}/steps/check-basics/submissions"),
        json!({ "payload": { "answers": { "transport": ["http"], "control": ["tools"], "authorization": ["challenge"] } } }),
    )
    .await;
    ada.post_ok(&format!("{base}/steps/config-example/complete"), json!({}))
        .await;
    ada.post_ok(
        &format!("{base}/steps/config-blanks/submissions"),
        json!({ "payload": { "blanks": { "root": "mcpServers", "transport": "http", "path": "mcp" } } }),
    )
    .await;

    let ada_token = token(&ada).await;
    let client = connect(&app, &ada_token).await;
    let tools = client.list_all_tools().await.unwrap();
    let names: Vec<_> = tools.iter().map(|tool| tool.name.to_string()).collect();
    for expected in [
        "get_course_context",
        "get_current_step",
        "report_task_result",
        "get_session_status",
        "get_project",
    ] {
        assert!(names.contains(&expected.to_owned()), "{names:?}");
    }

    let (is_error, project) = call(&client, "get_project", json!({})).await;
    assert!(!is_error, "{project}");
    assert_eq!(project["project"]["theme"]["id"], "bike-rental");
    assert_eq!(project["seed_data"]["items"].as_array().unwrap().len(), 12);

    // A different tool does not satisfy the connection step...
    let (_, status) = call(&client, "get_session_status", json!({})).await;
    assert_eq!(status["current_step_id"], "connect-agent");
    // ...calling get_current_step does, and the response already shows the next step.
    let (is_error, current) = call(&client, "get_current_step", json!({})).await;
    assert!(!is_error, "{current}");
    assert_eq!(current["step"]["id"], "fetch-project");
    assert!(current["step"]["submission_schema"].is_object());
    let progress = ada.json(&format!("{base}/progress")).await;
    assert_eq!(step_status(&progress, "connect-agent"), "completed");

    // The file checks are filled in for Ada's theme.
    let items = project["seed_data"]["items"].to_string();
    let pages = project["seed_data"]["pages"].to_string();
    let (_, missing_readme) = call(
        &client,
        "report_task_result",
        json!({
            "step_id": "fetch-project",
            "evidence": { "files": [
                { "path": "data/items.json", "content": items },
                { "path": "data/pages.json", "content": pages },
                { "path": "README.md", "content": "# My site\n\nSomething I am building for a course." }
            ] }
        }),
    )
    .await;
    assert_eq!(missing_readme["outcome"]["status"], "failed");
    let (_, fetched) = call(
        &client,
        "report_task_result",
        json!({
            "step_id": "fetch-project",
            "evidence": { "files": [
                { "path": "data/items.json", "content": items },
                { "path": "data/pages.json", "content": pages },
                { "path": "README.md", "content": "# Velora Bikes\n\nThe bike rental website I build in the track." }
            ] }
        }),
    )
    .await;
    assert_eq!(fetched["outcome"]["status"], "passed", "{fetched}");

    let hardcoded = json!({ "mcpServers": { "lp": { "type": "http", "url": "http://localhost:8080/mcp", "headers": { "Authorization": "Bearer lp_mcp_secret" } } } });
    let (_, rejected) = call(
        &client,
        "report_task_result",
        json!({
            "step_id": "verify-config",
            "evidence": { "files": [ { "path": ".mcp.json", "content": hardcoded.to_string() } ] }
        }),
    )
    .await;
    assert_eq!(rejected["outcome"]["status"], "failed");

    let safe = json!({ "mcpServers": { "lp": { "type": "http", "url": "http://localhost:8080/mcp", "tools": ["*"] } } });
    let (_, accepted) = call(
        &client,
        "report_task_result",
        json!({
            "step_id": "verify-config",
            "evidence": { "files": [ { "path": ".mcp.json", "content": safe.to_string() } ] }
        }),
    )
    .await;
    assert_eq!(accepted["outcome"]["status"], "passed", "{accepted}");
    assert_eq!(accepted["course_progress_percent"], 100);
    let detail = ada.json(&base).await;
    assert_eq!(detail["session"]["status"], "completed");

    // Another user's token cannot reach Ada's session.
    let eve = app.user("eve").await;
    let eve_client = connect(&app, &token(&eve).await).await;
    let (is_error, denied) = call(
        &eve_client,
        "get_session_status",
        json!({ "session_id": session_id }),
    )
    .await;
    assert!(is_error, "{denied}");
    let (is_error, _) = call(&eve_client, "get_current_step", json!({})).await;
    assert!(is_error, "eve has no active session");

    let connections = ada.json("/api/v1/integrations").await;
    assert_eq!(connections["connections"][0]["status"], "connected");

    let _ = client.cancel().await;
    let _ = eve_client.cancel().await;
    app.cleanup().await;
}

#[tokio::test]
async fn rejects_missing_invalid_and_revoked_tokens() {
    let app = TestApp::spawn().await;
    let http = reqwest::Client::new();
    let url = format!("http://localhost:{}/mcp", app.address.port());
    let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "t", "version": "1" } } });

    let anonymous = http
        .post(&url)
        .header("accept", "application/json, text/event-stream")
        .json(&init)
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), 401);
    let forged = http
        .post(&url)
        .bearer_auth("lp_mcp_forged")
        .header("accept", "application/json, text/event-stream")
        .json(&init)
        .send()
        .await
        .unwrap();
    assert_eq!(forged.status(), 401);

    let ada = app.user("ada").await;
    let created = ada
        .post_ok("/api/v1/integrations/mcp", json!({ "label": "Laptop" }))
        .await;
    let token = created["token"].as_str().unwrap();
    let id = created["connection"]["id"].as_str().unwrap();
    let revoke = ada
        .client
        .delete(app.url(&format!("/api/v1/integrations/{id}")))
        .header("cookie", &ada.cookie)
        .header("x-requested-with", "lp-web")
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status(), 204);
    let revoked = http
        .post(&url)
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .json(&init)
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status(), 401);

    // Browser origins are rejected even with a valid token (DNS rebinding / drive-by).
    let other = ada
        .post_ok("/api/v1/integrations/mcp", json!({ "label": "Desktop" }))
        .await;
    let from_browser = http
        .post(&url)
        .bearer_auth(other["token"].as_str().unwrap())
        .header("origin", "https://evil.example")
        .header("accept", "application/json, text/event-stream")
        .json(&init)
        .send()
        .await
        .unwrap();
    assert!(
        from_browser.status().is_client_error(),
        "{}",
        from_browser.status()
    );
    app.cleanup().await;
}

#[tokio::test]
async fn super_admin_agents_author_courses() {
    let app = TestApp::spawn().await;
    let ada = app.user("ada").await;
    let root = app.user("root").await;
    app.promote("root").await;

    let learner = connect(&app, &token(&ada).await).await;
    let names: Vec<String> = learner
        .list_all_tools()
        .await
        .unwrap()
        .iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert!(
        !names.iter().any(|name| name.starts_with("authoring_")),
        "{names:?}"
    );
    let (is_error, _) = call(&learner, "authoring_list_courses", json!({})).await;
    assert!(is_error, "learners cannot author");

    let admin = connect(&app, &token(&root).await).await;
    let (is_error, guide) = call(&admin, "authoring_get_guide", json!({})).await;
    assert!(!is_error, "{guide}");
    assert!(guide["step_types"].as_array().unwrap().len() > 5);

    let source = "id: agent-made\nversion: 1.0.0\ntitle: Agent made\nsummary: s\ndescription: d\ncategory: Testing\ndifficulty: beginner\nestimated_minutes: 3\nsections:\n  - id: a\n    title: A\n    steps:\n      - id: intro\n        type: explanation\n        title: Intro\n        body: Hello\n";
    let (_, report) = call(
        &admin,
        "authoring_validate_course",
        json!({ "source": source.replace("type: explanation", "type: nope") }),
    )
    .await;
    assert_eq!(report["valid"], false, "{report}");
    let (is_error, published) = call(
        &admin,
        "authoring_publish_course",
        json!({ "source": source }),
    )
    .await;
    assert!(!is_error, "{published}");
    assert_eq!(published["status"], "created");
    let listed = ada.json("/api/v1/courses?q=agent%20made").await;
    assert_eq!(listed["items"][0]["id"], "agent-made");

    let _ = learner.cancel().await;
    let _ = admin.cancel().await;
    app.cleanup().await;
}
