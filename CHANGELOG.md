# Changelog

All notable changes to EdgeFirst IMU will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/EdgeFirstAI/imu/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/EdgeFirstAI/imu/compare/v2.2.0...v3.0.0
[2.2.0]: https://github.com/EdgeFirstAI/imu/compare/v2.1.3...v2.2.0
[2.1.3]: https://github.com/EdgeFirstAI/imu/compare/v2.1.2...v2.1.3
[2.1.2]: https://github.com/EdgeFirstAI/imu/compare/v2.1.1...v2.1.2
[2.1.1]: https://github.com/EdgeFirstAI/imu/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/EdgeFirstAI/imu/releases/tag/v2.1.0
