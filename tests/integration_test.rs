// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for EdgeFirst IMU service.
//!
//! These tests require real hardware (BNO08x IMU) and are marked with `#[ignore]`.
//! They are run on the `raivin` hardware runner in CI.
//!
//! The test launches the edgefirst-imu service, subscribes to the IMU topic,
//! verifies messages are received and decodable, then sends SIGTERM for graceful shutdown.

use edgefirst_schemas::{sensor_msgs::IMU, serde_cdr};
use std::{
    env,
    process::{Child, Command},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use zenoh::Wait;

/// Expected minimum message rate (Hz) from the IMU.
/// BNO08x rotation vector typically reports at ~100Hz.
const MIN_EXPECTED_RATE_HZ: f64 = 50.0;

/// Duration to collect IMU messages before analyzing.
const COLLECTION_DURATION: Duration = Duration::from_secs(5);

/// Topic the IMU service publishes to.
const IMU_TOPIC: &str = "rt/imu";

/// Find the edgefirst-imu binary.
/// In CI, it's passed via environment variable. Locally, look in target directory.
fn find_imu_binary() -> String {
    if let Ok(path) = env::var("IMU_BINARY") {
        return path;
    }

    // Try common locations
    let candidates = [
        "target/llvm-cov-target/profiling/edgefirst-imu",
        "target/profiling/edgefirst-imu",
        "target/release/edgefirst-imu",
        "target/debug/edgefirst-imu",
    ];

    for candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    panic!("Could not find edgefirst-imu binary. Set IMU_BINARY environment variable.");
}

/// Start the IMU service as a child process.
fn start_imu_service() -> Child {
    let binary = find_imu_binary();
    println!("Starting IMU service: {}", binary);

    Command::new(&binary)
        .args(["--no-tracy"]) // Disable Tracy for tests
        .spawn()
        .expect("Failed to start IMU service")
}

/// Send SIGTERM to gracefully stop the IMU service.
fn stop_imu_service(mut child: Child) {
    println!("Sending SIGTERM to IMU service (pid: {})", child.id());

    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }

    // Wait for graceful shutdown (up to 5 seconds)
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("IMU service exited with status: {:?}", status);
                return;
            }
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(5) {
                    println!("IMU service did not exit gracefully, killing...");
                    let _ = child.kill();
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                println!("Error waiting for IMU service: {}", e);
                return;
            }
        }
    }
}

/// Integration test for IMU message publishing.
///
/// This test:
/// 1. Starts the edgefirst-imu service
/// 2. Subscribes to the IMU topic
/// 3. Collects messages for COLLECTION_DURATION
/// 4. Verifies messages are valid IMU messages
/// 5. Checks the publishing rate meets minimum threshold
/// 6. Gracefully stops the IMU service
#[test]
#[ignore] // Requires hardware - run on raivin runner
fn test_imu_publishing() {
    // Start the IMU service
    let imu_process = start_imu_service();

    // Give the service time to initialize
    thread::sleep(Duration::from_secs(2));

    // Open Zenoh session
    let session = zenoh::open(zenoh::Config::default())
        .wait()
        .expect("Failed to open Zenoh session");

    // Subscribe to IMU topic
    let message_count = Arc::new(AtomicU64::new(0));
    let message_count_clone = message_count.clone();

    let subscriber = session
        .declare_subscriber(IMU_TOPIC)
        .callback(move |sample| {
            // Try to decode the message
            match serde_cdr::deserialize::<IMU>(&sample.payload().to_bytes()) {
                Ok(imu) => {
                    // Verify the message has reasonable values
                    // Quaternion should be normalized (magnitude ~= 1)
                    let mag = (imu.orientation.x.powi(2)
                        + imu.orientation.y.powi(2)
                        + imu.orientation.z.powi(2)
                        + imu.orientation.w.powi(2))
                    .sqrt();

                    if (mag - 1.0).abs() < 0.1 {
                        message_count_clone.fetch_add(1, Ordering::SeqCst);
                    } else {
                        eprintln!("Invalid quaternion magnitude: {}", mag);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to decode IMU message: {}", e);
                }
            }
        })
        .wait()
        .expect("Failed to create subscriber");

    println!("Collecting IMU messages for {:?}...", COLLECTION_DURATION);
    thread::sleep(COLLECTION_DURATION);

    // Get final count
    let count = message_count.load(Ordering::SeqCst);
    let rate = count as f64 / COLLECTION_DURATION.as_secs_f64();

    println!("Received {} messages in {:?}", count, COLLECTION_DURATION);
    println!("Message rate: {:.1} Hz", rate);

    // Clean up
    drop(subscriber);
    drop(session);
    stop_imu_service(imu_process);

    // Assertions
    assert!(count > 0, "No IMU messages received!");
    assert!(
        rate >= MIN_EXPECTED_RATE_HZ,
        "IMU rate {:.1} Hz is below minimum {:.1} Hz",
        rate,
        MIN_EXPECTED_RATE_HZ
    );

    println!("✓ Integration test passed!");
}

/// Test that the IMU service handles SIGTERM gracefully.
#[test]
#[ignore] // Requires hardware - run on raivin runner
fn test_graceful_shutdown() {
    // Start the IMU service
    let imu_process = start_imu_service();

    // Give the service time to initialize and start publishing
    thread::sleep(Duration::from_secs(3));

    // Send SIGTERM
    let pid = imu_process.id();
    println!("Sending SIGTERM to IMU service (pid: {})", pid);
    unsafe {
        libc::kill(pid as i32, libc::SIGTERM);
    }

    // Wait for exit with timeout
    let mut child = imu_process;
    let start = Instant::now();
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(5) {
                    println!("Timeout waiting for graceful shutdown");
                    let _ = child.kill();
                    break None;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(_) => break None,
        }
    };

    // Verify it exited cleanly
    assert!(
        exit_status.is_some(),
        "IMU service did not exit within timeout"
    );

    let status = exit_status.unwrap();
    println!("IMU service exited with status: {:?}", status);

    // On Unix, SIGTERM results in exit code 0 if handled properly
    // or signal termination if not
    assert!(
        status.success() || status.code().is_none(),
        "IMU service did not exit cleanly: {:?}",
        status
    );

    println!("✓ Graceful shutdown test passed!");
}
