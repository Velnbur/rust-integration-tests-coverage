//! Talk to the HTTP server running as a local child process.

use integration_tests::{Server, get};

#[test]
fn server_add() {
    let server = Server::local();
    let resp = get(&server.url("/calc/add?a=40&b=2"));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, r#"{"result":42}"#);
    assert_eq!(server.shutdown(), 0, "server must exit cleanly");
}

#[test]
fn server_unknown_op() {
    let server = Server::local();
    let resp = get(&server.url("/calc/pow?a=2&b=3"));
    assert_eq!(resp.status, 404);
    assert_eq!(server.shutdown(), 0, "server must exit cleanly");
}

/// Passes, but the coverage of everything this server process executed is
/// lost: SIGKILL gives the profiling runtime no chance to write its file.
/// `reverse_handler` in `src/server.rs` is only reached from here, so it
/// stays uncovered in the report.
#[test]
fn sigkill_loses_coverage() {
    let server = Server::local();
    let resp = get(&server.url("/text/reverse?s=abc"));
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, "cba");
    server.kill();
}
