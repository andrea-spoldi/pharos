//! Http sources: one request per run, only to an origin the user approved, never following
//! redirects. Checked on every run, not just at consent time. Error messages never contain
//! secret values, even when the server or the transport echoes them back.

use crate::run::{MAX_ROWS, RowsError, WidgetData, select_rows};
use pharos_manifest::{HttpMethod, http_origin};
use std::collections::BTreeSet;
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Largest response body read; anything bigger fails the run.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;
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
    #[error(transparent)]
    Rows(#[from] RowsError),
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
    /// The manifest's `rows` selector.
    pub rows: Option<&'a str>,
    /// The view is markdown, so a `text/markdown` or `text/plain` body is the content. Set from
    /// the stored manifest's view type, never by the frontend.
    pub text_ok: bool,
}

/// Sends `request` if its origin is in `approved` and parses the JSON body into rows. With
/// `text_ok`, a `text/markdown` or `text/plain` body becomes one `{ value }` row instead.
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
        HttpMethod::Get => {
            with_headers(agent.get(request.url), request.headers, request.text_ok).call()
        }
        HttpMethod::Post => {
            with_headers(agent.post(request.url), request.headers, request.text_ok).send_empty()
        }
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
    let media = media_type(&response);
    if request.text_ok && is_text(media.as_deref()) {
        if let Some(path) = request.rows {
            return Err(not_json(path));
        }
        let text = String::from_utf8_lossy(&body).into_owned();
        return Ok(WidgetData {
            rows: vec![serde_json::json!({ "value": text })],
            truncated: false,
        });
    }
    let value: serde_json::Value = match (serde_json::from_slice(&body), request.rows) {
        (Ok(value), _) => value,
        (Err(_), Some(path)) => return Err(not_json(path)),
        (Err(e), None) => {
            return Err(HttpError::Parse {
                origin,
                message: parse_message(request.text_ok, media.as_deref(), &e),
            });
        }
    };
    Ok(select_rows(value, request.rows, "the output", MAX_ROWS)?)
}

fn not_json(path: &str) -> HttpError {
    RowsError::NotJson {
        path: path.into(),
        what: "the response was not JSON".into(),
    }
    .into()
}

/// The response's media type: parameters stripped, lowercased.
fn media_type(response: &ureq::http::Response<ureq::Body>) -> Option<String> {
    response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(|m| m.trim().to_ascii_lowercase())
}

/// The media type is markdown or plain text.
fn is_text(media: Option<&str>) -> bool {
    matches!(media, Some("text/markdown" | "text/plain"))
}

/// A body that failed to parse as JSON; when text was acceptable and the response is not
/// declared JSON, say what was expected instead of surfacing the JSON error.
fn parse_message(text_ok: bool, media: Option<&str>, e: &serde_json::Error) -> String {
    if text_ok && !media.is_some_and(|m| m.contains("json")) {
        format!(
            "expected text/markdown, text/plain or JSON, got {}",
            media.unwrap_or("no content type")
        )
    } else {
        e.to_string()
    }
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
    text_ok: bool,
) -> ureq::RequestBuilder<B> {
    if !headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("accept"))
    {
        builder = builder.header(
            "Accept",
            if text_ok {
                "text/markdown, text/plain;q=0.9, application/json;q=0.5"
            } else {
                "application/json"
            },
        );
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
    fn unexpected_content_type_is_explained() {
        let e = serde_json::from_str::<serde_json::Value>("<html>").unwrap_err();
        assert_eq!(
            parse_message(true, Some("text/html"), &e),
            "expected text/markdown, text/plain or JSON, got text/html"
        );
        assert_eq!(
            parse_message(true, None, &e),
            "expected text/markdown, text/plain or JSON, got no content type"
        );
        assert_eq!(
            parse_message(true, Some("application/json"), &e),
            e.to_string()
        );
        assert_eq!(parse_message(false, Some("text/html"), &e), e.to_string());
    }

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
            rows: None,
            text_ok: false,
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
            rows: None,
            text_ok: false,
        };
        let approved = BTreeSet::from(["https://gitlab.com".to_string()]);
        assert_eq!(
            fetch(&request, &approved, &[]),
            Err(HttpError::InvalidHeader("X-Token".into()))
        );
    }
}
