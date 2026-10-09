#!/usr/bin/env bash
# Collect coverage for the whole workspace (libs + main binary) driven by the
# tests in the separate `integration-tests` crate, including a copy of the
# binary running inside a Docker container.
#
# Usage:
#   scripts/coverage.sh                  collect, then print a text summary
#   scripts/coverage.sh collect          only collect
#   scripts/coverage.sh report [ARGS]    report from collected data, ARGS go to
#                                        `cargo llvm-cov report` (--html, --lcov ...)
#
# Set SKIP_DOCKER=1 to skip the container part.
set -euo pipefail
cd "$(dirname "$0")/.."

IMAGE="integration-tests-coverage:coverage"

# Exports RUSTC_WRAPPER (instruments workspace crates in a plain `cargo build`)
# and LLVM_PROFILE_FILE (where every instrumented process writes its profile).
# Captured first so a failure stops the script (`source <(...)` would hide it).
cov_env=$(cargo llvm-cov show-env --sh)
eval "$cov_env"

collect() {
    cargo llvm-cov clean --workspace

    cargo build --workspace --bins

    if [[ -z "${SKIP_DOCKER:-}" ]] && docker info >/dev/null 2>&1; then
        docker build \
            --build-arg COVERAGE=1 \
            --build-arg WORKSPACE_DIR="$PWD" \
            --tag "$IMAGE" .

        # `report` needs the exact binary that produced the profiles. It scans
        # target/debug recursively for files named after workspace targets, so
        # a copy in a subdirectory is picked up next to the host binary.
        local out="$CARGO_LLVM_COV_TARGET_DIR/debug/docker"
        mkdir -p "$out"
        local cid
        cid=$(docker create "$IMAGE")
        docker cp "$cid:/usr/local/bin/integration-tests-coverage" "$out/"
        docker rm "$cid" >/dev/null

        export APP_IMAGE="$IMAGE"
        export COVERAGE_DIR="$CARGO_LLVM_COV_TARGET_DIR"
    else
        echo "warning: docker is not available, skipping container tests" >&2
    fi

    cargo test -p integration-tests
}

report() {
    # --workspace: without it `report` only shows the root package.
    # The container build instruments dependencies too (plain RUSTFLAGS), and
    # their sources live under the image's CARGO_HOME: hide them as well.
    cargo llvm-cov report --workspace \
        --ignore-filename-regex '(^|/)integration-tests/|^/usr/local/cargo/' "$@"
}

case "${1:-}" in
    collect) collect ;;
    report) shift; report "$@" ;;
    "") collect; report ;;
    *) echo "unknown command: $1" >&2; exit 2 ;;
esac
