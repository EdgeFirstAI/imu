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

2. **Publish Pipeline** (`publisher.rs`)
   - `ImuSample`: one fixed-size, `Copy` reading handed between threads
   - `SampleQueue`: lock-free bounded queue, oldest sample discarded when full
   - `BufferPool`: pre-encoded CDR messages recycled across publications
   - `write_sample()`: patches the four varying fields of an encoded message in place

3. **Data Processing**
   - Quaternion orientation from rotation vector reports
   - Angular velocity from gyroscope reports
   - Linear acceleration from accelerometer reports
   - Sensor fusion algorithms (performed by BNO08x hardware)

4. **Output Generation**
   - CDR-serialized IMU messages via `edgefirst-schemas`
   - Zenoh topic publishing
   - Configurable topic names

### ROS 2 Year 2038 Limit

The ROS 2 `builtin_interfaces/msg/Time` message uses `int32` for the `sec` field, which overflows on 2038-01-19T03:14:07Z.

IMU handles this as follows:
1. `timestamp()` detects when seconds exceed `i32::MAX` and returns `TimestampError::Overflow`.
2. The caller logs a warning and publishes with a saturated timestamp (`sec = i32::MAX`, `nanosec = 999_999_999`).
3. IMU sensor data (orientation, angular velocity, linear acceleration) is still published — only the header timestamp is clamped.

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

Sampling and publishing run on separate threads so that Zenoh latency never
delays the thread servicing the sensor interrupt. The BNO085/086 times out,
retries and eventually starves its own processing when the host is slower
than roughly 1/10 of the fastest report period, so the sensor thread must do
as little as possible.

```
      sampling thread              shared              publish thread

BNO08x → SPI → Driver → ImuSample ──▶ SampleQueue ──▶ patch CDR ──▶ Zenoh
 reports  IRQ   fusion   (Copy,        (lock-free,     buffer in     imu
                          no alloc)     drop oldest)   place         topic
```

The sensor callback only copies values into the queue. The publish thread
parks while the queue is empty and is unparked on each push, so it costs no
CPU between samples and adds no latency to the ones that arrive.

### Buffer Reuse

Zenoh takes ownership of a payload, so a single buffer cannot be mutated and
republished: the previous publication may still reference it. `BufferPool`
therefore keeps a few `Bytes` messages and reclaims one with
`Bytes::try_into_mut`, which succeeds once Zenoh has released its reference.

Each publication:

1. Takes a recycled buffer already holding a valid encoded message.
2. Overwrites only the stamp, orientation, angular velocity and linear
   acceleration. The frame ID and the three covariance arrays never change.
3. Freezes it to `Bytes`, which Zenoh accepts with no copy and no allocation.
4. Returns it to the pool once the publication has been handed over.

In steady state this recycles a single allocation: a 9-second run at 111 Hz
published 1016 messages having allocated one buffer.

### Backpressure

The queue holds `SAMPLE_QUEUE_DEPTH` (64) samples, roughly a quarter second
at the rotation vector rate. If publishing stalls for longer, the oldest
samples are discarded rather than blocking the sensor thread: consumers care
about current attitude, not history. Discards are counted and logged when the
run ends.

### Watchdog

The `--timeout` watchdog measures time since the last sample was *read from
the sensor*, not since the last message was published. A slow subscriber or a
stalled network therefore cannot trigger a sensor reset.

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
