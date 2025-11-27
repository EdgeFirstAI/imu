# Maivin IMU

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Build Status](https://github.com/EdgeFirstAI/imu/actions/workflows/build.yml/badge.svg)](https://github.com/EdgeFirstAI/imu/actions/workflows/build.yml)
[![Test Status](https://github.com/EdgeFirstAI/imu/actions/workflows/test.yml/badge.svg)](https://github.com/EdgeFirstAI/imu/actions/workflows/test.yml)

Inertial Measurement Unit (IMU) driver for EdgeFirst Maivin platform.

## Overview

Maivin IMU is a ROS 2 node that provides IMU sensor data from BNO08x sensors for the EdgeFirst Maivin platform.

## Features

- BNO08x IMU sensor support
- Calibration and sensor fusion
- Real-time processing with Tracy profiling support
- Zenoh integration for distributed communication
- Quaternion and rotation vector outputs

## Requirements

- Rust 1.70 or later
- ROS 2 Humble or later
- BNO08x IMU sensor hardware

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

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

Licensed under the Apache License, Version 2.0. See [LICENSE.txt](LICENSE.txt) for details.

## Security

For security vulnerabilities, see [SECURITY.md](SECURITY.md).
