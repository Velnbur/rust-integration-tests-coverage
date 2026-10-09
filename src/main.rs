use std::net::SocketAddr;
use std::process::ExitCode;

mod server;

fn usage() -> ExitCode {
    eprintln!("usage: integration-tests-coverage <add|sub|mul|div> <a> <b>");
    eprintln!("       integration-tests-coverage <reverse|palindrome|words> <text>");
    eprintln!("       integration-tests-coverage serve [--host <host>] [--port <port>]");
    ExitCode::from(2)
}

fn parse(s: &str) -> Option<i64> {
    s.parse().ok()
}

fn run_calc(op: &str, a: &str, b: &str) -> ExitCode {
    let (Some(a), Some(b)) = (parse(a), parse(b)) else {
        eprintln!("error: arguments must be integers");
        return ExitCode::from(2);
    };
    let res = match op {
        "add" => calc::add(a, b),
        "sub" => calc::sub(a, b),
        "mul" => calc::mul(a, b),
        "div" => calc::div(a, b),
        _ => unreachable!(),
    };
    match res {
        Ok(v) => {
            println!("{v}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e:?}");
            ExitCode::FAILURE
        }
    }
}

fn run_serve(args: &[&str]) -> ExitCode {
    let mut host = "127.0.0.1";
    let mut port = "8080";
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match (*flag, it.next()) {
            ("--host", Some(v)) => host = v,
            ("--port", Some(v)) => port = v,
            _ => return usage(),
        }
    }
    let Ok(addr) = format!("{host}:{port}").parse::<SocketAddr>() else {
        eprintln!("error: invalid address {host}:{port}");
        return ExitCode::from(2);
    };
    let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
    match rt.block_on(server::serve(addr)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [op @ ("add" | "sub" | "mul" | "div"), a, b] => run_calc(op, a, b),
        ["reverse", s] => {
            println!("{}", text::reverse(s));
            ExitCode::SUCCESS
        }
        ["palindrome", s] => {
            println!("{}", text::is_palindrome(s));
            ExitCode::SUCCESS
        }
        ["words", s] => {
            println!("{}", text::word_count(s));
            ExitCode::SUCCESS
        }
        ["serve", rest @ ..] => run_serve(rest),
        _ => usage(),
    }
}
