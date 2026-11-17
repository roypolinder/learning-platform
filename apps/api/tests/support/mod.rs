#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;

use lp_api::app;
use lp_api::config::AppConfig;
use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection};
use tokio_util::sync::CancellationToken;

/// A running API on an ephemeral port backed by a throwaway database.
pub struct TestApp {
    pub base: String,
    pub address: SocketAddr,
    database_url: String,
    admin_url: String,
    database: String,
    shutdown: CancellationToken,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Self::spawn_with_traffic_report(None).await
    }

    pub async fn spawn_with_traffic_report(traffic_report_path: Option<PathBuf>) -> Self {
        let _ = dotenvy::from_path(repo_root().join(".env"));
        let admin_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set (see .env.example)");
        let database = format!("lp_test_{}", uuid::Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&admin_url)
            .await
            .expect("connect to PostgreSQL");
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {database}")))
            .execute(&mut admin)
            .await
            .expect("create test database (role needs CREATEDB)");
        let url = replace_database(&admin_url, &database);
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let config = AppConfig {
            database_url: url.clone(),
            bind_address: address,
            public_base_url: format!("http://localhost:{}", address.port()),
            web_base_url: "http://localhost:5173".into(),
            courses_dir: repo_root().join("courses"),
            web_dist_dir: None,
            cookie_secure: false,
            mutable_course_versions: false,
            trust_proxy_headers: false,
            mcp_allowed_hosts: vec!["localhost".into(), "127.0.0.1".into()],
            database_max_connections: 5,
            super_admin_emails: Vec::new(),
            traffic_report_path,
        };
        let state = app::bootstrap_with_pool(config, pool)
            .await
            .expect("bootstrap");
        let shutdown = CancellationToken::new();
        let router = app::router(state, shutdown.clone());
        let stop = shutdown.clone();
        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move { stop.cancelled().await })
            .await
            .unwrap();
        });
        Self {
            base: format!("http://127.0.0.1:{}", address.port()),
            address,
            database_url: url,
            admin_url,
            database,
            shutdown,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Grants super admin to a user created with [`TestApp::user`].
    pub async fn promote(&self, name: &str) {
        let mut connection = PgConnection::connect(&self.database_url).await.unwrap();
        lp_api::domains::users::repository::promote_super_admins(
            &mut connection,
            &[format!("{name}@example.com")],
        )
        .await
        .unwrap();
    }

    pub async fn user(&self, name: &str) -> TestUser {
        let client = Client::new();
        let response = client
            .post(self.url("/api/v1/auth/register"))
            .header("x-requested-with", "lp-web")
            .json(&json!({ "email": format!("{name}@example.com"), "password": "a long enough password", "display_name": name }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .unwrap()
            .to_owned();
        TestUser {
            client,
            cookie,
            base: self.base.clone(),
        }
    }

    pub async fn cleanup(self) {
        self.shutdown.cancel();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Ok(mut admin) = PgConnection::connect(&self.admin_url).await {
            let _ = sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                self.database
            )))
            .execute(&mut admin)
            .await;
        }
    }
}

pub struct TestUser {
    pub client: Client,
    pub cookie: String,
    base: String,
}

impl TestUser {
    pub async fn get(&self, path: &str) -> Response {
        self.client
            .get(format!("{}{path}", self.base))
            .header("cookie", &self.cookie)
            .send()
            .await
            .unwrap()
    }

    pub async fn post(&self, path: &str, body: Value) -> Response {
        self.client
            .post(format!("{}{path}", self.base))
            .header("cookie", &self.cookie)
            .header("x-requested-with", "lp-web")
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    pub async fn send(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Response {
        let request = self
            .client
            .request(method, format!("{}{path}", self.base))
            .header("cookie", &self.cookie)
            .header("x-requested-with", "lp-web");
        match body {
            Some(body) => request.json(&body),
            None => request,
        }
        .send()
        .await
        .unwrap()
    }

    pub async fn json(&self, path: &str) -> Value {
        let response = self.get(path).await;
        assert!(
            response.status().is_success(),
            "GET {path}: {}",
            response.status()
        );
        response.json().await.unwrap()
    }

    pub async fn post_ok(&self, path: &str, body: Value) -> Value {
        let response = self.post(path, body).await;
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        assert!(status.is_success(), "POST {path}: {status} {body}");
        body
    }
}

pub fn step_status(progress: &Value, step_id: &str) -> String {
    progress["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["step_id"] == step_id)
        .map(|step| step["status"].as_str().unwrap().to_owned())
        .unwrap_or_else(|| panic!("no step {step_id}"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn replace_database(url: &str, database: &str) -> String {
    let (base, query) = url
        .split_once('?')
        .map_or((url, None), |(b, q)| (b, Some(q)));
    let base = base.rsplit_once('/').map_or(base, |(prefix, _)| prefix);
    match query {
        Some(query) => format!("{base}/{database}?{query}"),
        None => format!("{base}/{database}"),
    }
}
