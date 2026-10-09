# syntax=docker/dockerfile:1

# Build stage.
#
# COVERAGE=1 builds an instrumented binary. WORKSPACE_DIR should be the host
# workspace path: the coverage mapping embedded in the binary stores absolute
# source paths, and they must match the host checkout for the report.
FROM rust:1.98.0-slim-trixie AS builder
ARG WORKSPACE_DIR=/app
ARG COVERAGE=0
WORKDIR ${WORKSPACE_DIR}
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=${WORKSPACE_DIR}/target \
    if [ "$COVERAGE" = "1" ]; then \
        export RUSTFLAGS="-C instrument-coverage --cfg coverage"; \
        profile=dev; out=debug; \
    else \
        profile=release; out=release; \
    fi \
    && cargo build --locked --profile "$profile" --bin integration-tests-coverage \
    && cp "target/$out/integration-tests-coverage" /usr/local/bin/

# Runtime stage.
FROM debian:trixie-slim
COPY --from=builder /usr/local/bin/integration-tests-coverage /usr/local/bin/
EXPOSE 8080
# The binary handles SIGTERM itself, so it can run as PID 1 without an init.
ENTRYPOINT ["integration-tests-coverage"]
CMD ["serve", "--host", "0.0.0.0", "--port", "8080"]
