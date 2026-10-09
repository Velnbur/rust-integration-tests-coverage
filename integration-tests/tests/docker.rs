//! Talk to the HTTP server running inside a Docker container.
//!
//! Needs the `APP_IMAGE` env var (set by `scripts/coverage.sh`); the tests are
//! skipped without it so a plain `cargo test` still works.

use integration_tests::{Server, get, post};

fn server() -> Option<Server> {
    match std::env::var("APP_IMAGE") {
        Ok(image) => Some(Server::docker(&image)),
        Err(_) => {
            eprintln!("skipping: APP_IMAGE is not set");
            None
        }
    }
}

#[test]
fn docker_div_by_zero() {
    let Some(server) = server() else { return };
    let resp = get(&server.url("/calc/div?a=1&b=0"));
    assert_eq!(resp.status, 422);
    assert_eq!(resp.body, r#"{"error":"DivisionByZero"}"#);
    assert_eq!(server.shutdown(), 0, "container must exit cleanly");
}

/// `palindrome_handler` is only reached from inside the container, so its
/// coverage proves the container's profile made it into the report.
#[test]
fn docker_palindrome() {
    let Some(server) = server() else { return };
    let resp = post(
        &server.url("/text/palindrome"),
        "Was it a car or a cat I saw?",
    );
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, r#"{"palindrome":true}"#);
    assert_eq!(server.shutdown(), 0, "container must exit cleanly");
}
