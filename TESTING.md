# Testing

This document describes the testing strategy for EdgeFirst IMU, including automated
CI testing and manual on-target testing procedures.

## Overview

EdgeFirst IMU uses a three-tier testing approach:

1. **Unit Tests**: Run on GitHub-hosted runners (x86_64 and aarch64)
2. **Static Analysis**: Format checking and Clippy linting
3. **Hardware Integration Tests**: Run on real hardware with BNO08x IMU sensor

## Prerequisites

Before testing the IMU service, ensure the underlying BNO08x driver works correctly.
Follow the testing instructions in the [bno08x-rs TESTING.md](../bno08x-rs/TESTING.md)
to verify basic sensor communication and data integrity.

## Automated Testing (CI)

### Test Workflow Architecture

The test workflow (`.github/workflows/test.yml`) implements a three-phase architecture:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Phase 1: Build & Unit Test                  │
│  ┌─────────────────────┐        ┌─────────────────────┐            │
│  │   ubuntu-22.04      │        │  ubuntu-22.04-arm   │            │
│  │   (x86_64)          │        │  (aarch64)          │            │
│  │                     │        │                     │            │
│  │  • Unit tests       │        │  • Unit tests       │            │
│  │  • Coverage         │        │  • Coverage         │            │
│  │                     │        │  • Build instrumented│            │
│  │                     │        │    binaries         │            │
│  └─────────────────────┘        └──────────┬──────────┘            │
└─────────────────────────────────────────────┼───────────────────────┘
                                              │ artifacts
                                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 2: Hardware Integration Tests              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     raivin (self-hosted)                     │   │
│  │                                                              │   │
│  │  • Download instrumented binaries                           │   │
│  │  • Run integration tests with real BNO08x IMU               │   │
│  │  • Collect coverage profraw files                           │   │
│  │                                                              │   │
│  └──────────────────────────────┬───────────────────────────────┘   │
└─────────────────────────────────┼───────────────────────────────────┘
                                  │ profraw files
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Phase 3: Process Coverage                        │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   ubuntu-22.04-arm                           │   │
│  │                                                              │   │
│  │  • Merge profraw files                                      │   │
│  │  • Generate LCOV coverage report                            │   │
│  │  • Upload to SonarCloud                                     │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Runner Types

| Runner | Architecture | Type | Purpose |
|--------|--------------|------|---------|
| `ubuntu-22.04` | x86_64 | GitHub-hosted | Unit tests, linting, SonarCloud |
| `ubuntu-22.04-arm` | aarch64 | GitHub-hosted | Unit tests, build instrumented binaries |
| `raivin` | aarch64 | Self-hosted | Hardware integration tests with BNO08x |

### Hardware Test Triggers

Hardware integration tests run automatically when:

- Code is pushed to `main` branch
- A pull request has the `test-hardware` label applied

### Coverage Collection

Coverage is collected using `cargo-llvm-cov` with source-based instrumentation:

1. **Unit test coverage**: Collected on both x86_64 and aarch64 runners
2. **Hardware test coverage**: Collected on the `raivin` runner using instrumented binaries
3. **Combined reporting**: All coverage is merged and reported to SonarCloud

## Integration Tests

The integration tests in `tests/integration_test.rs` verify:

### `test_imu_publishing`

- Starts the `edgefirst-imu` service
- Subscribes to the `rt/imu` Zenoh topic
- Collects messages for 5 seconds
- Validates quaternion normalization (magnitude ≈ 1.0)
- Verifies message rate ≥ 50 Hz
- Gracefully stops the service

### `test_graceful_shutdown`

- Starts the `edgefirst-imu` service
- Sends SIGTERM signal
- Verifies the service exits cleanly within 5 seconds

## Manual Testing

### Hardware Requirements

- Linux system with SPI and GPIO support (e.g., Maivin, Raspberry Pi)
- BNO08x IMU sensor connected via SPI
- GPIO pins for interrupt and reset signals

### Building for Target

```bash
# Build release binary
cargo build --release

# Or build with profiling support (includes debug symbols)
cargo build --profile profiling
```

### Copying to Target

```bash
# Copy the binary to the target device
scp target/release/edgefirst-imu user@target:/usr/local/bin/

# Copy the test binary (for integration tests)
scp target/release/deps/integration_test-* user@target:~/
```

### Running the Service Manually

```bash
# Basic usage with default settings
edgefirst-imu

# Specify SPI device and GPIO pins
edgefirst-imu --device /dev/spidev1.0 --interrupt IMU_INT --reset IMU_RST

# With custom Zenoh topic
edgefirst-imu --topic rt/imu

# Enable Tracy profiling
edgefirst-imu --tracy
```

### Verifying IMU Output

Use a Zenoh subscriber to verify IMU messages:

```bash
# Install zenoh-plugin-rest for command-line tools
cargo install zenoh

# Subscribe to IMU topic
z_sub -k "rt/imu"
```

Or use the EdgeFirst CLI:

```bash
edgefirst-client subscribe rt/imu --format json
```

### Running Integration Tests Manually

```bash
# Set the binary location
export IMU_BINARY=/usr/local/bin/edgefirst-imu

# Run all integration tests (including ignored/hardware tests)
./integration_test-* --include-ignored --test-threads=1

# Run a specific test
./integration_test-* --include-ignored test_imu_publishing
```

### Expected Output

Successful integration test output:

```
Starting IMU service: /usr/local/bin/edgefirst-imu
Collecting IMU messages for 5s...
Received 487 messages in 5s
Message rate: 97.4 Hz
Sending SIGTERM to IMU service (pid: 12345)
IMU service exited with status: ExitStatus(unix_wait_status(0))
✓ Integration test passed!
```

## Troubleshooting

### No IMU Messages Received

1. Verify BNO08x driver works: Follow [bno08x-rs TESTING.md](../bno08x-rs/TESTING.md)
2. Check SPI device exists: `ls -la /dev/spidev*`
3. Verify GPIO pins are correct: Check device tree configuration
4. Review service logs: `journalctl -u edgefirst-imu -f`

### Low Message Rate

1. Check for SPI communication errors in logs
2. Verify no other process is accessing the SPI device
3. Check CPU usage - high load can cause timing issues

### Service Won't Start

1. Verify binary has execute permissions: `chmod +x edgefirst-imu`
2. Check for missing libraries: `ldd edgefirst-imu`
3. Run with environment logging: `RUST_LOG=debug edgefirst-imu`

### Graceful Shutdown Fails

1. Ensure SIGTERM handler is working
2. Check for blocked I/O operations
3. Verify Zenoh session is closing properly

## Test Coverage Goals

| Category | Target | Notes |
|----------|--------|-------|
| Unit tests | 70% | Basic code paths |
| Integration tests | Critical paths | Hardware-dependent functionality |
| Combined | 80%+ | Including hardware coverage |

## See Also

- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines
- [README.md](README.md) - Usage documentation
- [bno08x-rs TESTING.md](../bno08x-rs/TESTING.md) - BNO08x driver testing
