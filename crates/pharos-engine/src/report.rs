//! A pre-filled report a user can send the platform team about a blocked widget.

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use pharos_manifest::{Capabilities, Support};
use serde::Serialize;

/// What to send the platform team, and where; the link is `None` without `spec.support`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Report {
    pub title: String,
    pub body: String,
    pub link: Option<String>,
}

/// Title and body for widget `widget` blocked at `commit` (first 7 chars used).
pub(crate) fn compose(widget: &str, commit: &str, pending: &Capabilities) -> (String, String) {
    let title = format!("Pharos: widget \"{widget}\" asks for new permissions");
    let commit: String = commit.chars().take(7).collect();
    let mut body = format!(
        "Pharos blocked the widget `{widget}` at commit {commit}: it now asks for permissions I haven't approved.\n\nNew permissions:\n"
    );
    for host in &pending.hosts {
        body.push_str(&format!("- Call {host}\n"));
    }
    for exe in &pending.executables {
        body.push_str(&format!("- Run {exe}\n"));
    }
    for path in &pending.paths {
        body.push_str(&format!("- Read {path}\n"));
    }
    body.push_str("\nWhy I haven't approved it:\n");
    (title, body)
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// The URL that opens a new issue or email with `title` and `body` filled in.
pub(crate) fn link(support: &Support, title: &str, body: &str) -> String {
    let (t, b) = (enc(title), enc(body));
    match support {
        Support::Github { repo } => {
            format!(
                "{}/issues/new?title={t}&body={b}",
                repo.trim_end_matches('/')
            )
        }
        Support::Gitlab { repo } => format!(
            "{}/-/issues/new?issue%5Btitle%5D={t}&issue%5Bdescription%5D={b}",
            repo.trim_end_matches('/')
        ),
        Support::Email { address } => format!("mailto:{address}?subject={t}&body={b}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> Capabilities {
        Capabilities {
            hosts: ["https://gitlab.com".to_string()].into(),
            executables: ["kubectl".to_string()].into(),
            paths: ["catalog/services".to_string()].into(),
        }
    }

    #[test]
    fn compose_lists_pending_capabilities_in_order() {
        let (title, body) = compose("svc", "0123456789", &caps());
        assert_eq!(title, "Pharos: widget \"svc\" asks for new permissions");
        assert_eq!(
            body,
            "Pharos blocked the widget `svc` at commit 0123456: it now asks for permissions I haven't approved.\n\n\
             New permissions:\n- Call https://gitlab.com\n- Run kubectl\n- Read catalog/services\n\n\
             Why I haven't approved it:\n"
        );
    }

    #[test]
    fn links_percent_encode_every_value() {
        let gh = Support::Github {
            repo: "https://github.com/acme/platform".into(),
        };
        assert!(link(&gh, "&#? é", "b").contains("%26%23%3F%20%C3%A9"));
        let mail = Support::Email {
            address: "p@acme.dev".into(),
        };
        let l = link(&mail, "a b", "c d");
        assert!(l.starts_with("mailto:p@acme.dev?subject=a%20b&body=c%20d"));
        assert!(!l.contains('+'));
        let gl = Support::Gitlab {
            repo: "https://gitlab.com/acme/platform".into(),
        };
        let l = link(&gl, "t", "b");
        assert!(l.contains("/-/issues/new?issue%5Btitle%5D=t&issue%5Bdescription%5D=b"));
    }

    #[test]
    fn trailing_slash_in_repo_is_tolerated() {
        let gh = Support::Github {
            repo: "https://github.com/acme/platform/".into(),
        };
        assert!(link(&gh, "t", "b").starts_with("https://github.com/acme/platform/issues/new?"));
    }
}
