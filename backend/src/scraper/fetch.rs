//! HTTP fetching with SSRF guard, size cap and block detection.

use crate::config::ScraperConfig;
use anyhow::{Result, anyhow, bail};
use reqwest::{Client, StatusCode, header, redirect};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

pub struct Fetched {
    pub final_url: Url,
    pub status: StatusCode,
    pub body: String,
}

pub fn build_client(cfg: &ScraperConfig) -> Result<Client> {
    let allow_private = cfg.allow_private_hosts;
    Ok(Client::builder()
        .user_agent(&cfg.user_agent)
        .timeout(cfg.timeout)
        .redirect(redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else if !allow_private && is_private_literal(attempt.url()) {
                attempt.error("redirect to a private address")
            } else {
                attempt.follow()
            }
        }))
        .default_headers({
            let mut h = header::HeaderMap::new();
            h.insert(
                header::ACCEPT,
                "text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8".parse()?,
            );
            h.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse()?);
            h
        })
        .build()?)
}

/// Outcome of the pre-fetch safety check.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// Safe to fetch.
    Ok,
    /// DNS lookup failed; skip the fetch and use URL-only hints.
    Unresolvable(String),
}

/// Rejects non-http(s) URLs and hosts that resolve to private addresses,
/// so the parser cannot be used to probe the local network.
pub async fn validate_target(url: &Url, allow_private: bool) -> Result<Target> {
    if !matches!(url.scheme(), "http" | "https") {
        bail!("only http and https URLs are supported");
    }
    let host = url.host_str().ok_or_else(|| anyhow!("URL has no host"))?;
    if allow_private {
        return Ok(Target::Ok);
    }
    if is_private_literal(url)
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".localhost")
    {
        bail!("refusing to fetch a local address");
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = match tokio::net::lookup_host((host.trim_matches(['[', ']']), port)).await {
        Ok(a) => a,
        Err(e) => {
            return Ok(Target::Unresolvable(format!(
                "could not resolve {host}: {e}"
            )));
        }
    };
    for addr in addrs {
        if is_private_ip(addr.ip()) {
            bail!("refusing to fetch a private address");
        }
    }
    Ok(Target::Ok)
}

fn is_private_literal(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(ip)) => is_private_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => is_private_ip(IpAddr::V6(ip)),
        Some(Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        None => true,
    }
}

pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private_v4(v4);
            }
            let seg0 = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg0 & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (seg0 & 0xffc0) == 0xfe80 // link local fe80::/10
                || v6 == Ipv6Addr::LOCALHOST
        }
    }
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || (a == 100 && (64..128).contains(&b)) // CGNAT 100.64.0.0/10
        || a == 0
}

pub async fn get(client: &Client, url: &Url, max_bytes: usize) -> Result<Fetched> {
    let mut resp = client.get(url.clone()).send().await?;
    let status = resp.status();
    let final_url = resp.url().clone();
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        buf.extend_from_slice(&chunk);
        if buf.len() > max_bytes {
            buf.truncate(max_bytes);
            break;
        }
    }
    Ok(Fetched {
        final_url,
        status,
        body: String::from_utf8_lossy(&buf).into_owned(),
    })
}

/// Bot walls that still return 200.
pub fn looks_blocked(status: StatusCode, body: &str) -> bool {
    if matches!(status.as_u16(), 401 | 403 | 429 | 503) {
        return true;
    }
    let head = &body[..super::heuristics::floor_char_boundary(body, body.len().min(20_000))];
    let lower = head.to_ascii_lowercase();
    [
        "validatecaptcha",
        "robot check",
        "are you a human",
        "cf-challenge",
        "challenge-platform",
        "access denied",
        "enable javascript and cookies to continue",
    ]
    .iter()
    .any(|m| lower.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_ranges() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.10",
            "172.16.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "::ffff:10.0.0.1",
        ] {
            assert!(is_private_ip(ip.parse().unwrap()), "{ip} should be private");
        }
        for ip in ["8.8.8.8", "151.101.1.1", "2606:4700::1111"] {
            assert!(!is_private_ip(ip.parse().unwrap()), "{ip} should be public");
        }
    }

    #[tokio::test]
    async fn rejects_bad_targets() {
        let u = |s: &str| Url::parse(s).unwrap();
        assert!(
            validate_target(&u("file:///etc/passwd"), false)
                .await
                .is_err()
        );
        assert!(
            validate_target(&u("http://localhost:5432"), false)
                .await
                .is_err()
        );
        assert!(
            validate_target(&u("http://127.0.0.1/"), false)
                .await
                .is_err()
        );
        assert!(validate_target(&u("http://[::1]/"), false).await.is_err());
        assert_eq!(
            validate_target(&u("http://127.0.0.1/"), true)
                .await
                .unwrap(),
            Target::Ok
        );
        assert!(matches!(
            validate_target(&u("https://no-such-host.invalid/"), false)
                .await
                .unwrap(),
            Target::Unresolvable(_)
        ));
    }

    #[test]
    fn block_detection() {
        assert!(looks_blocked(StatusCode::FORBIDDEN, ""));
        assert!(looks_blocked(StatusCode::OK, "<title>Robot Check</title>"));
        assert!(!looks_blocked(StatusCode::OK, "<title>PLA Basic</title>"));
    }
}
