# Architecture

## Overview

EdgeFirst IMU is a Zenoh-based service that provides Inertial Measurement Unit (IMU) data from BNO08x sensors for the EdgeFirst Maivin platform.

## System Architecture

### Service Design

The IMU service operates as a standalone binary with the following responsibilities:

- Initialize and configure BNO08x sensor via SPI
- Read sensor data at configured rates
- Publish IMU messages to Zenoh topics
- Handle graceful shutdown on SIGTERM

### Key Components

1. **Driver Layer** (`driver.rs`)
   - BNO08x sensor interface via `bno08x-rs` crate
   - SPI communication
   - Sensor initialization and configuration
   - Rotation vector and sensor data reading

2. **Data Processing**
   - Quaternion orientation from rotation vector reports
   - Angular velocity from gyroscope reports
   - Linear acceleration from accelerometer reports
   - Sensor fusion algorithms (performed by BNO08x hardware)

3. **Output Generation**
   - CDR-serialized IMU messages via `edgefirst-schemas`
   - Zenoh topic publishing
   - Configurable topic names

### Configuration (`args.rs`)

Configuration via command-line arguments and environment variables:

| Option | Environment | Default | Description |
|--------|-------------|---------|-------------|
| `--device` | `IMU_DEVICE` | `/dev/spidev1.0` | SPI device path |
| `--interrupt` | `IMU_INTERRUPT` | `IMU_INT` | GPIO interrupt pin |
| `--reset` | `IMU_RESET` | `IMU_RST` | GPIO reset pin |
| `--topic` | `IMU_TOPIC` | `imu` | Zenoh topic for IMU data |
| `--timeout` | `IMU_TIMEOUT` | `165` | Message timeout (ms) |
| `--configure` | - | `false` | Configure FRS and exit |
| `--tracy` | - | `false` | Enable Tracy profiling |

## Communication

### Zenoh Integration

The IMU service uses Zenoh for distributed communication, enabling:

- Low-latency data distribution
- Efficient network utilization
- Integration with EdgeFirst ecosystem
- Topic-based publish/subscribe

### Message Format

IMU messages use the `sensor_msgs::IMU` schema from `edgefirst-schemas`, serialized with CDR:

```rust
pub struct IMU {
    pub header: Header,
    pub orientation: Quaternion,
    pub orientation_covariance: [f64; 9],
    pub angular_velocity: Vector3,
    pub angular_velocity_covariance: [f64; 9],
    pub linear_acceleration: Vector3,
    pub linear_acceleration_covariance: [f64; 9],
}
```

### Data Flow

```
BNO08x Sensor → SPI Interface → Driver → CDR Serialization → Zenoh Publisher
     ↓              ↓            ↓              ↓                  ↓
  Hardware      /dev/spidevX   Fusion      IMU Message        rt/imu topic
  Reports       GPIO IRQ/RST   Processing   Creation          Distribution
```

## Performance

### Tracy Profiling

The IMU service includes Tracy profiling support for:

- Real-time performance monitoring
- Timing analysis of sensor reads
- Publish latency measurement
- Bottleneck identification

Enable with `--tracy` flag and connect with Tracy profiler.

### Sensor Capabilities

- 9-axis sensor fusion (accelerometer, gyroscope, magnetometer)
- On-chip sensor fusion and calibration (BNO08x handles fusion)
- Rotation vector at ~30 Hz (33ms update rate)
- Low power consumption

### Timing

| Operation | Typical Duration |
|-----------|-----------------|
| Sensor read | < 1 ms |
| CDR serialization | < 100 µs |
| Zenoh publish | < 500 µs |
| Total loop | ~33 ms (30 Hz) |

## Error Handling

### Automatic Recovery

- Automatic IMU restart on timeout (up to 3 retries)
- Longer timeout for first IMU message (5x normal)
- Graceful degradation on sensor errors

### Signal Handling

- SIGTERM triggers graceful shutdown
- Zenoh session cleanup
- Clean process exit

## CI/CD Architecture

### Workflow Dependencies

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  build.yml   │     │   test.yml   │     │   sbom.yml   │
│              │     │              │     │              │
│ x86_64 build │     │ Unit tests   │     │ SBOM gen     │
│ aarch64 build│     │ Clippy/fmt   │     │ License check│
│              │     │ HW tests     │     │              │
│ → artifacts  │     │ → coverage   │     │ → sbom.json  │
└──────┬───────┘     └──────────────┘     └──────┬───────┘
       │                                         │
       │         ┌──────────────┐               │
       └────────►│ release.yml  │◄──────────────┘
                 │              │
                 │ Wait for CI  │
                 │ Download     │
                 │ GH Release   │
                 │ crates.io    │
                 └──────────────┘
```

### On-Target Testing

Hardware integration tests run on a self-hosted `raivin` runner with:

- Real BNO08x IMU sensor
- Coverage instrumentation via `cargo-llvm-cov`
- Three-phase execution (build → test → coverage processing)

See [TESTING.md](TESTING.md) for details.

## Dependencies

### Runtime Dependencies

- `bno08x-rs`: BNO08x sensor driver
- `zenoh`: Distributed communication
- `edgefirst-schemas`: Message definitions and CDR serialization
- `tracing`: Structured logging
- `tracing-tracy`: Tracy profiler integration

### Build Dependencies

- Rust 1.90+ (constrained by bno08x-rs dependency)
- Linux with SPI/GPIO support (for hardware testing)

## Future Enhancements

- Multiple IMU support
- Enhanced calibration procedures
- Additional sensor modes (game rotation vector, etc.)
- Sensor diagnostics and health monitoring
- WebSocket/REST API for configuration
