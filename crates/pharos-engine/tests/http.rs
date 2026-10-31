//! Http sources against a local server: templating, declared secrets, the approved origin
//! and what is never done (following redirects, leaking secret values into errors).

mod common;

use common::{Fixture, MemoryStore};
use pharos_engine::{Engine, EngineError, HttpError};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

const TOKEN: &str = "glpat-s3cret";

/// Answers one connection per canned response, in order, and records each request head.
struct Server {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
}

fn serve(responses: Vec<String>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                head.push(byte[0]);
            }
            seen.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).into_owned());
            let _ = stream.write_all(response.as_bytes());
        }
    });
    Server { origin, requests }
}

fn response(status: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn json_ok(body: &str) -> String {
    response("200 OK", "Content-Type: application/json\r\n", body)
}

/// The example repo plus a `local` dashboard whose only widget calls `origin`, the way
/// My Pipelines calls GitLab. Team and token are set unless `setup` is false.
fn engine_for(origin: &str, setup: bool) -> Engine {
    let f = Fixture::new();
    f.write(
        "widgets/local-api/widget.yaml",
        &format!(
            r#"apiVersion: pharos.io/v1alpha1
kind: Widget
metadata: {{ name: local-api, version: 0.1.0 }}
spec:
  inputs:
    project: {{ type: string, required: true }}
  source:
    type: http
    url: "{origin}/api/{{{{ inputs.project }}}}/pipelines?team={{{{ user.team }}}}"
    headers:
      PRIVATE-TOKEN: "{{{{ secrets.GITLAB_TOKEN }}}}"
  view:
    type: table
    columns: [ {{ field: id }} ]
"#
        ),
    );
    f.write(
        "dashboards/local.yaml",
        r#"apiVersion: pharos.io/v1alpha1
kind: Dashboard
metadata: { name: local }
spec:
  widgets:
    - { widget: local-api, x: 0, y: 0, w: 12, h: 4, inputs: { project: "acme%2F{{ user.team }}" } }
"#,
    );
    f.commit("local api widget");
    let mut engine = f.synced_engine_with(MemoryStore::default());
    if setup {
        engine.set_input("team", Some("payments-team")).unwrap();
        engine.set_secret("GITLAB_TOKEN", TOKEN).unwrap();
    }
    // The fixture's temp dir must outlive the engine's clone.
    std::mem::forget(f);
    engine
}

#[test]
fn sends_the_templated_request_and_returns_rows() {
    let server = serve(vec![json_ok(
        r#"[{"id": 1, "status": "success"}, {"id": 2}]"#,
    )]);
    let engine = engine_for(&server.origin, true);

    let data = engine.run_placement("local", 0).unwrap();

    assert_eq!(
        data.rows,
        [json!({"id": 1, "status": "success"}), json!({"id": 2})]
    );
    assert!(!data.truncated);
    let requests = server.requests.lock().unwrap();
    let head = requests[0].to_ascii_lowercase();
    // Literal `%2F` from the dashboard is kept; the user's team is encoded into it.
    assert!(
        head.starts_with("get /api/acme%2fpayments-team/pipelines?team=payments-team http/1.1"),
        "{head}"
    );
    assert!(head.contains(&format!("private-token: {}", TOKEN.to_ascii_lowercase())));
    assert!(head.contains("accept: application/json"));
}

#[test]
fn needs_setup_until_team_and_token_are_set() {
    let server = serve(vec![]);
    let engine = engine_for(&server.origin, false);

    assert!(matches!(
        engine.run_placement("local", 0),
        Err(EngineError::NeedsSetup(name)) if name == "local-api"
    ));
    assert!(server.requests.lock().unwrap().is_empty());
}

#[test]
fn auth_failures_never_echo_the_token() {
    let body = format!(r#"{{"message": "401 Unauthorized: bad token {TOKEN}"}}"#);
    let server = serve(vec![response("401 Unauthorized", "", &body)]);
    let engine = engine_for(&server.origin, true);

    let err = engine.run_placement("local", 0).unwrap_err();

    let EngineError::Http(http) = &err else {
        panic!("{err:?}")
    };
    assert!(http.is_auth());
    let message = err.to_string();
    assert!(message.contains("answered 401"), "{message}");
    assert!(message.contains("bad token •••"), "{message}");
    assert!(!message.contains(TOKEN), "{message}");
}

#[test]
fn does_not_follow_redirects() {
    let server = serve(vec![
        response(
            "302 Found",
            "Location: https://elsewhere.example/steal\r\n",
            "",
        ),
        json_ok("[]"),
    ]);
    let engine = engine_for(&server.origin, true);

    let err = engine.run_placement("local", 0).unwrap_err();

    assert!(
        matches!(&err, EngineError::Http(HttpError::Redirected { location, .. }) if location == "https://elsewhere.example/steal"),
        "{err:?}"
    );
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}

#[test]
fn reports_bodies_that_are_not_json() {
    let server = serve(vec![response("200 OK", "", "<html>login</html>")]);
    let engine = engine_for(&server.origin, true);

    assert!(matches!(
        engine.run_placement("local", 0),
        Err(EngineError::Http(HttpError::Parse { .. }))
    ));
}

#[test]
fn reports_unreachable_hosts() {
    // Bound, then closed: nothing listens on this port any more.
    let origin = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let engine = engine_for(&origin, true);

    let err = engine.run_placement("local", 0).unwrap_err();

    assert!(
        matches!(&err, EngineError::Http(HttpError::Request { origin: o, .. }) if *o == origin),
        "{err:?}"
    );
}
