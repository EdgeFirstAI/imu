# Changelog

All notable changes to EdgeFirst IMU will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.3.0] - 2026-09-01

Requires bno08x-rs 3.0.0, which fixes the SPI transport defects behind
EDGEAI-1100 and EDGEAI-520. On verdin-imx8mp-15141091 the service went from
roughly 25 restarts per ten minutes to none.

### Changed

- Upgrade `bno08x-rs` 2.0.1 to 3.0.0. The first report is now enabled
  immediately after `init()` with no sleep in between, because the sensor hub
  sleeps as soon as its startup packets are drained and cannot be woken over
  SPI.
- Sampling and publishing now run on separate threads. The sensor callback
  only copies a fixed-size sample into a lock-free queue, so Zenoh latency no
  longer delays servicing the sensor interrupt (which the BNO085/086 penalises
  by timing out and starving its own processing).
- IMU messages are published from recycled, pre-encoded CDR buffers. Only the
  stamp, orientation, angular velocity and linear acceleration are rewritten
  per sample; the frame ID and covariances are encoded once. Buffers are
  reclaimed with `Bytes::try_into_mut` and handed to Zenoh with no copy and no
  allocation, so a steady-state run reuses a single allocation.
- The Zenoh publisher is declared once at startup instead of calling
  `session.put()` per message.
- The `--timeout` watchdog now measures time since the last sample was read
  from the sensor rather than since the last successful publish, so a slow
  subscriber or a stalled network can no longer trigger a sensor reset. Tune
  `TIMEOUT` against sensor behaviour rather than publish latency.
- When publishing stalls, the oldest queued samples are discarded instead of
  blocking the sampling thread; discards are counted and logged when the run
  ends.
- A run now ends if the publisher thread cannot publish at all, instead of
  sampling indefinitely with nothing reaching the wire.

### Added

- `bytes` and `crossbeam-queue` dependencies for the publish buffer pool and
  the sample queue.

## [3.2.0] - 2026-08-31

### Changed

- Attach a Zenoh source timestamp on every `imu` sample so the recorder
  can use publisher time instead of receive time.
- Set the Zenoh session namespace to the system hostname and publish on
  `imu` instead of `rt/imu`. Wire keys are `{hostname}/imu` (EDGEAI-1396).
- Upgrade `edgefirst-schemas` 1.5.1 → 4.0.0. IMU messages are built with
  `Imu::builder()` and encoded via `into_cdr()`; `serde_cdr` is removed.

### Fixed

- Skip IMU samples that fail CDR encode instead of aborting the publish
  path (EDGEAI-1422).

## [3.1.0] - 2026-03-23

### Fixed

- Use `CLOCK_REALTIME` (wall-clock time) for IMU header timestamps, following the ROS 2
  convention where `rclcpp::Node::now()` returns system time by default
- Make `timestamp()` return `Result` with `TimestampError` enum to handle pre-epoch clock
  and Y2038 overflow instead of panicking or silently wrapping
- CI: only suppress nextest exit code 4 ("no tests to run") instead of masking all failures

### Added

- `TimestampError` enum with `BeforeEpoch` and `Overflow` variants, mirroring the navsat
  implementation
- ROS 2 Year 2038 Limit section in ARCHITECTURE.md
- Timestamp wall-clock proximity assertion in integration tests
- Repository AI assistant instructions (`.github/copilot-instructions.md`)

## [3.0.5] - 2026-03-01

### Fixed

- Handle empty CONNECT/LISTEN environment variables without panicking. The
  default config ships with `CONNECT=""` and `LISTEN=""` which produced a
  single empty-string endpoint, causing Zenoh's endpoint parser to panic.

## [3.0.4] - 2026-02-26

### Changed

- Use short environment variable names in imu.default configuration
- Pin all GitHub Actions to SHA hashes for security (prevents tag mutation attacks)
- Enforce SBOM requirement in release workflow (fails if sbom.json missing)

### Removed

- Remove legacy bitbucket-pipelines.yml

## [3.0.2] - 2026-01-27

### Changed

- Officially switched license to Apache-2.0

## [3.0.1] - 2026-01-26

### Changed

- Initial crates.io release with trusted publisher support

## [3.0.0] - 2026-01-26

### Changed

- **BREAKING**: Renamed package from `maivin-imu` to `edgefirst-imu`
- **BREAKING**: Updated to bno08x-rs 2.0.1 with new API
  - `BNO08x::new_bno08x_from_symbol()` → `BNO08x::new_spi_from_symbol()`
  - Import path changed from `bno08x::wrapper::*` to `bno08x_rs::*`
- Updated to use `edgefirst_schemas::serde_cdr` for CDR serialization
- Migrated from Bitbucket to GitHub
- Updated license to Apache-2.0
- Updated all dependencies to latest versions:
  - zenoh 1.7.2
  - clap 4.5.54
  - edgefirst-schemas 1.5.0
  - tracing 0.1.44

### Added

- Initial open source release
- crates.io publishing support (`cargo install edgefirst-imu`)
- Pre-built binaries for x86_64 and aarch64 in GitHub releases
- Tracy profiling support
- Zenoh integration for distributed communication
- GitHub Actions CI/CD workflows

### Security

- Added security policy and vulnerability reporting process

## [2.2.0] - 2025-11-27

### Changed

- Updated bno08x dependency to use GitHub repository
- Ported to Zenoh 1.2

### Added

- Basic instrumentation support
- Longer timeout for first IMU message (5x normal timeout)

## [2.1.3] - 2025-10-15

### Changed

- Use Duration type instead of millisecond constants
- Use monotonic clock for message timestamps

## [2.1.2] - 2025-09-20

### Changed

- Updated bno08x driver to use master branch

## [2.1.1] - 2025-08-10

### Changed

- Renamed project to maivin-imu
- Applied Clippy fixes

## [2.1.0] - 2025-07-15

### Added

- Better error handling for FRS configuration
- Automatic IMU restart on timeout (up to 3 retries)
- Environment logger for debugging

### Changed

- Rotation vector update rate changed to 33ms
- Default message timeout set to 165ms

[Unreleased]: https://github.com/EdgeFirstAI/imu/compare/v3.3.0...HEAD
[3.3.0]: https://github.com/EdgeFirstAI/imu/compare/v3.2.0...v3.3.0
[3.2.0]: https://github.com/EdgeFirstAI/imu/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/EdgeFirstAI/imu/compare/v3.0.5...v3.1.0
[3.0.5]: https://github.com/EdgeFirstAI/imu/compare/v3.0.4...v3.0.5
[3.0.4]: https://github.com/EdgeFirstAI/imu/compare/v3.0.2...v3.0.4
[3.0.2]: https://github.com/EdgeFirstAI/imu/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/EdgeFirstAI/imu/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/EdgeFirstAI/imu/compare/v2.2.0...v3.0.0
[2.2.0]: https://github.com/EdgeFirstAI/imu/compare/v2.1.3...v2.2.0
[2.1.3]: https://github.com/EdgeFirstAI/imu/compare/v2.1.2...v2.1.3
[2.1.2]: https://github.com/EdgeFirstAI/imu/compare/v2.1.1...v2.1.2
[2.1.1]: https://github.com/EdgeFirstAI/imu/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/EdgeFirstAI/imu/releases/tag/v2.1.0
