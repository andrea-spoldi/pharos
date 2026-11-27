//! File sources: read repo files for a widget, confined to the repo root and to the paths
//! the user approved. Checked on every run, not just at consent time. Also the row shape
//! every source returns.

use pharos_manifest::{ConfineError, confine, display_relative};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Most files one file source reads; the rest are dropped and `truncated` is set.
pub const MAX_FILES: usize = 500;
/// Most rows any source yields, counted after `rows` selection; the rest are dropped and
/// `truncated` is set.
pub const MAX_ROWS: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WidgetData {
    /// One per item of a top-level list, else one per document. Non-object values are
    /// wrapped as `{ value }`. File rows carry `_source`: the repo-relative file.
    #[cfg_attr(
        feature = "specta",
        specta(type = Vec<std::collections::BTreeMap<String, JsonValue>>)
    )]
    pub rows: Vec<Value>,
    pub truncated: bool,
}

/// TS shape of `serde_json::Value`, which specta refuses for its i64 numbers (D-018).
/// Only used for the bindings.
#[cfg(feature = "specta")]
#[derive(Serialize, specta::Type)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(std::collections::BTreeMap<String, JsonValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FileError {
    #[error("`{path}`: {reason}")]
    Denied { path: String, reason: ConfineError },
    #[error("`{0}` is not an approved path")]
    NotApproved(String),
    #[error("invalid glob: {0}")]
    Glob(String),
    #[error("cannot read `{file}`: {message}")]
    Read { file: String, message: String },
    #[error("cannot parse `{file}`: {message}")]
    Parse { file: String, message: String },
    #[error(transparent)]
    Rows(#[from] RowsError),
}

/// A source's `rows` selector did not fit the data. Never a lint error: only the data knows.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RowsError {
    #[error("`rows: {path}` matched nothing in {place}")]
    NotFound { path: String, place: String },
    #[error("`rows: {path}` needs JSON output, but {what}")]
    NotJson { path: String, what: String },
}

/// The value at a dot path, mirroring `field()` in `app/src/views/fields.ts` (shared cases in
/// `testdata/dot-paths.json`): an object takes the key, an array the index that JS `Number()`
/// makes of the segment, anything else has nothing below it. Lint guarantees no empty or
/// whitespace segments.
pub(crate) fn select<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |value, key| match value {
        Value::Object(map) => map.get(key),
        Value::Array(items) => items.get(js_index(key)?),
        _ => None,
    })
}

/// What `Number(key)` is, when that is a non-negative integer (so `"01"` is 1, `"-1"` and
/// `"1.5"` are not indexes).
fn js_index(key: &str) -> Option<usize> {
    let radix = |prefix: &[&str], radix| {
        let digits = prefix.iter().find_map(|p| key.strip_prefix(p))?;
        // `from_str_radix` accepts a leading `+`, which `Number` does not after a prefix.
        if digits.starts_with('+') {
            return None;
        }
        u64::from_str_radix(digits, radix).ok().map(|n| n as f64)
    };
    let number = radix(&["0x", "0X"], 16)
        .or_else(|| radix(&["0o", "0O"], 8))
        .or_else(|| radix(&["0b", "0B"], 2))
        .or_else(|| {
            // Rust also accepts `inf`/`nan`, which are never indexes anyway.
            key.parse::<f64>().ok()
        })?;
    (number.is_finite() && number >= 0.0 && number.fract() == 0.0 && number < usize::MAX as f64)
        .then_some(number as usize)
}

/// Applies the optional `rows` selector, then converts to rows. `place` names where the data
/// came from for the error. A path that resolves to `null` counts as a miss: an empty result
/// would hide an authoring mistake.
pub(crate) fn select_rows(
    value: Value,
    path: Option<&str>,
    place: &str,
    max: usize,
) -> Result<WidgetData, RowsError> {
    Ok(rows(selected(value, path, place)?, max))
}

fn selected(value: Value, path: Option<&str>, place: &str) -> Result<Value, RowsError> {
    let Some(path) = path else {
        return Ok(value);
    };
    match select(&value, path) {
        Some(Value::Null) | None => Err(RowsError::NotFound {
            path: path.into(),
            place: place.into(),
        }),
        Some(found) => Ok(found.clone()),
    }
}

/// Reads `path` (relative to the widget's `dir`) as one file, or as a directory filtered by
/// `glob`. `root` must be canonical. `approved` holds repo-relative paths.
#[allow(clippy::too_many_arguments)] // file cap and row cap differ per caller (widget vs catalog)
pub(crate) fn read_files(
    root: &Path,
    dir: &Path,
    path: &str,
    glob: Option<&str>,
    rows_path: Option<&str>,
    approved: &BTreeSet<String>,
    max: usize,
    max_rows: usize,
) -> Result<WidgetData, FileError> {
    let target = confine(root, dir, path).map_err(|reason| FileError::Denied {
        path: path.into(),
        reason,
    })?;
    let target_rel = display_relative(root, &target);
    if !approved.contains(&target_rel) {
        return Err(FileError::NotApproved(target_rel));
    }

    let mut files = match glob {
        None => vec![target.clone()],
        Some(glob) => {
            // `*` stops at `/`, so only `**` descends into subdirectories.
            let matcher = globset::GlobBuilder::new(glob)
                .literal_separator(true)
                .build()
                .map_err(|e| FileError::Glob(e.kind().to_string()))?
                .compile_matcher();
            let mut files = Vec::new();
            walk(root, &target, &mut files)?;
            files.retain(|f| {
                f.strip_prefix(&target)
                    .is_ok_and(|rel| matcher.is_match(rel))
            });
            files.sort();
            files
        }
    };
    let truncated = files.len() > max;
    files.truncate(max);

    let mut rows = Vec::new();
    for file in files {
        let rel = display_relative(root, &file);
        let real = fs::canonicalize(&file).map_err(|e| read_error(&rel, e))?;
        // A symlink may not leave the approved path, let alone the repo.
        if !real.starts_with(&target) {
            return Err(FileError::Denied {
                path: rel,
                reason: ConfineError::SymlinkEscape,
            });
        }
        let text = fs::read_to_string(&real).map_err(|e| read_error(&rel, e))?;
        let value: Value = serde_yaml_ng::from_str(&text).map_err(|e| FileError::Parse {
            file: rel.clone(),
            message: e.to_string(),
        })?;
        // The selector applies to each document; the error names the file.
        match selected(value, rows_path, &rel)? {
            Value::Array(items) => rows.extend(items.into_iter().map(|v| with_source(v, &rel))),
            value => rows.push(with_source(value, &rel)),
        }
    }
    let truncated = truncated || rows.len() > max_rows;
    rows.truncate(max_rows);
    Ok(WidgetData { rows, truncated })
}

/// Rows of one parsed response: the items of a top-level list, else the value itself.
pub(crate) fn rows(value: Value, max: usize) -> WidgetData {
    let items = match value {
        Value::Array(items) => items,
        value => vec![value],
    };
    let truncated = items.len() > max;
    let rows = items
        .into_iter()
        .take(max)
        .map(|v| Value::Object(as_row(v)))
        .collect();
    WidgetData { rows, truncated }
}

/// Every row of the YAML/JSON files directly under a catalog directory, e.g. for input
/// choices. Confined to the repo like widget reads, but not a widget run, so no approved-path
/// check (D-029). A missing directory has no rows.
pub(crate) fn read_catalog(root: &Path, from: &str, max: usize) -> Result<Vec<Value>, FileError> {
    let target = confine(root, root, from).map_err(|reason| FileError::Denied {
        path: from.into(),
        reason,
    })?;
    if !target.is_dir() {
        return Ok(Vec::new());
    }
    let approved = BTreeSet::from([display_relative(root, &target)]);
    Ok(read_files(
        root,
        root,
        from,
        Some("*.{yaml,yml,json}"),
        None,
        &approved,
        max,
        usize::MAX,
    )?
    .rows)
}

/// Every non-directory under `dir`. Symlinked directories are not followed.
fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), FileError> {
    let entries = fs::read_dir(dir).map_err(|e| read_error(&display_relative(root, dir), e))?;
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(root, &entry.path(), out)?,
            Ok(_) => out.push(entry.path()),
            Err(_) => {}
        }
    }
    Ok(())
}

fn as_row(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(row) => row,
        value => Map::from_iter([("value".to_string(), value)]),
    }
}

fn with_source(value: Value, source: &str) -> Value {
    let mut row = as_row(value);
    row.insert("_source".into(), source.into());
    Value::Object(row)
}

fn read_error(file: &str, e: std::io::Error) -> FileError {
    FileError::Read {
        file: file.into(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pharos_manifest::ConfineError;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// root/
    /// ├─ widget/            (the widget's directory)
    /// └─ data/a.yaml, b.json, notes.txt, nested/c.yaml
    fn setup() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap().join("repo");
        fs::create_dir_all(root.join("widget")).unwrap();
        fs::create_dir_all(root.join("data/nested")).unwrap();
        fs::write(root.join("data/a.yaml"), "name: a").unwrap();
        fs::write(root.join("data/b.json"), r#"{"name": "b"}"#).unwrap();
        fs::write(root.join("data/notes.txt"), "hello").unwrap();
        fs::write(root.join("data/nested/c.yaml"), "name: c").unwrap();
        (tmp, root)
    }

    fn approved(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    fn sources(data: &WidgetData) -> Vec<&str> {
        data.rows
            .iter()
            .map(|r| r["_source"].as_str().unwrap())
            .collect()
    }

    fn read(
        root: &Path,
        path: &str,
        glob: Option<&str>,
        ok: &[&str],
    ) -> Result<WidgetData, FileError> {
        read_files(
            root,
            &root.join("widget"),
            path,
            glob,
            None,
            &approved(ok),
            MAX_FILES,
            MAX_ROWS,
        )
    }

    #[test]
    fn glob_matches_the_top_level_only_unless_it_says_otherwise() {
        let (_tmp, root) = setup();
        let top = read(&root, "../data", Some("*.yaml"), &["data"]).unwrap();
        assert_eq!(sources(&top), ["data/a.yaml"]);
        assert_eq!(top.rows[0]["name"], "a");

        let deep = read(&root, "../data", Some("**/*.yaml"), &["data"]).unwrap();
        assert_eq!(sources(&deep), ["data/a.yaml", "data/nested/c.yaml"]);
    }

    #[test]
    fn parses_json_files_too() {
        let (_tmp, root) = setup();
        let data = read(&root, "../data", Some("*.json"), &["data"]).unwrap();
        assert_eq!(data.rows[0]["name"], "b");
    }

    #[test]
    fn reads_a_single_file_without_a_glob() {
        let (_tmp, root) = setup();
        let data = read(&root, "../data/a.yaml", None, &["data/a.yaml"]).unwrap();
        assert_eq!(sources(&data), ["data/a.yaml"]);
    }

    #[test]
    fn a_top_level_list_becomes_one_row_per_item() {
        let (_tmp, root) = setup();
        fs::write(root.join("data/list.yaml"), "- name: x\n- name: y\n- 3\n").unwrap();
        let data = read(&root, "../data/list.yaml", None, &["data/list.yaml"]).unwrap();
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[1]["name"], "y");
        assert_eq!(data.rows[1]["_source"], "data/list.yaml");
        assert_eq!(data.rows[2]["value"], 3);
    }

    #[test]
    fn denies_traversal_out_of_the_repo() {
        let (_tmp, root) = setup();
        let err = read(&root, "../../etc", Some("*"), &["data"]).unwrap_err();
        assert_eq!(
            err,
            FileError::Denied {
                path: "../../etc".into(),
                reason: ConfineError::Escapes
            }
        );
    }

    #[test]
    fn denies_paths_that_were_not_approved() {
        let (_tmp, root) = setup();
        let err = read(&root, "../data", Some("*.yaml"), &["other"]).unwrap_err();
        assert_eq!(err, FileError::NotApproved("data".into()));
    }

    #[test]
    fn denies_matched_files_that_symlink_out_of_the_repo() {
        let (tmp, root) = setup();
        let outside = tmp.path().join("outside.yaml");
        fs::write(&outside, "secret: 1").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("data/z.yaml")).unwrap();
        let err = read(&root, "../data", Some("*.yaml"), &["data"]).unwrap_err();
        assert_eq!(
            err,
            FileError::Denied {
                path: "data/z.yaml".into(),
                reason: ConfineError::SymlinkEscape
            }
        );
    }

    #[test]
    fn does_not_descend_into_symlinked_directories() {
        let (tmp, root) = setup();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("s.yaml"), "secret: 1").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("data/link")).unwrap();
        let data = read(&root, "../data", Some("**/*.yaml"), &["data"]).unwrap();
        assert_eq!(sources(&data), ["data/a.yaml", "data/nested/c.yaml"]);
    }

    #[test]
    fn stops_at_the_file_cap() {
        let (_tmp, root) = setup();
        let data = read_files(
            &root,
            &root.join("widget"),
            "../data",
            Some("**/*.yaml"),
            None,
            &approved(&["data"]),
            1,
            MAX_ROWS,
        )
        .unwrap();
        assert_eq!(sources(&data), ["data/a.yaml"]);
        assert!(data.truncated);
    }

    #[test]
    fn reports_unparseable_files_by_path() {
        let (_tmp, root) = setup();
        fs::write(root.join("data/bad.yaml"), "a: [").unwrap();
        let err = read(&root, "../data", Some("bad.yaml"), &["data"]).unwrap_err();
        assert!(matches!(err, FileError::Parse { ref file, .. } if file == "data/bad.yaml"));
    }

    fn read_rows(
        root: &Path,
        path: &str,
        glob: Option<&str>,
        rows: &str,
    ) -> Result<WidgetData, FileError> {
        read_files(
            root,
            &root.join("widget"),
            path,
            glob,
            Some(rows),
            &approved(&[path.trim_start_matches("../")]),
            MAX_FILES,
            MAX_ROWS,
        )
    }

    #[test]
    fn select_matches_shared_cases() {
        let cases: Vec<Value> =
            serde_json::from_str(include_str!("../../../testdata/dot-paths.json")).unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let path = case["path"].as_str().unwrap();
            let got = select(&case["value"], path);
            if case["found"].as_bool().unwrap() {
                assert_eq!(got, Some(&case["expected"]), "{case}");
            } else {
                assert_eq!(got, None, "{case}");
            }
        }
    }

    #[test]
    fn file_rows_selected_per_document() {
        let (_tmp, root) = setup();
        fs::write(
            root.join("data/one.yaml"),
            "items:\n  - name: x\n  - name: y\n",
        )
        .unwrap();
        fs::write(root.join("data/two.yaml"), "items:\n  - name: z\n").unwrap();
        let data = read_rows(&root, "../data", Some("{one,two}.yaml"), "items").unwrap();
        let names: Vec<_> = data
            .rows
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["x", "y", "z"]);
        assert_eq!(
            sources(&data),
            ["data/one.yaml", "data/one.yaml", "data/two.yaml"]
        );
        assert!(!data.truncated);
    }

    #[test]
    fn file_rows_not_found_names_the_file() {
        let (_tmp, root) = setup();
        fs::write(root.join("data/one.yaml"), "items: []\n").unwrap();
        let err = read_rows(&root, "../data/one.yaml", None, "nope").unwrap_err();
        assert_eq!(
            err,
            FileError::Rows(RowsError::NotFound {
                path: "nope".into(),
                place: "data/one.yaml".into()
            })
        );
        assert_eq!(
            err.to_string(),
            "`rows: nope` matched nothing in data/one.yaml"
        );
    }

    #[test]
    fn file_rows_capped_after_selection() {
        let (_tmp, root) = setup();
        let body: String = (0..600).map(|i| format!("  - n: {i}\n")).collect();
        fs::write(root.join("data/big.yaml"), format!("items:\n{body}")).unwrap();
        let data = read_rows(&root, "../data/big.yaml", None, "items").unwrap();
        assert_eq!(data.rows.len(), MAX_ROWS);
        assert!(data.truncated);
    }

    #[test]
    fn catalog_reads_are_not_capped_at_the_widget_row_limit() {
        let (_tmp, root) = setup();
        let body: String = (0..600).map(|i| format!("- n: {i}\n")).collect();
        fs::write(root.join("data/big.yaml"), body).unwrap();
        let rows = read_catalog(&root, "data", MAX_FILES).unwrap();
        let big = rows
            .iter()
            .filter(|r| r["_source"] == "data/big.yaml")
            .count();
        assert_eq!(big, 600);
    }

    #[test]
    fn a_selected_null_is_not_found() {
        let err = select_rows(
            serde_json::json!({ "a": null }),
            Some("a"),
            "the output",
            10,
        )
        .unwrap_err();
        assert!(matches!(err, RowsError::NotFound { .. }));
    }

    #[test]
    fn rows_from_lists_objects_and_scalars() {
        use serde_json::json;
        let data = rows(json!([{ "id": 1 }, 2, { "id": 3 }]), 2);
        assert_eq!(data.rows, [json!({ "id": 1 }), json!({ "value": 2 })]);
        assert!(data.truncated);
        assert_eq!(rows(json!({ "id": 1 }), 10).rows, [json!({ "id": 1 })]);
    }
}
