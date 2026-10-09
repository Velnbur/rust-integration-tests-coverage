//! Spawn the main binary as a subprocess.

use integration_tests::{run, stdout};

#[test]
fn cli_add() {
    let out = run(&["add", "40", "2"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "42");
}

#[test]
fn cli_div_by_zero() {
    let out = run(&["div", "1", "0"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn cli_words() {
    let out = run(&["words", "one two  three"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "3");
}

#[test]
fn cli_usage() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(2));
}
