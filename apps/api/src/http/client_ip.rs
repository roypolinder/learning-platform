//! Client address for rate limiting. Behind a reverse proxy every connection comes from the
//! proxy, so the forwarded address is used instead, but only when the deployment opts in and
//! the connection really comes from loopback, where nobody else can forge the header.

use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderMap;

use crate::config::AppConfig;

pub fn client_ip(config: &AppConfig, peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    if config.trust_proxy_headers && peer.ip().is_loopback() {
        // The proxy appends the address it saw, so the last entry is the trustworthy one.
        let forwarded = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit(',').next())
            .and_then(|ip| ip.trim().parse().ok());
        if let Some(ip) = forwarded {
            return ip;
        }
    }
    peer.ip()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(trust: bool) -> AppConfig {
        let mut config = AppConfig::for_tests();
        config.trust_proxy_headers = trust;
        config
    }

    #[test]
    fn uses_forwarded_address_only_from_a_trusted_local_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1, 203.0.113.7".parse().unwrap());
        let local: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let remote: SocketAddr = "198.51.100.2:5000".parse().unwrap();
        assert_eq!(
            client_ip(&config(true), local, &headers).to_string(),
            "203.0.113.7"
        );
        assert_eq!(client_ip(&config(false), local, &headers), local.ip());
        assert_eq!(client_ip(&config(true), remote, &headers), remote.ip());
    }
}
