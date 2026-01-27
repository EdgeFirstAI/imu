# EdgeFirst IMU

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Build Status](https://github.com/EdgeFirstAI/imu/actions/workflows/build.yml/badge.svg)](https://github.com/EdgeFirstAI/imu/actions/workflows/build.yml)
[![Test Status](https://github.com/EdgeFirstAI/imu/actions/workflows/test.yml/badge.svg)](https://github.com/EdgeFirstAI/imu/actions/workflows/test.yml)
[![Crates.io](https://img.shields.io/crates/v/edgefirst-imu.svg)](https://crates.io/crates/edgefirst-imu)

Inertial Measurement Unit (IMU) service for EdgeFirst Maivin platform using BNO08x sensors.

## Overview

EdgeFirst IMU provides IMU sensor data from BNO08x sensors, publishing fused orientation
and raw sensor data over Zenoh for robotics and edge AI applications.

## Features

- BNO08x IMU sensor support via SPI
- Sensor fusion with quaternion and rotation vector outputs
- Real-time processing with Tracy profiling support
- Zenoh integration for distributed communication
- Configurable update rates for rotation, accelerometer, and gyroscope

## Installation

### From crates.io

```bash
cargo install edgefirst-imu
```

### From GitHub Releases

Download pre-built binaries from the [releases page](https://github.com/EdgeFirstAI/imu/releases):

```bash
# For x86_64
wget https://github.com/EdgeFirstAI/imu/releases/latest/download/edgefirst-imu-linux-x86_64
chmod +x edgefirst-imu-linux-x86_64

# For ARM64 (Maivin)
wget https://github.com/EdgeFirstAI/imu/releases/latest/download/edgefirst-imu-linux-aarch64
chmod +x edgefirst-imu-linux-aarch64
```

### From Source

```bash
git clone https://github.com/EdgeFirstAI/imu.git
cd imu
cargo build --release
```

## Requirements

- Linux with SPI and GPIO support
- BNO08x IMU sensor hardware
- Rust 1.90 or later (for building from source)

## Usage

```bash
edgefirst-imu --device /dev/spidev1.0 --interrupt IMU_INT --reset IMU_RST
```

### Options

| Option | Environment Variable | Default | Description |
|--------|---------------------|---------|-------------|
| `--device` | `IMU_DEVICE` | `/dev/spidev1.0` | SPI device path |
| `--interrupt` | `IMU_INTERRUPT` | `IMU_INT` | GPIO interrupt pin name |
| `--reset` | `IMU_RESET` | `IMU_RST` | GPIO reset pin name |
| `--topic` | `IMU_TOPIC` | `imu` | Zenoh topic for IMU data |
| `--timeout` | `IMU_TIMEOUT` | `165` | Message timeout in milliseconds |
| `--configure` | - | `false` | Configure FRS records and exit |
| `--tracy` | - | `false` | Enable Tracy profiling |

## Testing

```bash
cargo test
```

## Documentation

For detailed documentation, visit [EdgeFirst Documentation](https://doc.edgefirst.ai/latest/maivin/).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to this project.

## License

Copyright 2025 Au-Zone Technologies Inc.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Security

For security vulnerabilities, see [SECURITY.md](SECURITY.md).
