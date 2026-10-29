use clap::{Parser, Subcommand, ValueEnum};
use pharos_manifest::LintReport;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "pharos",
    version,
    about = "Tooling for Pharos platform repos"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a platform repo. Exits 1 if there are errors.
    Lint {
        /// Platform repo root (the directory containing pharos.yaml).
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let Command::Lint { path, format } = Cli::parse().command;
    let report = pharos_manifest::lint(&path);
    match format {
        Format::Text => print!("{}", render_text(&report)),
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serialises")
        ),
    }
    if report.errors() > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn render_text(report: &LintReport) -> String {
    let mut out = String::new();
    for d in &report.diagnostics {
        out.push_str(&format!("{d}\n"));
    }
    let s = &report.summary;
    out.push_str(&format!(
        "{}, {}, {}: {}, {}\n",
        plural(s.widgets, "widget"),
        plural(s.dashboards, "dashboard"),
        plural(s.catalog_files, "catalog file"),
        plural(report.errors(), "error"),
        plural(report.warnings(), "warning"),
    ));
    if report.errors() == 0 {
        out.push_str(&format!("\ncapabilities ({}):\n", report.capability_hash));
        for (widget, caps) in &report.capabilities.widgets {
            let lines = caps
                .hosts
                .iter()
                .map(|h| ("host", h))
                .chain(caps.executables.iter().map(|e| ("exec", e)))
                .chain(caps.paths.iter().map(|p| ("path", p)));
            for (kind, value) in lines {
                out.push_str(&format!("  {widget:<24} {kind:<5} {value}\n"));
            }
        }
    }
    out
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}
