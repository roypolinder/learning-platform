use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_address: SocketAddr,
    /// Base URL agents use to reach the MCP endpoint, shown in setup instructions.
    pub public_base_url: String,
    /// Origin of the web app, which hosts the OAuth consent page.
    pub web_base_url: String,
    pub courses_dir: PathBuf,
    /// Built SvelteKit app, served as a SPA when present.
    pub web_dist_dir: Option<PathBuf>,
    pub cookie_secure: bool,
    /// Lets the same course version be edited in place. Development only.
    pub mutable_course_versions: bool,
    pub mcp_allowed_hosts: Vec<String>,
    pub database_max_connections: u32,
    /// Use `X-Forwarded-For` from a reverse proxy on loopback as the client address.
    pub trust_proxy_headers: bool,
    /// Lowercased emails of existing accounts promoted to super admin at startup.
    pub super_admin_emails: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be set")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = optional("API_BIND_ADDRESS")
            .unwrap_or_else(|| "127.0.0.1:8080".into())
            .parse()
            .map_err(|error: std::net::AddrParseError| ConfigError::Invalid {
                name: "API_BIND_ADDRESS",
                reason: error.to_string(),
            })?;
        let public_base_url = optional("PUBLIC_BASE_URL")
            .unwrap_or_else(|| format!("http://localhost:{}", bind_address_port(&bind_address)));
        let mcp_allowed_hosts = optional("MCP_ALLOWED_HOSTS")
            .map(|hosts| {
                hosts
                    .split(',')
                    .map(str::trim)
                    .filter(|host| !host.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| vec!["localhost".into(), "127.0.0.1".into(), "::1".into()]);
        let public_base_url = public_base_url.trim_end_matches('/').to_owned();
        let web_dist_dir = optional("WEB_DIST_DIR").map(PathBuf::from);
        // Without a built SPA the web app is the Vite dev server.
        let web_base_url = optional("WEB_BASE_URL").unwrap_or_else(|| {
            if web_dist_dir.is_some() {
                public_base_url.clone()
            } else {
                "http://localhost:5173".into()
            }
        });

        Ok(Self {
            database_url: optional("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?,
            bind_address,
            public_base_url,
            web_base_url: web_base_url.trim_end_matches('/').to_owned(),
            courses_dir: optional("COURSES_DIR")
                .unwrap_or_else(|| "courses".into())
                .into(),
            web_dist_dir,
            cookie_secure: flag("COOKIE_SECURE", false)?,
            mutable_course_versions: flag("MUTABLE_COURSE_VERSIONS", false)?,
            trust_proxy_headers: flag("TRUST_PROXY_HEADERS", false)?,
            mcp_allowed_hosts,
            database_max_connections: optional("DATABASE_MAX_CONNECTIONS")
                .map(|value| {
                    value.parse().map_err(|_| ConfigError::Invalid {
                        name: "DATABASE_MAX_CONNECTIONS",
                        reason: "expected a positive integer".into(),
                    })
                })
                .transpose()?
                .unwrap_or(10),
            super_admin_emails: optional("SUPER_ADMIN_EMAILS")
                .map(|emails| {
                    emails
                        .split(',')
                        .map(|email| email.trim().to_lowercase())
                        .filter(|email| !email.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
impl AppConfig {
    pub fn for_tests() -> Self {
        Self {
            database_url: String::new(),
            bind_address: "127.0.0.1:0".parse().unwrap_or_else(|_| unreachable!()),
            public_base_url: "http://localhost".into(),
            web_base_url: "http://localhost".into(),
            courses_dir: "courses".into(),
            web_dist_dir: None,
            cookie_secure: false,
            mutable_course_versions: false,
            trust_proxy_headers: false,
            mcp_allowed_hosts: Vec::new(),
            database_max_connections: 1,
            super_admin_emails: Vec::new(),
        }
    }
}

fn bind_address_port(address: &SocketAddr) -> u16 {
    address.port()
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn flag(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    match optional(name).as_deref() {
        None => Ok(default),
        Some("1" | "true" | "yes") => Ok(true),
        Some("0" | "false" | "no") => Ok(false),
        Some(other) => Err(ConfigError::Invalid {
            name,
            reason: format!("`{other}` is not a boolean"),
        }),
    }
}
