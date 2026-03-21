# AI Assistant Instructions

This file provides guidance to AI coding assistants when working with code in this repository.

## Project Overview

EdgeFirst IMU is a Rust service that reads BNO08x IMU sensor data over SPI and publishes fused orientation (quaternion), accelerometer, and gyroscope data as CDR-serialized `sensor_msgs/IMU` messages over Zenoh. It runs on the EdgeFirst Maivin platform (aarch64 Linux with GPIO/SPI).

## Build & Development Commands

After any code change, always run `cargo fmt` and `cargo clippy` before building:

```bash
cargo fmt                      # Format code (always run before committing)
cargo clippy --all-targets --all-features -- -D warnings  # Lint (always run, zero warnings required)
```

### Building

This crate depends on Linux-only libraries (`gpiod-core`, `spidev`) and will not compile with a native `cargo build` on macOS or other non-Linux platforms. Use `cargo zigbuild` for cross-compilation:

```bash
# On Linux (native)
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --profile profiling # Release + debug symbols (for Tracy)

# On macOS or when cross-compiling for the target platform
cargo zigbuild --target aarch64-unknown-linux-gnu            # Debug build
cargo zigbuild --target aarch64-unknown-linux-gnu --release  # Release build
```

Always use `cargo zigbuild --target aarch64-unknown-linux-gnu` when on a non-Linux host or when cross-compiling for the Maivin platform (aarch64).

### Testing

```bash
cargo test                     # Unit tests (no hardware needed)
cargo test --test integration_test -- --include-ignored --test-threads=1  # Hardware integration tests (requires BNO08x)
```

### Makefile Targets

```bash
make format        # Format with nightly rustfmt
make lint          # Clippy strict mode
make test          # Tests with coverage via cargo-llvm-cov + nextest
make pre-release   # format + lint + verify-version + test
make clean         # Remove build artifacts
```

### Coverage

```bash
cargo llvm-cov nextest --all-features --workspace --lcov --output-path target/rust-coverage.lcov
```

Requires `cargo-nextest` and `cargo-llvm-cov` installed.

## Architecture

Three source files in `src/`:

- **`main.rs`** — Entry point. Sets up signal handlers (SIGTERM/SIGINT → graceful shutdown via `SHUTDOWN` AtomicBool), initializes tracing (stdout + journald + optional Tracy), opens a Zenoh session, and runs the main loop with automatic restart (up to 3 consecutive failures). The `run_imu` function creates the driver, enables reports, registers a rotation vector callback that serializes and publishes IMU messages, and monitors for timeouts (5x timeout for first message).

- **`args.rs`** — Clap-based CLI args with env var fallback. Implements `From<Args> for zenoh::Config` to configure Zenoh mode/connect/listen/scouting from the same args struct. Environment variable names are short (e.g., `TIMEOUT`, `MODE`, `CONNECT`) matching the systemd EnvironmentFile format in `imu.default`.

- **`driver.rs`** — Thin wrapper around `BNO08x` from `bno08x-rs`. Initializes the SPI interface with GPIO interrupt/reset pins. Enables rotation vector (5ms), accelerometer (20ms), and gyroscope (20ms) reports with retry logic. Also handles FRS configuration for sensor orientation.

### Key Dependencies

- `bno08x-rs` — BNO08x sensor driver (SPI + GPIO via gpiod)
- `edgefirst-schemas` — CDR serialization and ROS-compatible message types (`sensor_msgs::IMU`)
- `zenoh` — Pub/sub messaging (messages published to configurable topic, default `rt/imu`)
- `tracing-tracy` / `tracy-client` — Optional Tracy profiling (feature-gated, enabled by default)

### Integration Tests

`tests/integration_test.rs` — Hardware-only tests (`#[ignore]`), run on the `raivin` self-hosted runner. Tests launch the binary as a child process, subscribe to Zenoh, validate quaternion normalization and message rate (≥50 Hz), and verify graceful SIGTERM shutdown. Binary location via `IMU_BINARY` env var.

## Conventions

- MSRV: Rust 1.90
- All source files must have the SPDX header: `// Copyright 2025 Au-Zone Technologies Inc.` + `// SPDX-License-Identifier: Apache-2.0`
- Commits must be signed off (`git commit -s`) per DCO
- `rustfmt.toml`: `use_field_init_shorthand = true`, edition 2021
- Max line length: 100 characters
- Branch naming: `feature/<desc>`, `bugfix/<desc>`, `docs/<desc>`
- nextest configured for serial test execution (`test-threads = 1`) since hardware requires exclusive access
- Coverage target: 70% unit, 80%+ combined with hardware tests

### Timestamp Convention

Header stamps in published messages must use `CLOCK_REALTIME` (wall-clock time), following the ROS2 convention where `rclcpp::Node::now()` returns `SYSTEM_TIME` by default. This ensures timestamps are:
- Correlatable with system logs and external systems
- Compatible with rosbag recording
- Human-readable

`CLOCK_MONOTONIC` should only be used for internal duration/interval measurements (e.g., timeout tracking). For sensors that provide monotonic timestamps (e.g., V4L2), convert to wall-clock via a cached `REALTIME - MONOTONIC` offset.

## CI/CD

- **test.yml** — Three-phase: (1) unit tests on x86_64 + aarch64, (2) hardware integration on `raivin` runner (triggered on `main` push or `test-hardware` PR label), (3) coverage merge → SonarCloud
- **build.yml** — Release binaries for x86_64 and aarch64
- **sbom.yml** — CycloneDX SBOM + license compliance check
- **release.yml** — Triggered by `vX.Y.Z` tag; waits for build+sbom, creates GitHub Release, publishes to crates.io

## Feature Flags

- `tracy` (default) — Enables Tracy profiling support; runtime activation requires `--tracy` CLI flag
- `profiling` — Adds sampling and system tracing to Tracy
