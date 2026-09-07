// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end check that `KEY=""` in the environment behaves as unset.
//!
//! Runs with `harness = false` so this `main` is the only thread in the
//! process when the environment is mutated, which `scrub_empty_env` requires.
//!
//! `main` speaks the small subset of the libtest CLI that `cargo test` and
//! `cargo nextest` use to enumerate (`--list --format terse`) and select
//! (`--exact <name>`, `--ignored`, `--skip <pattern>`, positional filters)
//! tests, so the target is discovered and reported like any other test.
// args.rs's own #[cfg(test)] unit tests are compiled but never run here, and
// without the libtest harness their #[test] fns (and imports) are stripped.
#![allow(dead_code, unused_imports)]

#[path = "../src/args.rs"]
mod args;
use args::{scrub_empty_env, Args, KEEP};
use clap::Parser;
use tracing::level_filters::LevelFilter;
use zenoh::config::Config;

/// The single test this binary provides, as reported to the harness.
const TEST_NAME: &str = "empty_env_is_treated_as_unset";

/// Numeric, log-level, enum and bare-flag arguments, all written as `KEY=""`
/// in /etc/default/imu.
const VARS: [&str; 4] = ["TIMEOUT", "RUST_LOG", "MODE", "NO_MULTICAST_SCOUTING"];
const ARGV: [&str; 1] = ["edgefirst-imu"];

/// libtest flags that consume the following argument (or take `=value`) and
/// carry nothing this harness needs.
const VALUE_FLAGS: [&str; 5] = [
    "--test-threads",
    "--format",
    "--logfile",
    "--color",
    "--shuffle-seed",
];

/// What the harness asked this binary to do.
struct Request {
    list: bool,
    ignored: bool,
    exact: bool,
    filters: Vec<String>,
    skips: Vec<String>,
}

fn parse_request(argv: impl IntoIterator<Item = String>) -> Request {
    let mut req = Request {
        list: false,
        ignored: false,
        exact: false,
        filters: Vec::new(),
        skips: Vec::new(),
    };
    let mut argv = argv.into_iter();
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--list" => req.list = true,
            "--ignored" => req.ignored = true,
            "--exact" => req.exact = true,
            "--skip" => req.skips.extend(argv.next()),
            flag if VALUE_FLAGS.contains(&flag) => {
                argv.next();
            }
            flag if flag.starts_with('-') => {
                if let Some(pattern) = flag.strip_prefix("--skip=") {
                    req.skips.push(pattern.to_owned());
                }
            }
            filter => req.filters.push(filter.to_owned()),
        }
    }
    req
}

/// libtest matching: substring by default, equality with `--exact`.
fn matches(req: &Request, pattern: &str) -> bool {
    if req.exact {
        pattern == TEST_NAME
    } else {
        TEST_NAME.contains(pattern)
    }
}

fn selected(req: &Request) -> bool {
    let filtered_in = req.filters.is_empty() || req.filters.iter().any(|f| matches(req, f));
    let skipped = req.skips.iter().any(|s| matches(req, s));
    filtered_in && !skipped
}

fn main() {
    let req = parse_request(std::env::args().skip(1));
    // This binary has no #[ignore]d tests, so `--ignored` selects nothing.
    if req.list {
        if !req.ignored && selected(&req) {
            println!("{TEST_NAME}: test");
        }
        return;
    }
    if req.ignored || !selected(&req) {
        return;
    }

    for name in VARS {
        // Single-threaded: this is `main` before any thread is spawned.
        std::env::set_var(name, "");
    }
    let before = Args::try_parse_from(ARGV);
    assert!(
        before.is_err(),
        "empty vars must fail to parse before scrubbing: {before:?}"
    );

    // SAFETY: still single-threaded; no thread has been spawned in this process.
    unsafe { scrub_empty_env::<Args>(KEEP) };
    for name in VARS {
        assert!(
            std::env::var_os(name).is_none(),
            "{name} should have been removed"
        );
    }

    let args = Args::try_parse_from(ARGV).expect("defaults must apply after scrubbing");
    assert_eq!(args.timeout, 165);
    assert_eq!(args.rust_log, LevelFilter::INFO);

    // `mode` and `no_multicast_scouting` are private to `args`; observe their
    // defaults through the Zenoh config they produce.
    let cfg: serde_json::Value =
        serde_json::from_str(&Config::from(args).to_string()).expect("config is JSON");
    assert_eq!(cfg.pointer("/mode").and_then(|m| m.as_str()), Some("peer"));
    assert_ne!(
        cfg.pointer("/scouting/multicast/enabled")
            .and_then(|e| e.as_bool()),
        Some(false),
        "multicast scouting must not be disabled by default"
    );

    // Non-empty values are left alone and still parsed.
    std::env::set_var("TIMEOUT", "200");
    // SAFETY: still single-threaded.
    unsafe { scrub_empty_env::<Args>(KEEP) };
    assert_eq!(std::env::var("TIMEOUT").as_deref(), Ok("200"));
    let args = Args::try_parse_from(ARGV).expect("valid value should parse");
    assert_eq!(args.timeout, 200);

    println!("env_scrub: ok");
}
