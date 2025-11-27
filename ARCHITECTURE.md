# Architecture

## Overview

Maivin IMU is a ROS 2 node that provides Inertial Measurement Unit (IMU) data from BNO08x sensors for the EdgeFirst Maivin platform.

## System Architecture

### ROS 2 Node

The IMU node operates as a ROS 2 component with the following responsibilities:

- Initialize and configure BNO08x sensor
- Read sensor data at configured rate
- Publish IMU messages to ROS 2 topics
- Provide calibration status

### Key Components

1. **Driver Layer**
   - BNO08x sensor interface
   - I2C/SPI communication
   - Sensor initialization and configuration

2. **Data Processing**
   - Quaternion calculations
   - Rotation vector processing
   - Sensor fusion algorithms (performed by BNO08x)

3. **Output Generation**
   - ROS 2 IMU messages
   - Calibration status
   - Performance metrics

## Communication

### Zenoh Integration

The IMU node uses Zenoh for distributed communication, enabling:

- Low-latency data distribution
- Efficient network utilization
- Zero-copy transfers where applicable

### Data Flow

```
BNO08x Sensor → IMU Node → ROS 2 Topics
      ↓            ↓            ↓
   I2C/SPI    Processing    IMU Data
   Interface   + Fusion     + Status
```

## Performance

### Tracy Profiling

The IMU node includes Tracy profiling support for:

- Real-time performance monitoring
- Timing analysis
- Bottleneck identification

### Sensor Capabilities

- 9-axis sensor fusion (accelerometer, gyroscope, magnetometer)
- On-chip sensor fusion and calibration
- High-speed sampling rates
- Low power consumption

## Configuration

Configuration is managed through command-line arguments and environment variables. See `args.rs` for available options.

## Future Enhancements

- Multiple IMU support
- Enhanced calibration procedures
- Additional sensor modes
- Sensor diagnostics and health monitoring
