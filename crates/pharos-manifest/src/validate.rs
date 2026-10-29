use crate::capabilities::{bare_origin, http_origin};
use crate::diagnostic::{Diagnostic, Sink};
use crate::duration::parse_duration;
use crate::kinds::*;
use crate::load::{Loaded, Repo};
use crate::paths::confine;
use crate::template::{self, Namespace};
use std::collections::{BTreeMap, HashMap};

/// Which template references a string may use.
struct Scope<'a> {
    /// `None` when pharos.yaml failed to load: user/secret references are not checked.
    platform: Option<&'a Platform>,
    /// `None` outside widgets (dashboards): `inputs.*` is not available.
    inputs: Option<&'a BTreeMap<String, InputDef>>,
    allow_secrets: bool,
}

pub(crate) fn validate(repo: &Repo, out: &mut Vec<Diagnostic>) {
    let platform = repo.platform.as_ref().map(|p| &p.manifest);
    if let Some(p) = &repo.platform {
        validate_platform(repo, &p.manifest, &mut Sink { file: &p.file, out });
    }

    let mut widgets: HashMap<&str, &Loaded<Widget>> = HashMap::new();
    for w in &repo.widgets {
        let mut sink = Sink { file: &w.file, out };
        check_unique(&mut sink, &mut widgets, w, "widget");
        validate_widget(repo, platform, w, &mut sink);
    }

    let mut dashboards: HashMap<&str, &Loaded<Dashboard>> = HashMap::new();
    for d in &repo.dashboards {
        let mut sink = Sink { file: &d.file, out };
        check_unique(&mut sink, &mut dashboards, d, "dashboard");
        validate_dashboard(platform, &widgets, &d.manifest, &mut sink);
    }
}

fn check_unique<'a, S>(
    sink: &mut Sink,
    seen: &mut HashMap<&'a str, &'a Loaded<Manifest<S>>>,
    item: &'a Loaded<Manifest<S>>,
    what: &str,
) {
    let name = item.manifest.metadata.name.as_str();
    if !is_dns_label(name) {
        sink.error(
            "invalid-name",
            "metadata.name",
            format!("`{name}` must be lowercase letters, digits and `-` (max 63)"),
        );
    }
    if let Some(first) = seen.get(name) {
        sink.error(
            "duplicate-name",
            "metadata.name",
            format!("{what} `{name}` is already defined in {}", first.file),
        );
    } else {
        seen.insert(name, item);
    }
}

fn validate_platform(repo: &Repo, platform: &Platform, sink: &mut Sink) {
    for (key, def) in &platform.spec.inputs {
        check_input_def(repo, sink, &format!("spec.inputs.{key}"), key, def);
    }
    let mut seen = Vec::new();
    for (i, secret) in platform.spec.secrets.iter().enumerate() {
        let path = format!("spec.secrets[{i}].name");
        let name = secret.name.as_str();
        let valid = name.starts_with(|c: char| c.is_ascii_uppercase())
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if !valid {
            sink.error(
                "invalid-secret-name",
                &path,
                format!("`{name}` must be UPPER_SNAKE_CASE"),
            );
        }
        if seen.contains(&name) {
            sink.error(
                "duplicate-secret",
                &path,
                format!("secret `{name}` is declared twice"),
            );
        }
        seen.push(name);
    }
}

fn check_input_def(repo: &Repo, sink: &mut Sink, path: &str, key: &str, def: &InputDef) {
    let valid_key = key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid_key {
        sink.error(
            "invalid-input-name",
            path,
            format!("`{key}` must be letters, digits and `_`"),
        );
    }
    match def.kind {
        InputType::Enum => {
            if def.values.is_empty() && def.from.is_none() {
                sink.error(
                    "enum-without-values",
                    path,
                    "enum inputs need `values` or `from` + `field`",
                );
            }
            if let Some(from) = &def.from {
                if def.field.is_none() {
                    sink.error(
                        "missing-field",
                        &format!("{path}.field"),
                        "`from` needs `field`",
                    );
                }
                if let Err(e) = confine(&repo.root, &repo.root, from) {
                    sink.error(
                        "path-escape",
                        &format!("{path}.from"),
                        format!("`{from}`: {e}"),
                    );
                }
            }
            if let Some(default) = &def.default
                && !def.values.is_empty()
                && !def.values.contains(default)
            {
                sink.error(
                    "invalid-default",
                    &format!("{path}.default"),
                    format!("`{default}` is not one of `values`"),
                );
            }
        }
        InputType::String => {
            if !def.values.is_empty() || def.from.is_some() || def.field.is_some() {
                sink.warn(
                    "ignored-field",
                    path,
                    "`values`, `from` and `field` only apply to enum inputs",
                );
            }
        }
    }
}

fn validate_widget(
    repo: &Repo,
    platform: Option<&Platform>,
    widget: &Loaded<Widget>,
    sink: &mut Sink,
) {
    let spec = &widget.manifest.spec;
    for (key, def) in &spec.inputs {
        check_input_def(repo, sink, &format!("spec.inputs.{key}"), key, def);
    }
    let scope = |allow_secrets| Scope {
        platform,
        inputs: Some(&spec.inputs),
        allow_secrets,
    };

    if let Some(refresh) = &spec.refresh {
        check_duration(sink, "spec.refresh", refresh);
        if spec.source.is_none() {
            sink.warn(
                "refresh-without-source",
                "spec.refresh",
                "widgets without a source never refresh",
            );
        }
    }

    match &spec.source {
        Some(Source::File { path, glob }) => {
            if path.contains("{{") {
                sink.error(
                    "template-not-allowed",
                    "spec.source.path",
                    "file paths cannot be templated: they define the widget's permissions",
                );
            } else {
                match confine(&repo.root, &widget.dir, path) {
                    Err(e) => {
                        sink.error("path-escape", "spec.source.path", format!("`{path}`: {e}"))
                    }
                    Ok(p) if !p.exists() => sink.warn(
                        "path-not-found",
                        "spec.source.path",
                        format!("`{path}` does not exist"),
                    ),
                    Ok(_) => {}
                }
            }
            if let Some(glob) = glob
                && let Err(e) = globset::Glob::new(glob)
            {
                sink.error("invalid-glob", "spec.source.glob", e.kind().to_string());
            }
        }
        Some(Source::Command {
            executable,
            args,
            timeout,
        }) => {
            if let Some(problem) = executable_problem(executable) {
                sink.error("invalid-executable", "spec.source.executable", problem);
            }
            for (i, arg) in args.iter().enumerate() {
                check_templates(sink, &scope(false), &format!("spec.source.args[{i}]"), arg);
            }
            if let Some(timeout) = timeout {
                check_duration(sink, "spec.source.timeout", timeout);
            }
        }
        Some(Source::Http { url, headers, .. }) => {
            check_templates(sink, &scope(true), "spec.source.url", url);
            match http_origin(url) {
                Err(e) => sink.error("invalid-url", "spec.source.url", e.to_string()),
                Ok(origin) if origin.starts_with("http://") => sink.warn(
                    "insecure-http",
                    "spec.source.url",
                    "plain http sends headers, including secrets, unencrypted",
                ),
                Ok(_) => {}
            }
            for (name, value) in headers {
                let path = format!("spec.source.headers.{name}");
                if name.contains("{{") {
                    sink.error(
                        "template-not-allowed",
                        &path,
                        "header names cannot be templated",
                    );
                }
                check_templates(sink, &scope(true), &path, value);
            }
        }
        None => {}
    }

    validate_view(spec, sink);

    for (i, host) in spec.permissions.hosts.iter().enumerate() {
        if let Err(e) = bare_origin(host) {
            sink.error(
                "invalid-host",
                &format!("spec.permissions.hosts[{i}]"),
                format!("`{host}`: {e}"),
            );
        }
    }
    if !spec.permissions.hosts.is_empty() && !matches!(spec.source, Some(Source::Http { .. })) {
        sink.warn(
            "unused-permission",
            "spec.permissions.hosts",
            "extra hosts only apply to http sources",
        );
    }
}

fn validate_view(spec: &WidgetSpec, sink: &mut Sink) {
    let view = &spec.view;
    if view.kind == ViewType::Table && view.columns.is_empty() {
        sink.error(
            "missing-columns",
            "spec.view.columns",
            "table views need at least one column",
        );
    }
    for (i, column) in view.columns.iter().enumerate() {
        if column.field.split('.').any(str::is_empty) {
            sink.error(
                "invalid-field",
                &format!("spec.view.columns[{i}].field"),
                format!("`{}` is not a dot path like `metadata.name`", column.field),
            );
        }
    }
    let has_content = view
        .content
        .as_deref()
        .is_some_and(|c| !c.trim().is_empty());
    if spec.source.is_none() {
        if view.kind != ViewType::Markdown {
            sink.error(
                "static-view",
                "spec.view.type",
                "widgets without a source must use a markdown view",
            );
        } else if !has_content {
            sink.error(
                "missing-content",
                "spec.view.content",
                "static markdown widgets need `content`",
            );
        }
    } else if view.content.is_some() {
        sink.warn(
            "ignored-field",
            "spec.view.content",
            "`content` is only used by widgets without a source",
        );
    }
}

fn validate_dashboard(
    platform: Option<&Platform>,
    widgets: &HashMap<&str, &Loaded<Widget>>,
    dashboard: &Dashboard,
    sink: &mut Sink,
) {
    let scope = Scope {
        platform,
        inputs: None,
        allow_secrets: false,
    };
    let placements = &dashboard.spec.widgets;
    for (i, p) in placements.iter().enumerate() {
        let path = format!("spec.widgets[{i}]");
        if p.w == 0 || p.h == 0 {
            sink.error("invalid-size", &path, "w and h must be at least 1");
        } else if p.x.saturating_add(p.w) > GRID_COLUMNS {
            sink.error(
                "out-of-grid",
                &path,
                format!(
                    "x + w = {} exceeds the {GRID_COLUMNS}-column grid",
                    p.x.saturating_add(p.w)
                ),
            );
        }
        if let Some((j, other)) = placements[..i]
            .iter()
            .enumerate()
            .find(|(_, q)| overlaps(p, q))
        {
            sink.error(
                "overlap",
                &path,
                format!("overlaps spec.widgets[{j}] (`{}`)", other.widget),
            );
        }

        let Some(widget) = widgets.get(p.widget.as_str()) else {
            sink.error(
                "unknown-widget",
                &format!("{path}.widget"),
                format!("no widget named `{}`", p.widget),
            );
            continue;
        };
        let declared = &widget.manifest.spec.inputs;
        for (key, value) in &p.inputs {
            let input_path = format!("{path}.inputs.{key}");
            if !declared.contains_key(key) {
                sink.error(
                    "unknown-input",
                    &input_path,
                    format!("widget `{}` has no input `{key}`", p.widget),
                );
            }
            check_templates(sink, &scope, &input_path, value);
        }
        for (key, def) in declared {
            if def.required && def.default.is_none() && !p.inputs.contains_key(key) {
                sink.error(
                    "missing-input",
                    &format!("{path}.inputs"),
                    format!("required input `{key}` of widget `{}` is not set", p.widget),
                );
            }
        }
    }
}

fn overlaps(a: &Placement, b: &Placement) -> bool {
    let nonempty = |p: &Placement| p.w > 0 && p.h > 0;
    nonempty(a)
        && nonempty(b)
        && a.x < b.x.saturating_add(b.w)
        && b.x < a.x.saturating_add(a.w)
        && a.y < b.y.saturating_add(b.h)
        && b.y < a.y.saturating_add(a.h)
}

fn check_templates(sink: &mut Sink, scope: &Scope, path: &str, value: &str) {
    let refs = match template::refs(value) {
        Ok(refs) => refs,
        Err(e) => return sink.error("invalid-template", path, e.to_string()),
    };
    for r in refs {
        let key = &r.key;
        match r.namespace {
            Namespace::User => {
                if let Some(p) = scope.platform
                    && !p.spec.inputs.contains_key(key)
                {
                    sink.error(
                        "unknown-user-input",
                        path,
                        format!("`user.{key}` is not declared in pharos.yaml spec.inputs"),
                    );
                }
            }
            Namespace::Inputs => match scope.inputs {
                Some(inputs) if !inputs.contains_key(key) => sink.error(
                    "unknown-input",
                    path,
                    format!("`inputs.{key}` is not declared in spec.inputs"),
                ),
                Some(_) => {}
                None => sink.error(
                    "template-not-allowed",
                    path,
                    "`inputs.*` is only available inside widgets",
                ),
            },
            Namespace::Secrets => {
                if !scope.allow_secrets {
                    sink.error(
                        "secret-not-allowed",
                        path,
                        "secrets can only be used in http url and headers",
                    );
                } else if let Some(p) = scope.platform
                    && !p.spec.secrets.iter().any(|s| &s.name == key)
                {
                    sink.error(
                        "undeclared-secret",
                        path,
                        format!("`secrets.{key}` is not declared in pharos.yaml spec.secrets"),
                    );
                }
            }
            Namespace::Env => sink.error(
                "env-not-supported",
                path,
                "`env.*` needs spec.environments, which is not supported yet",
            ),
        }
    }
}

fn check_duration(sink: &mut Sink, path: &str, value: &str) {
    if parse_duration(value).is_none() {
        sink.error(
            "invalid-duration",
            path,
            format!("`{value}` is not a duration like 30s, 5m or 1h"),
        );
    }
}

/// Why `executable` is not acceptable as a command source, if it isn't.
pub(crate) fn executable_problem(executable: &str) -> Option<&'static str> {
    if executable.trim().is_empty() {
        Some("executable must not be empty")
    } else if executable.contains("{{") {
        Some("executable cannot be templated: it defines the widget's permissions")
    } else if executable
        .chars()
        .any(|c| c.is_whitespace() || "|&;<>$`()*?".contains(c))
    {
        Some("must be a program name or path; arguments go in `args` (no shell is used)")
    } else {
        None
    }
}

fn is_dns_label(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
