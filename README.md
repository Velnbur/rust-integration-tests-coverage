# Integration test coverage across crates, processes and containers

[![coverage](https://github.com/Velnbur/rust-integration-tests-coverage/actions/workflows/coverage.yml/badge.svg)](https://github.com/Velnbur/rust-integration-tests-coverage/actions/workflows/coverage.yml)
[![codecov](https://codecov.io/gh/Velnbur/rust-integration-tests-coverage/branch/main/graph/badge.svg)](https://codecov.io/gh/Velnbur/rust-integration-tests-coverage)

An example showing that tests living in a **separate `integration-tests`
crate** produce coverage for:

- library crates they call directly (`crates/calc`, `crates/text`),
- the main binary started as a **child process** (CLI and HTTP server),
- the main binary running **inside a Docker container**.

## Layout

| Path | What |
|---|---|
| `crates/calc`, `crates/text` | Libraries. Each has one function no test calls (`factorial`, `shout`). |
| `src/main.rs` | CLI on top of both libraries, plus `serve` for the HTTP server. |
| `src/server.rs` | axum server with graceful shutdown on SIGTERM / Ctrl-C. |
| `integration-tests/` | The separate test crate. `src/lib.rs` has the process/container helpers. |
| `Dockerfile` | Release image, or an instrumented one with `COVERAGE=1`. |
| `scripts/coverage.sh` | Builds, runs the tests, merges and reports coverage. |

## Run it

```sh
rustup toolchain install          # reads rust-toolchain.toml (adds llvm-tools)
cargo install cargo-llvm-cov
scripts/coverage.sh                # text summary
scripts/coverage.sh report --html  # HTML in target/llvm-cov/html
```

Docker is optional; without it (or with `SKIP_DOCKER=1`) the container tests
are skipped.

## How it works

1. `cargo llvm-cov show-env` exports a `RUSTC_WRAPPER` that adds
   `-C instrument-coverage` to workspace crates, and an `LLVM_PROFILE_FILE`
   pattern. A plain `cargo build` then produces an instrumented binary.
2. Every instrumented process (test binaries and the spawned app) inherits
   `LLVM_PROFILE_FILE` and writes its own `.profraw` **when it exits
   normally**.
3. `cargo llvm-cov report` merges all `.profraw` files and maps them back to
   source using the coverage mapping inside each instrumented executable.

### HTTP server

The profile is written from an exit hook, so a server that is SIGKILLed
(`Child::kill`, `docker kill`, a `docker stop` timeout) loses **all** of its
coverage. The server handles SIGTERM via `with_graceful_shutdown`, and the
test helper stops it with SIGTERM, also from `Drop` so panicking tests are
still recorded.

`sigkill_loses_coverage` in `integration-tests/tests/server.rs` demonstrates
the failure mode: the test passes, yet `reverse_handler` (only requested by
that test) is reported as uncovered.

### Docker

- The image is built with `COVERAGE=1` and `WORKSPACE_DIR=$PWD`, so the
  absolute source paths in the binary's coverage mapping match the host
  checkout.
- The target dir is bind-mounted into the container and `LLVM_PROFILE_FILE`
  points into it. The file name includes a per-container id because the
  server is PID 1 in every container, so `%p` alone would collide.
- The exact binary from the image is copied to `target/debug/docker/`, where
  `cargo llvm-cov report` picks it up next to the host binary.

`palindrome_handler` is only requested from the container tests, so its
coverage in the report proves the container's profile was merged.

## CI

`.github/workflows/coverage.yml` runs the script on every push and PR, writes
the summary to the job page, uploads the HTML report as an artifact, and sends
`lcov.info` to Codecov. For Codecov, sign in at codecov.io with GitHub and
enable the repo; add a `CODECOV_TOKEN` secret unless tokenless upload is
enabled for the org.
