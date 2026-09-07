// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{CommandFactory, Parser};
use serde_json::json;
use tracing::level_filters::LevelFilter;
use zenoh::config::{Config, WhatAmI};

/// Command-line arguments for EdgeFirst IMU Node.
///
/// This structure defines all configuration options for the IMU node,
/// including device paths, Zenoh configuration, and debugging options.
/// Arguments can be specified via command line or environment variables.
///
/// # Example
///
/// ```bash
/// # Via command line
/// edgefirst-imu --timeout 200 --topic imu
///
/// # Via environment variables
/// export TIMEOUT=200
/// export MODE=client
/// edgefirst-imu
/// ```
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// IMU times out after not recieving a message for this many
    /// milliseconds,
    #[arg(long, env = "TIMEOUT", default_value = "165")]
    pub timeout: u64,

    /// Specify the path to the spidevice.
    #[arg(long, default_value = "/dev/spidev1.0")]
    pub device: String,

    /// Specify the interrupt pin.
    #[arg(long, default_value = "IMU_INT")]
    pub interrupt: String,

    /// Specify the reset pin.
    #[arg(long, default_value = "IMU_RST")]
    pub reset: String,

    /// Apply the Maivin2 FRS Configuration.
    #[arg(long)]
    pub configure: bool,

    /// Zenoh key expression for IMU messages (sensor_msgs/Imu).
    /// The session namespace prefixes this with `{hostname}/` on the wire.
    #[arg(long, default_value = "imu")]
    pub topic: String,

    /// Application log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    pub rust_log: LevelFilter,

    /// Enable Tracy profiler broadcast
    #[arg(long, env = "TRACY")]
    pub tracy: bool,

    /// Zenoh participant mode (peer, client, or router)
    #[arg(long, env = "MODE", default_value = "peer")]
    mode: WhatAmI,

    /// Zenoh endpoints to connect to (can specify multiple)
    #[arg(long, env = "CONNECT")]
    connect: Vec<String>,

    /// Zenoh endpoints to listen on (can specify multiple)
    #[arg(long, env = "LISTEN")]
    listen: Vec<String>,

    /// Disable Zenoh multicast peer discovery
    #[arg(long, env = "NO_MULTICAST_SCOUTING")]
    no_multicast_scouting: bool,
}

/// Environment variables where an empty value is meaningful and must be preserved
/// (i.e. the argument has a non-empty default but "" is a documented "disable" sentinel).
///
/// The IMU service has no such variables: `CONNECT` and `LISTEN` have no
/// default, so scrubbing them to "unset" is exactly the intended meaning.
pub const KEEP: &[&str] = &[];

/// Names of this program's env-bound arguments whose value, as reported by
/// `var`, is present but empty and not listed in `keep`.
///
/// Pure: the environment is only read through `var`, so this can be unit
/// tested with a fake lookup and no process-wide mutation.
pub fn empty_env_vars<C: CommandFactory>(
    keep: &[&str],
    var: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    C::command()
        .get_arguments()
        .filter_map(|arg| arg.get_env().map(|e| e.to_string_lossy().into_owned()))
        .filter(|name| !keep.contains(&name.as_str()))
        .filter(|name| var(name).is_some_and(|v| v.is_empty()))
        .collect()
}

/// Treat an empty environment variable as unset, so clap's declared
/// `default_value` applies instead of failing to parse.
///
/// Only variables bound to this program's own arguments are considered;
/// unrelated process environment is left alone. `keep` names variables
/// where an empty value is meaningful and must be preserved.
///
/// # Safety
/// Must be called before any thread is spawned. Mutating the process
/// environment is not thread-safe.
pub unsafe fn scrub_empty_env<C: CommandFactory>(keep: &[&str]) {
    for name in empty_env_vars::<C>(keep, |name| std::env::var(name).ok()) {
        std::env::remove_var(&name);
    }
}

/// System hostname used as the Zenoh session namespace.
///
/// Empty or `/`-containing hostnames would create unintended sub-keys, so we
/// fall back to `"localhost"` and warn. Two devices both falling back would
/// silently share a namespace; that is a deployment defect.
fn zenoh_namespace() -> String {
    let raw = gethostname::gethostname().to_string_lossy().into_owned();
    if raw.is_empty() || raw.contains('/') {
        tracing::warn!(
            hostname = %raw,
            "system hostname is empty or contains '/' — falling back to \"localhost\""
        );
        "localhost".into()
    } else {
        raw
    }
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        let mut config = Config::default();

        // Session namespace = hostname: application keys are bare (`imu`)
        // and the wire form is `{hostname}/imu`.
        config
            .insert_json5("namespace", &json!(zenoh_namespace()).to_string())
            .unwrap();

        config
            .insert_json5("mode", &json!(args.mode).to_string())
            .unwrap();

        let connect: Vec<_> = args.connect.into_iter().filter(|s| !s.is_empty()).collect();
        if !connect.is_empty() {
            config
                .insert_json5("connect/endpoints", &json!(connect).to_string())
                .unwrap();
        }

        let listen: Vec<_> = args.listen.into_iter().filter(|s| !s.is_empty()).collect();
        if !listen.is_empty() {
            config
                .insert_json5("listen/endpoints", &json!(listen).to_string())
                .unwrap();
        }

        if args.no_multicast_scouting {
            config
                .insert_json5("scouting/multicast/enabled", &json!(false).to_string())
                .unwrap();
        }

        config
            .insert_json5("scouting/multicast/interface", &json!("lo").to_string())
            .unwrap();

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::collections::HashMap;

    /// Env-bound arguments with a non-empty default where we have consciously decided
    /// that an empty value is NOT meaningful (so scrubbing to the default is correct).
    const SCRUB_REVIEWED: &[&str] = &["TIMEOUT", "RUST_LOG", "MODE"];

    #[test]
    fn every_env_arg_is_either_scrubbable_or_explicitly_kept() {
        for arg in Args::command().get_arguments() {
            let Some(env) = arg.get_env() else { continue };
            let name = env.to_string_lossy().into_owned();
            let has_nonempty_default = arg
                .get_default_values()
                .first()
                .is_some_and(|d| !d.is_empty());
            if has_nonempty_default && !KEEP.contains(&name.as_str()) {
                assert!(
                    SCRUB_REVIEWED.contains(&name.as_str()),
                    "{name} has a non-empty default; decide whether empty is meaningful \
                     and add it to KEEP or SCRUB_REVIEWED"
                );
            }
        }
    }

    /// Fake environment lookup so the decision logic is tested without
    /// touching the process environment.
    fn lookup(env: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn empty_env_var_is_listed() {
        let found = empty_env_vars::<Args>(KEEP, lookup(&[("TIMEOUT", "")]));
        assert_eq!(found, ["TIMEOUT"]);
    }

    #[test]
    fn non_empty_env_var_is_not_listed() {
        let found = empty_env_vars::<Args>(KEEP, lookup(&[("TIMEOUT", "200")]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn unset_env_var_is_not_listed() {
        let found = empty_env_vars::<Args>(KEEP, lookup(&[]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn kept_env_var_is_not_listed_even_when_empty() {
        let found = empty_env_vars::<Args>(&["TIMEOUT"], lookup(&[("TIMEOUT", ""), ("MODE", "")]));
        assert_eq!(found, ["MODE"]);
    }

    #[test]
    fn unbound_env_var_is_never_listed() {
        let found = empty_env_vars::<Args>(KEEP, lookup(&[("UNRELATED_EMPTY_VAR", "")]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn all_empty_env_bound_vars_are_listed() {
        let mut found = empty_env_vars::<Args>(
            KEEP,
            lookup(&[
                ("TIMEOUT", ""),
                ("RUST_LOG", ""),
                ("MODE", ""),
                ("NO_MULTICAST_SCOUTING", ""),
                ("TRACY", "1"),
            ]),
        );
        found.sort();
        assert_eq!(
            found,
            ["MODE", "NO_MULTICAST_SCOUTING", "RUST_LOG", "TIMEOUT"]
        );
    }

    #[test]
    fn zenoh_config_sets_namespace() {
        let args = Args::parse_from(["edgefirst-imu"]);
        let cfg = Config::from(args);
        let ns: String = serde_json::from_str(&cfg.to_string())
            .ok()
            .and_then(|v: serde_json::Value| {
                v.pointer("/namespace")
                    .and_then(|n| n.as_str().map(String::from))
            })
            .expect("namespace should be set in config");
        assert!(!ns.is_empty(), "namespace should be non-empty");
        assert!(!ns.contains('/'), "namespace must not contain '/'");
    }

    #[test]
    fn default_topic_has_no_rt_prefix() {
        let args = Args::parse_from(["edgefirst-imu"]);
        assert_eq!(args.topic, "imu");
    }
}
