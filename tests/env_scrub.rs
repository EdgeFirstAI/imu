// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end check that `KEY=""` in the environment behaves as unset.
//!
//! Runs with `harness = false` so this `main` is the only thread in the
//! process when the environment is mutated, which `scrub_empty_env` requires.
// args.rs's own #[cfg(test)] unit tests are compiled but never run here, and
// without the libtest harness their #[test] fns (and imports) are stripped.
#![allow(dead_code, unused_imports)]

#[path = "../src/args.rs"]
mod args;
use args::{scrub_empty_env, Args, KEEP};
use clap::Parser;
use tracing::level_filters::LevelFilter;
use zenoh::config::Config;

const VARS: [&str; 4] = ["TIMEOUT", "RUST_LOG", "MODE", "NO_MULTICAST_SCOUTING"];
const ARGV: [&str; 1] = ["edgefirst-imu"];

fn main() {
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
