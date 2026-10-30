//! Http sources: one request per run, only to an origin the user approved, never following
//! redirects. Checked on every run, not just at consent time. Error messages never contain
//! secret values, even when the server or the transport echoes them back.

use crate::run::{WidgetData, rows};
use pharos_manifest::{HttpMethod, http_origin};
use std::collections::BTreeSet;
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Largest response body read; anything bigger fails the run.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Most rows one response yields; the rest are dropped and `truncated` is set.
pub const MAX_ROWS: usize = 500;
/// How much of an error response body a message quotes.
const BODY_EXCERPT: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("`{0}` is not an approved host")]
    NotApproved(String),
    #[error("header `{0}` contains a line break")]
    InvalidHeader(String),
    #[error("{origin} redirected to {location}; redirects are not followed")]
    Redirected { origin: String, location: String },
    #[error("{origin} answered {status}{}", excerpt(.body))]
    Status {
        origin: String,
        status: u16,
        body: String,
    },
    #[error("request to {origin} failed: {message}")]
    Request { origin: String, message: String },
    #[error("{origin} returned more than {} MB", MAX_BYTES / 1024 / 1024)]
    TooLarge { origin: String },
    #[error("{origin} did not return JSON: {message}")]
    Parse { origin: String, message: String },
}

fn excerpt(body: &str) -> String {
    if body.is_empty() {
        String::new()
    } else {
        format!(": {body}")
    }
}

impl HttpError {
    /// The server refused the credentials (401) or the access (403).
    pub fn is_auth(&self) -> bool {
        matches!(
            self,
            Self::Status {
                status: 401 | 403,
                ..
            }
        )
    }
}

/// A rendered request: templates already resolved, values already escaped.
pub(crate) struct Request<'a> {
    pub method: HttpMethod,
    pub url: &'a str,
    pub headers: &'a [(String, String)],
}

/// Sends `request` if its origin is in `approved` and parses the JSON body into rows.
/// `secrets` are the resolved values, redacted from every error message.
pub(crate) fn fetch(
    request: &Request,
    approved: &BTreeSet<String>,
    secrets: &[String],
) -> Result<WidgetData, HttpError> {
    let redact = |s: &str| redact(s, secrets);
    let origin = http_origin(request.url).map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
    if !approved.contains(&origin) {
        return Err(HttpError::NotApproved(origin));
    }
    for (name, value) in request.headers {
        if value.contains(['\r', '\n']) {
            return Err(HttpError::InvalidHeader(name.clone()));
        }
    }

    let failed = |e: ureq::Error| match e {
        ureq::Error::BodyExceedsLimit(_) => HttpError::TooLarge {
            origin: origin.clone(),
        },
        e => HttpError::Request {
            origin: origin.clone(),
            message: redact(&e.to_string()),
        },
    };
    let agent = agent();
    let mut response = match request.method {
        HttpMethod::Get => with_headers(agent.get(request.url), request.headers).call(),
        HttpMethod::Post => with_headers(agent.post(request.url), request.headers).send_empty(),
    }
    .map_err(failed)?;

    let status = response.status();
    if status.is_redirection() {
        let location = response
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .unwrap_or("another location");
        return Err(HttpError::Redirected {
            location: redact(location),
            origin,
        });
    }
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(failed)?;
    if !status.is_success() {
        let text = String::from_utf8_lossy(&body);
        let text = text.trim();
        let cut = text
            .char_indices()
            .nth(BODY_EXCERPT)
            .map_or(text.len(), |(i, _)| i);
        return Err(HttpError::Status {
            origin,
            status: status.as_u16(),
            body: redact(&text[..cut]),
        });
    }
    let value: serde_json::Value = serde_json::from_slice(&body).map_err(|e| HttpError::Parse {
        origin,
        message: e.to_string(),
    })?;
    Ok(rows(value, MAX_ROWS))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        // A redirect could leave the approved origin; it is reported, not followed.
        .max_redirects(0)
        .http_status_as_error(false)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent(concat!("pharos/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn with_headers<B>(
    mut builder: ureq::RequestBuilder<B>,
    headers: &[(String, String)],
) -> ureq::RequestBuilder<B> {
    if !headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("accept"))
    {
        builder = builder.header("Accept", "application/json");
    }
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder
}

/// Percent-encodes everything but RFC 3986 unreserved characters, so a value can't add
/// path segments, query parameters or a fragment to the URL it is placed in.
pub(crate) fn encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn redact(s: &str, secrets: &[String]) -> String {
    let mut out = s.to_owned();
    for secret in secrets.iter().filter(|s| !s.is_empty()) {
        out = out.replace(secret.as_str(), "•••");
        let encoded = encode_component(secret);
        if encoded != *secret {
            out = out.replace(&encoded, "•••");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_components() {
        assert_eq!(encode_component("payments-team_1.x~"), "payments-team_1.x~");
        assert_eq!(encode_component("a/b?c=d&e#f"), "a%2Fb%3Fc%3Dd%26e%23f");
        assert_eq!(encode_component("é "), "%C3%A9%20");
    }

    #[test]
    fn redacts_plain_and_encoded_secrets() {
        let secrets = vec!["s3cr/t".to_string(), String::new()];
        assert_eq!(
            redact("token s3cr/t in https://h/?t=s3cr%2Ft", &secrets),
            "token ••• in https://h/?t=•••"
        );
    }

    #[test]
    fn refuses_unapproved_origins_before_sending() {
        let request = Request {
            method: HttpMethod::Get,
            url: "https://elsewhere.example/x",
            headers: &[],
        };
        let approved = BTreeSet::from(["https://gitlab.com".to_string()]);
        assert_eq!(
            fetch(&request, &approved, &[]),
            Err(HttpError::NotApproved("https://elsewhere.example".into()))
        );
    }

    #[test]
    fn refuses_header_line_breaks() {
        let headers = [("X-Token".to_string(), "a\r\nHost: evil".to_string())];
        let request = Request {
            method: HttpMethod::Get,
            url: "https://gitlab.com/x",
            headers: &headers,
        };
        let approved = BTreeSet::from(["https://gitlab.com".to_string()]);
        assert_eq!(
            fetch(&request, &approved, &[]),
            Err(HttpError::InvalidHeader("X-Token".into()))
        );
    }
}
