//! HTTP server built on axum.
//!
//! The important part for coverage is `shutdown_signal`: the LLVM profiling
//! runtime writes the `.profraw` file from an exit hook, so the process must
//! return from `main` instead of being killed by a signal. With graceful
//! shutdown, SIGTERM (what `docker stop` sends) and Ctrl-C make `serve` return.

use std::net::SocketAddr;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Operands {
    a: i64,
    b: i64,
}

#[derive(Deserialize)]
struct ReverseQuery {
    s: String,
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/calc/{op}", get(calc_handler))
        .route("/text/palindrome", post(palindrome_handler))
        .route("/text/reverse", get(reverse_handler))
}

async fn calc_handler(
    Path(op): Path<String>,
    Query(q): Query<Operands>,
) -> (StatusCode, Json<Value>) {
    let res = match op.as_str() {
        "add" => calc::add(q.a, q.b),
        "sub" => calc::sub(q.a, q.b),
        "mul" => calc::mul(q.a, q.b),
        "div" => calc::div(q.a, q.b),
        _ => {
            let body = json!({ "error": format!("unknown operation: {op}") });
            return (StatusCode::NOT_FOUND, Json(body));
        }
    };
    match res {
        Ok(v) => (StatusCode::OK, Json(json!({ "result": v }))),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": format!("{e:?}") })),
        ),
    }
}

async fn palindrome_handler(body: String) -> Json<Value> {
    Json(json!({ "palindrome": text::is_palindrome(&body) }))
}

/// Only requested by the test that SIGKILLs the server, so it shows up as
/// uncovered even though the request was served successfully.
async fn reverse_handler(Query(q): Query<ReverseQuery>) -> String {
    text::reverse(&q.s)
}

pub async fn serve(addr: SocketAddr) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // Tests read this line to learn the port (when started with --port 0) and
    // to know the server is ready.
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    eprintln!("shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
