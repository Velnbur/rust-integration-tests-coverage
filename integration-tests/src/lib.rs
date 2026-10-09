//! Helpers shared by the integration tests.
//!
//! `CARGO_BIN_EXE_<name>` is only available to tests of the package that owns
//! the binary, so from a separate crate we locate the binary in the target dir
//! next to the test executable (`target/<profile>/deps/<test>` ->
//! `target/<profile>/<bin>`). The binary must be built beforehand with the
//! same (instrumented) flags, see `scripts/coverage.sh`.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub const BIN_NAME: &str = "integration-tests-coverage";

/// Port the server listens on inside the container.
const CONTAINER_PORT: u16 = 8080;

pub fn bin_path() -> PathBuf {
    if let Ok(p) = std::env::var("APP_BIN") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary is expected in target/<profile>/deps");
    let path = profile_dir.join(format!("{BIN_NAME}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.exists(),
        "binary not found at {}; build it first (cargo build -p {BIN_NAME}) or set APP_BIN",
        path.display()
    );
    path
}

pub fn run(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("failed to spawn binary")
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Minimal HTTP response used by the tests.
pub struct Response {
    pub status: u16,
    pub body: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into()
}

pub fn get(url: &str) -> Response {
    try_get(url).unwrap_or_else(|e| panic!("GET {url} failed: {e}"))
}

pub fn post(url: &str, body: &str) -> Response {
    let mut resp = agent()
        .post(url)
        .send(body)
        .unwrap_or_else(|e| panic!("POST {url} failed: {e}"));
    Response {
        status: resp.status().as_u16(),
        body: resp.body_mut().read_to_string().expect("read body"),
    }
}

fn try_get(url: &str) -> Result<Response, ureq::Error> {
    let mut resp = agent().get(url).call()?;
    Ok(Response {
        status: resp.status().as_u16(),
        body: resp.body_mut().read_to_string()?,
    })
}

enum Backend {
    /// The binary started as a child process of the test.
    Local(Child),
    /// The binary running inside a container; holds the container id.
    Docker(String),
}

/// A running server. Stopped gracefully (SIGTERM) on `shutdown` or drop, so
/// the instrumented binary gets to write its coverage profile.
pub struct Server {
    base_url: String,
    backend: Option<Backend>,
}

impl Server {
    /// Spawn the binary on a random local port.
    pub fn local() -> Self {
        let mut child = Command::new(bin_path())
            .args(["serve", "--host", "127.0.0.1", "--port", "0"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn server");

        // The server prints "listening on <addr>" once it is ready.
        let mut line = String::new();
        BufReader::new(child.stdout.take().expect("piped stdout"))
            .read_line(&mut line)
            .expect("read server stdout");
        let addr = line
            .trim()
            .strip_prefix("listening on ")
            .unwrap_or_else(|| panic!("unexpected server output: {line:?}"))
            .to_string();

        Server {
            base_url: format!("http://{addr}"),
            backend: Some(Backend::Local(child)),
        }
    }

    /// Run `image` in a container, publishing the port on a random host port.
    ///
    /// If `COVERAGE_DIR` is set, it is bind-mounted into the container and
    /// `LLVM_PROFILE_FILE` points into it, so the profile written on exit
    /// lands on the host where `cargo llvm-cov report` looks for it.
    pub fn docker(image: &str) -> Self {
        let mut cmd = Command::new("docker");
        cmd.args(["run", "--detach", "--publish"])
            .arg(format!("127.0.0.1::{CONTAINER_PORT}"));

        if let Ok(dir) = std::env::var("COVERAGE_DIR") {
            // Inside a container the server is always PID 1, so `%p` is not
            // unique across containers: put a unique id in the name instead.
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let id = format!(
                "{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            cmd.arg("--volume")
                .arg(format!("{dir}:/coverage"))
                .arg("--env")
                .arg(format!(
                    "LLVM_PROFILE_FILE=/coverage/{BIN_NAME}-docker-{id}-%p.profraw"
                ))
                // Write the profile as the host user so it can be cleaned up.
                .arg("--user")
                .arg(format!("{}:{}", id_of("-u"), id_of("-g")));
        }

        let port = CONTAINER_PORT.to_string();
        cmd.arg(image)
            .args(["serve", "--host", "0.0.0.0", "--port", &port]);
        let container = docker_output(&mut cmd);

        let mapping = docker_output(Command::new("docker").args([
            "port",
            &container,
            &format!("{CONTAINER_PORT}/tcp"),
        ]));
        // `docker port` may print one line per address family.
        let addr = mapping.lines().next().expect("port mapping").to_string();

        let server = Server {
            base_url: format!("http://{addr}"),
            backend: Some(Backend::Docker(container)),
        };
        server.wait_ready(Duration::from_secs(30));
        server
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn wait_ready(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let url = self.url("/health");
        loop {
            if matches!(try_get(&url), Ok(r) if r.status == 200) {
                return;
            }
            assert!(Instant::now() < deadline, "server not ready at {url}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Stop the server with SIGTERM and return its exit code. A clean exit
    /// (code 0) means the profiling runtime had the chance to write its data.
    pub fn shutdown(mut self) -> i32 {
        stop(self.backend.take().expect("server already stopped"))
    }

    /// Kill the server with SIGKILL, the way `Child::kill` or `docker kill`
    /// would. The process gets no chance to run exit hooks, so everything it
    /// executed is MISSING from the coverage report.
    pub fn kill(mut self) {
        match self.backend.take().expect("server already stopped") {
            Backend::Local(mut child) => {
                child.kill().expect("kill server");
                child.wait().expect("wait for server");
            }
            Backend::Docker(id) => {
                docker_output(Command::new("docker").args(["kill", &id]));
                docker_output(Command::new("docker").args(["rm", &id]));
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Fallback for tests that panic before calling `shutdown`.
        if let Some(backend) = self.backend.take() {
            stop(backend);
        }
    }
}

fn stop(backend: Backend) -> i32 {
    match backend {
        Backend::Local(mut child) => {
            let status = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status()
                .expect("run kill");
            assert!(status.success(), "failed to send SIGTERM");
            child.wait().expect("wait for server").code().unwrap_or(-1)
        }
        Backend::Docker(id) => {
            // `docker stop` sends SIGTERM and only falls back to SIGKILL after
            // the timeout, which would lose the coverage data.
            docker_output(Command::new("docker").args(["stop", "--timeout", "10", &id]));
            let code = docker_output(Command::new("docker").args([
                "inspect",
                "--format",
                "{{.State.ExitCode}}",
                &id,
            ]));
            docker_output(Command::new("docker").args(["rm", &id]));
            code.parse().unwrap_or(-1)
        }
    }
}

fn docker_output(cmd: &mut Command) -> String {
    let out = cmd.output().expect("failed to run docker");
    assert!(
        out.status.success(),
        "{cmd:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn id_of(flag: &str) -> String {
    let out = Command::new("id").arg(flag).output().expect("run id");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}
