use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// One finding, located by repo-relative file and a dotted path into the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}: {}", self.file)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
        }
        if let Some(path) = &self.path {
            write!(f, ": {path}")?;
        }
        write!(f, ": {} [{}]", self.message, self.code)
    }
}

/// Collects diagnostics for one file.
pub(crate) struct Sink<'a> {
    pub file: &'a str,
    pub out: &'a mut Vec<Diagnostic>,
}

impl Sink<'_> {
    pub fn error(&mut self, code: &'static str, path: &str, message: impl Into<String>) {
        self.push(Severity::Error, code, path, None, message.into());
    }

    pub fn warn(&mut self, code: &'static str, path: &str, message: impl Into<String>) {
        self.push(Severity::Warning, code, path, None, message.into());
    }

    pub fn error_at_line(
        &mut self,
        code: &'static str,
        path: &str,
        line: Option<usize>,
        message: String,
    ) {
        self.push(Severity::Error, code, path, line, message);
    }

    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        path: &str,
        line: Option<usize>,
        message: String,
    ) {
        self.out.push(Diagnostic {
            severity,
            code,
            file: self.file.to_owned(),
            path: (!path.is_empty() && path != ".").then(|| path.to_owned()),
            line,
            message,
        });
    }
}
