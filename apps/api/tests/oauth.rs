mod support;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::redirect::Policy;
use reqwest::{Client, StatusCode};
use rmcp::ServiceExt;
use rmcp::model::ClientConfig;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use support::{TestApp, TestUser};
use url::Url;
use url::form_urlencoded::Serializer;

const REDIRECT: &str = "http://127.0.0.1:33418/";
const VERIFIER: &str = "a-sufficiently-long-pkce-code-verifier-0123456789";

struct Flow {
    origin: String,
    http: Client,
    client_id: String,
}

impl Flow {
    fn authorize_query(&self, extra: &[(&str, &str)]) -> String {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
        let mut query = Serializer::new(String::new());
        query.extend_pairs([
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            // Loopback clients pick a free port at sign-in time.
            ("redirect_uri", "http://127.0.0.1:50123/"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", "xyz"),
            ("scope", "mcp"),
        ]);
        let resource = format!("{}/mcp", self.origin);
        query.append_pair("resource", &resource);
        query.extend_pairs(extra);
        query.finish()
    }

    async fn code(&self, user: &TestUser) -> String {
        let redirect = approve(user, &self.authorize_query(&[]), true).await;
        let url = Url::parse(&redirect).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params["state"], "xyz");
        assert!(
            redirect.starts_with("http://127.0.0.1:50123/?"),
            "{redirect}"
        );
        params["code"].clone()
    }

    async fn token(&self, form: &[(&str, &str)]) -> (StatusCode, Value) {
        let mut body = Serializer::new(String::new());
        body.append_pair("client_id", &self.client_id);
        body.extend_pairs(form);
        let response = self
            .http
            .post(format!("{}/oauth/token", self.origin))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body.finish())
            .send()
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        (response.status(), response.json().await.unwrap())
    }

    async fn exchange(&self, code: &str, verifier: &str) -> (StatusCode, Value) {
        self.token(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", "http://127.0.0.1:50123/"),
            ("code_verifier", verifier),
        ])
        .await
    }

    async fn mcp_status(&self, access_token: &str) -> StatusCode {
        self.http
            .post(format!("{}/mcp", self.origin))
            .bearer_auth(access_token)
            .header("accept", "application/json, text/event-stream")
            .json(
                &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "t", "version": "1" } } }),
            )
            .send()
            .await
            .unwrap()
            .status()
    }
}

async fn approve(user: &TestUser, query: &str, approve: bool) -> String {
    let params: Value = Url::parse(&format!("http://x/?{query}"))
        .unwrap()
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), Value::String(value.into_owned())))
        .collect::<serde_json::Map<_, _>>()
        .into();
    let decision = user
        .post_ok(
            "/api/v1/oauth/authorization",
            json!({ "params": params, "approve": approve }),
        )
        .await;
    decision["redirect_to"].as_str().unwrap().to_owned()
}

/// Walks the discovery chain an MCP client follows after a 401.
async fn discover_and_register(app: &TestApp) -> Flow {
    let origin = format!("http://localhost:{}", app.address.port());
    let http = Client::builder().redirect(Policy::none()).build().unwrap();

    let challenge = http
        .post(format!("{origin}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(challenge.status(), 401);
    let header = challenge.headers()["www-authenticate"].to_str().unwrap();
    let metadata_url = format!("{origin}/.well-known/oauth-protected-resource/mcp");
    assert!(
        header.contains(&format!("resource_metadata=\"{metadata_url}\"")),
        "{header}"
    );
    assert!(
        !header.contains("error="),
        "no error without a token: {header}"
    );

    let resource: Value = http
        .get(&metadata_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resource["resource"], format!("{origin}/mcp"));
    let issuer = resource["authorization_servers"][0].as_str().unwrap();
    let server: Value = http
        .get(format!("{issuer}/.well-known/oauth-authorization-server"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(server["issuer"], issuer);
    assert_eq!(server["code_challenge_methods_supported"], json!(["S256"]));

    let rejected = http
        .post(server["registration_endpoint"].as_str().unwrap())
        .json(&json!({ "redirect_uris": ["http://evil.example/callback"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 400);
    let registered = http
        .post(server["registration_endpoint"].as_str().unwrap())
        .json(&json!({
            "client_name": "Visual Studio Code",
            "redirect_uris": [REDIRECT, "https://vscode.dev/redirect"],
            "grant_types": ["authorization_code", "refresh_token"],
            "token_endpoint_auth_method": "none"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(registered.status(), 201);
    let registered: Value = registered.json().await.unwrap();
    assert_eq!(registered["token_endpoint_auth_method"], "none");
    Flow {
        client_id: registered["client_id"].as_str().unwrap().to_owned(),
        origin,
        http,
    }
}

#[tokio::test]
async fn mcp_clients_sign_in_with_oauth() {
    let app = TestApp::spawn().await;
    let flow = discover_and_register(&app).await;
    let ada = app.user("ada").await;

    // The authorize endpoint hands valid requests to the web app's consent page.
    let query = flow.authorize_query(&[]);
    let handoff = flow
        .http
        .get(format!("{}/oauth/authorize?{query}", flow.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(handoff.status(), 303);
    assert_eq!(
        handoff.headers()["location"],
        format!("http://localhost:5173/connect/authorize?{query}").as_str()
    );

    let preview_path = format!("/api/v1/oauth/authorization?{query}");
    let anonymous = flow.http.get(app.url(&preview_path)).send().await.unwrap();
    assert_eq!(anonymous.status(), 401);
    let preview = ada.json(&preview_path).await;
    assert_eq!(preview["client_name"], "Visual Studio Code");
    assert_eq!(preview["redirect_target"], "127.0.0.1");

    let denied = approve(&ada, &query, false).await;
    assert!(denied.contains("error=access_denied") && denied.contains("state=xyz"));

    // A wrong verifier burns the code.
    let code = flow.code(&ada).await;
    let (status, body) = flow.exchange(&code, &"b".repeat(43)).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_grant"))
    );
    let (status, _) = flow.exchange(&code, VERIFIER).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let code = flow.code(&ada).await;
    let (status, tokens) = flow.exchange(&code, VERIFIER).await;
    assert_eq!(status, StatusCode::OK, "{tokens}");
    assert_eq!(tokens["token_type"], "Bearer");
    assert_eq!(tokens["expires_in"], 3600);
    let access = tokens["access_token"].as_str().unwrap();

    let config = StreamableHttpClientTransportConfig::with_uri(format!("{}/mcp", flow.origin))
        .auth_header(access.to_owned());
    let client = ClientConfig::default()
        .serve(StreamableHttpClientTransport::from_config(config))
        .await
        .expect("MCP handshake with an OAuth token");
    assert!(!client.list_all_tools().await.unwrap().is_empty());
    let _ = client.cancel().await;

    let listed = ada.json("/api/v1/integrations").await;
    let connection = &listed["connections"][0];
    assert_eq!(connection["auth_method"], "oauth");
    assert_eq!(connection["label"], "Visual Studio Code");
    assert_eq!(
        listed["mcp_config"]["servers"]["learning-platform"]["type"],
        "http"
    );

    // Refresh tokens rotate: the old pair stops working.
    let refresh = tokens["refresh_token"].as_str().unwrap();
    let (status, rotated) = flow
        .token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)])
        .await;
    assert_eq!(status, StatusCode::OK, "{rotated}");
    let (status, _) = flow
        .token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)])
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let new_access = rotated["access_token"].as_str().unwrap();
    assert_eq!(flow.mcp_status(access).await, StatusCode::UNAUTHORIZED);
    assert_ne!(flow.mcp_status(new_access).await, StatusCode::UNAUTHORIZED);

    // Replaying a used code revokes the grant it produced.
    let (status, _) = flow.exchange(&code, VERIFIER).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(flow.mcp_status(new_access).await, StatusCode::UNAUTHORIZED);

    app.cleanup().await;
}

#[tokio::test]
async fn authorization_requests_are_validated_before_redirecting() {
    let app = TestApp::spawn().await;
    let flow = discover_and_register(&app).await;
    let get = |query: String| {
        let request = flow
            .http
            .get(format!("{}/oauth/authorize?{query}", flow.origin));
        async move { request.send().await.unwrap() }
    };

    // An unregistered redirect URI must never receive the browser.
    let foreign = flow
        .authorize_query(&[])
        .replace("127.0.0.1%3A50123", "evil.example");
    let response = get(foreign).await;
    assert_eq!(response.status(), 400);
    assert!(response.headers().get("location").is_none());

    let unknown = get(flow.authorize_query(&[]).replace(&flow.client_id, "nope")).await;
    assert_eq!(unknown.status(), 400);

    // Other problems go back to the client with its state.
    let mut without_pkce = Serializer::new(String::new());
    without_pkce.extend_pairs([
        ("response_type", "code"),
        ("client_id", flow.client_id.as_str()),
        ("redirect_uri", REDIRECT),
        ("state", "s1"),
    ]);
    let response = get(without_pkce.finish()).await;
    assert_eq!(response.status(), 303);
    let location = response.headers()["location"].to_str().unwrap();
    assert!(location.starts_with(REDIRECT), "{location}");
    assert!(location.contains("error=invalid_request") && location.contains("state=s1"));

    let response = get(flow
        .authorize_query(&[])
        .replace("scope=mcp", "scope=admin"))
    .await;
    let location = response.headers()["location"].to_str().unwrap();
    assert!(location.contains("error=invalid_scope"), "{location}");

    let (status, body) = flow
        .token(&[("grant_type", "password"), ("username", "ada")])
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "unsupported_grant_type");

    app.cleanup().await;
}
