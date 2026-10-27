//! `{{ namespace.key }}` references inside manifest strings.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    /// Platform-level user inputs (`cockpit.yaml` `spec.inputs`).
    User,
    /// Instance inputs (widget `spec.inputs`, set per placement).
    Inputs,
    /// Declared secrets, resolved from the OS keyring by the backend.
    Secrets,
    /// Environment variables (reserved; needs `spec.environments`).
    Env,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRef {
    pub namespace: Namespace,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("unclosed `{{{{` in template")]
    Unclosed,
    #[error("`{{{{ {0} }}}}` is not of the form `{{{{ namespace.key }}}}`")]
    Malformed(String),
    #[error("unknown template namespace `{0}` (expected user, inputs or secrets)")]
    UnknownNamespace(String),
}

pub fn refs(s: &str) -> Result<Vec<TemplateRef>, TemplateError> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or(TemplateError::Unclosed)?;
        let expr = after[..end].trim();
        let (ns, key) = expr
            .split_once('.')
            .filter(|(_, key)| is_key(key))
            .ok_or_else(|| TemplateError::Malformed(expr.into()))?;
        let namespace = match ns {
            "user" => Namespace::User,
            "inputs" => Namespace::Inputs,
            "secrets" => Namespace::Secrets,
            "env" => Namespace::Env,
            other => return Err(TemplateError::UnknownNamespace(other.into())),
        };
        out.push(TemplateRef {
            namespace,
            key: key.into(),
        });
        rest = &after[end + 2..];
    }
    Ok(out)
}

/// Replaces every complete `{{ … }}` with `replacement`; used to syntax-check templated URLs.
pub(crate) fn substitute(s: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(replacement);
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}

fn is_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(namespace: Namespace, key: &str) -> TemplateRef {
        TemplateRef {
            namespace,
            key: key.into(),
        }
    }

    #[test]
    fn extracts_refs() {
        assert_eq!(refs("plain"), Ok(vec![]));
        assert_eq!(
            refs("https://h/{{ inputs.project }}/x?t={{secrets.TOKEN}}"),
            Ok(vec![
                r(Namespace::Inputs, "project"),
                r(Namespace::Secrets, "TOKEN")
            ])
        );
    }

    #[test]
    fn rejects_bad_templates() {
        assert_eq!(refs("{{ user.team"), Err(TemplateError::Unclosed));
        assert_eq!(
            refs("{{ team }}"),
            Err(TemplateError::Malformed("team".into()))
        );
        assert_eq!(
            refs("{{ user.a.b }}"),
            Err(TemplateError::Malformed("user.a.b".into()))
        );
        assert_eq!(
            refs("{{ foo.bar }}"),
            Err(TemplateError::UnknownNamespace("foo".into()))
        );
    }

    #[test]
    fn substitutes() {
        assert_eq!(substitute("a/{{ x.y }}/b/{{z.w}}", "_"), "a/_/b/_");
        assert_eq!(substitute("a/{{ open", "_"), "a/{{ open");
    }
}
