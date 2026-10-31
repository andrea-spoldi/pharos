//! Platform repo contract: the `Platform`, `Widget` and `Dashboard` kinds, repo loading,
//! validation with structured diagnostics, and capability (permission) derivation.
//!
//! Deliberately free of Tauri so it can be shared by the app engine and `pharos lint`.

mod capabilities;
mod diagnostic;
mod duration;
mod kinds;
mod load;
mod paths;
mod template;
mod validate;

use serde::Serialize;
use std::path::Path;

pub use capabilities::{Capabilities, CapabilitySet, OriginError, http_origin};
pub use diagnostic::{Diagnostic, Severity};
pub use duration::parse_duration;
pub use kinds::*;
pub use load::{Loaded, Repo, load};
pub use paths::{ConfineError, confine, display_relative};
pub use template::{
    Namespace, TemplateError, TemplateRef, refs as template_refs, render as render_template,
};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub widgets: usize,
    pub dashboards: usize,
    pub catalog_files: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub diagnostics: Vec<Diagnostic>,
    pub summary: Summary,
    pub capabilities: CapabilitySet,
    pub capability_hash: String,
}

impl LintReport {
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }

    fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }
}

/// Loads and validates the platform repo at `root`.
pub fn lint(root: &Path) -> LintReport {
    let (repo, mut diagnostics) = load(root);
    let mut summary = Summary::default();
    let mut capabilities = CapabilitySet::default();

    if let Some(repo) = &repo {
        validate::validate(repo, &mut diagnostics);
        capabilities = capabilities::derive(repo);
        summary = Summary {
            widgets: repo.widgets.len(),
            dashboards: repo.dashboards.len(),
            catalog_files: repo.catalog_files.len(),
        };
    }

    // Stable: keeps per-file check order, groups by file.
    diagnostics.sort_by(|a, b| a.file.cmp(&b.file));
    let capability_hash = capabilities.hash();

    LintReport {
        diagnostics,
        summary,
        capabilities,
        capability_hash,
    }
}
