// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

mod args;
mod driver;
mod publisher;

use args::{scrub_empty_env, Args, KEEP};
use bno08x_rs::{
    interface::{
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    BNO08x, SENSOR_REPORTID_ROTATION_VECTOR,
};
use clap::Parser;
use driver::Driver;
use edgefirst_schemas::{builtin_interfaces, geometry_msgs};
use log::{debug, error, info, trace, warn};
use publisher::{write_sample, BufferPool, ImuSample, SampleQueue, SharedQueue};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH},
};

/// Global shutdown flag for graceful termination.
/// This is critical for coverage instrumentation - LLVM uses atexit() handlers
/// to flush profraw files, so the process must exit cleanly (not via SIGKILL).
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

/// Errors that can occur when generating timestamps.
#[derive(Debug)]
pub enum TimestampError {
    /// System clock is before Unix epoch.
    BeforeEpoch(SystemTimeError),
    /// System clock seconds exceed i32 range (Y2038).
    Overflow,
}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeEpoch(e) => write!(f, "system clock before Unix epoch: {e}"),
            Self::Overflow => write!(f, "system clock seconds exceed i32 range"),
        }
    }
}

impl std::error::Error for TimestampError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BeforeEpoch(e) => Some(e),
            Self::Overflow => None,
        }
    }
}

fn install_signal_handlers() {
    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
    }
}
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt as _, Layer as _, Registry};
use tracy_client::frame_mark;
use zenoh::{
    bytes::{Encoding, ZBytes},
    Session, Wait,
};

const SUCCESS_TIME_LIMIT: Duration = Duration::from_secs(3);

/// Samples buffered between the sampling and publishing threads. At the
/// 5 ms rotation vector period this absorbs about a quarter second of
/// publish stall before the oldest samples start being discarded.
const SAMPLE_QUEUE_DEPTH: usize = 64;

/// CDR buffers recycled across publications. Zenoh releases a payload once it
/// has been serialized to the transport, so a handful covers the in-flight
/// window. If every buffer is still in flight the pool allocates a fresh one
/// to keep publishing, but only this many are retained for reuse, so a value
/// below the real in-flight depth causes steady allocation rather than
/// growing the pool.
const PUBLISH_BUFFERS: usize = 4;

/// Consecutive failed publications before the run is ended so the service can
/// reset and start over. A single failure is treated as transient; this many
/// in a row means nothing is reaching the wire.
const MAX_CONSECUTIVE_PUBLISH_FAILURES: u32 = 10;

/// Upper bound on how long the publisher sleeps on an empty queue. It is
/// woken as soon as a sample is pushed, so this only bounds how quickly the
/// thread notices the stop flag.
const IDLE_PARK_TIMEOUT: Duration = Duration::from_millis(50);

fn main() {
    // SAFETY: single-threaded here; runs before any thread is spawned and
    // before clap reads the environment (EDGEAI-1094).
    unsafe { scrub_empty_env::<Args>(KEEP) };

    // Install signal handlers for graceful shutdown (required for coverage instrumentation)
    install_signal_handlers();

    let args = Args::parse();
    if args.configure {
        let mut driver = Driver::new(&args.device, &args.interrupt, &args.reset);
        driver.imu_driver.init().unwrap();
        match driver.configure_frs() {
            Ok(_) => info!("FRS records updated"),
            Err(e) => error!("ERROR: FRS records not updated: {}", e),
        }
        return;
    }

    args.tracy.then(tracy_client::Client::start);

    let stdout_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(args.rust_log);

    let journald = match tracing_journald::layer() {
        Ok(journald) => Some(journald.with_filter(args.rust_log)),
        Err(_) => None,
    };

    let tracy = match args.tracy {
        true => Some(tracing_tracy::TracyLayer::default().with_filter(args.rust_log)),
        false => None,
    };

    let subscriber = Registry::default()
        .with(stdout_log)
        .with(journald)
        .with(tracy);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    tracing_log::LogTracer::init().unwrap();

    let session = zenoh::open(args.clone()).wait().unwrap();

    let mut consecutive_fail_count = 0;
    while consecutive_fail_count < 3 && !SHUTDOWN.load(Ordering::SeqCst) {
        let elapsed = run_imu(&args, session.clone());
        // considered a success if the IMU runs for more than the time limit
        if elapsed > SUCCESS_TIME_LIMIT {
            consecutive_fail_count = 0;
        } else {
            consecutive_fail_count += 1;
        }
    }

    if SHUTDOWN.load(Ordering::SeqCst) {
        info!("Received shutdown signal, exiting gracefully...");
    } else {
        error!(
            "{} Consecutive failures. Exiting...",
            consecutive_fail_count
        );
    }
}

/// Publish samples from `queue` until `stop` is set and the queue is drained.
///
/// Runs on its own thread so that Zenoh latency never delays the thread
/// servicing the sensor's interrupt line. Each publication patches a recycled
/// CDR buffer in place rather than encoding and allocating a new message.
fn publish_loop(args: &Args, session: Session, queue: SharedQueue, stop: Arc<AtomicBool>) {
    let publisher = match session.declare_publisher(args.topic.clone()).wait() {
        Ok(publisher) => publisher,
        Err(e) => {
            error!("Could not declare publisher for {}: {}", args.topic, e);
            stop.store(true, Ordering::SeqCst);
            return;
        }
    };
    // Cloning this per publication is a refcount bump, not an allocation:
    // Encoding holds its schema in an Arc-backed ZSlice. Setting it on the
    // publisher instead would require zenoh's internal builder trait.
    let encoding = Encoding::APPLICATION_CDR.with_schema("sensor_msgs/msg/Imu");
    let mut pool = BufferPool::new(PUBLISH_BUFFERS);
    let mut consecutive_failures: u32 = 0;
    queue.set_consumer(std::thread::current());
    debug!(
        "Publishing {} to {} ({} byte messages, {} reusable buffers)",
        "sensor_msgs/msg/Imu",
        args.topic,
        pool.message_len(),
        PUBLISH_BUFFERS
    );

    loop {
        let Some(sample) = queue.pop() else {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            // Nothing pending: sleep until a sample is pushed. The producer
            // unparks us, so this costs no CPU between samples and adds no
            // latency to the ones that arrive.
            std::thread::park_timeout(IDLE_PARK_TIMEOUT);
            continue;
        };

        let published = info_span!("publish").in_scope(|| {
            trace!(
                "Pose:   x: {}, y: {}, z: {}, w: {}",
                sample.orientation.x,
                sample.orientation.y,
                sample.orientation.z,
                sample.orientation.w
            );
            trace!(
                "Accel:  x: {}, y: {}, z: {} [m/s^2]",
                sample.linear_acceleration.x,
                sample.linear_acceleration.y,
                sample.linear_acceleration.z
            );
            trace!(
                "Gyro:   x: {}, y: {}, z: {} [rad/s]",
                sample.angular_velocity.x,
                sample.angular_velocity.y,
                sample.angular_velocity.z
            );

            let mut buf = pool.acquire();
            if let Err(e) = write_sample(&mut buf, &sample) {
                warn!("Failed to encode Imu: {}", e);
                return false;
            }
            let payload = buf.freeze();

            let result = publisher
                .put(ZBytes::from(payload.clone()))
                .encoding(encoding.clone())
                .timestamp(session.new_timestamp())
                .wait();

            // The buffer goes back to the pool either way. A failed put may
            // still hold a reference, but the pool only reclaims a buffer
            // once every other reference is gone, so returning it is safe and
            // keeps a burst of failures from churning allocations.
            pool.release(payload);

            match result {
                Ok(()) => true,
                Err(e) => {
                    warn!("Failed to publish Imu: {}", e);
                    false
                }
            }
        });

        if published {
            consecutive_failures = 0;
        } else {
            consecutive_failures += 1;
            if consecutive_failures >= MAX_CONSECUTIVE_PUBLISH_FAILURES {
                error!(
                    "{} consecutive publish failures; ending run",
                    consecutive_failures
                );
                stop.store(true, Ordering::SeqCst);
                break;
            }
        }

        args.tracy.then(frame_mark);
    }

    let dropped = queue.dropped();
    if dropped > 0 {
        warn!(
            "Dropped {} sample(s) because the publish queue was full",
            dropped
        );
    }
    debug!(
        "Publisher thread exiting ({} buffers allocated)",
        pool.allocated()
    );
}

// This function will reset and initialize the IMU, enable reports, and queue
// samples for publishing. If the sensor has produced no sample for a while,
// the function will return.
// The function returns total elapsed duration
fn run_imu(args: &Args, session: Session) -> Duration {
    let fail_time_limit = Duration::from_millis(args.timeout);
    // Initializing the driver interface.
    debug!("Initializing driver wrapper with parameters:");
    debug!(
        "device: {} interrupt: {} reset: {}",
        args.device, args.interrupt, args.reset
    );

    let mut driver = driver::Driver::new(&args.device, &args.interrupt, &args.reset);
    if let Err(e) = driver.imu_driver.init() {
        error!("Could not initialize driver: {:?}", e);
        return Duration::from_nanos(0);
    }
    if let Err(e) = driver.enable_reports() {
        error!("Could not initialize reports: {:?}", e);
        return Duration::from_nanos(0);
    }

    info!("IMU Device Initialized");

    let queue: SharedQueue = Arc::new(SampleQueue::new(SAMPLE_QUEUE_DEPTH));
    let stop = Arc::new(AtomicBool::new(false));
    let publisher_thread = {
        let args = args.clone();
        let session = session.clone();
        let queue = queue.clone();
        let stop = stop.clone();
        std::thread::Builder::new()
            .name("imu-publish".into())
            .spawn(move || publish_loop(&args, session, queue, stop))
            .expect("spawn publisher thread")
    };

    // The watchdog tracks samples read from the sensor, not messages put on
    // the wire: a slow subscriber or a stalled network must not trigger a
    // sensor reset.
    let last_sample = Arc::from(Mutex::from((Instant::now(), false)));
    let last_sample_ = last_sample.clone();
    let queue_ = queue.clone();
    // The callback runs on the thread servicing the sensor interrupt, so it
    // only copies values into the queue: no allocation, no I/O, no lock held
    // across work.
    let report_update_cb =
        move |imu_driver: &BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>| {
            let [qi, qj, qk, qr] = imu_driver.rotation_quaternion().unwrap();
            let [lin_ax, lin_ay, lin_az] = imu_driver.accelerometer().unwrap();
            let [ang_ax, ang_ay, ang_az] = imu_driver.gyro().unwrap();

            let stamp = match timestamp() {
                Ok(t) => t,
                Err(TimestampError::Overflow) => {
                    warn!("Timestamp overflow: seconds exceed i32::MAX, saturating");
                    builtin_interfaces::Time {
                        sec: i32::MAX,
                        nanosec: 999_999_999,
                    }
                }
                Err(e) => {
                    warn!("Failed to get timestamp: {}", e);
                    return;
                }
            };

            queue_.push_overwrite(ImuSample {
                stamp,
                orientation: geometry_msgs::Quaternion {
                    x: qi as f64,
                    y: qj as f64,
                    z: qk as f64,
                    w: qr as f64,
                },
                angular_velocity: geometry_msgs::Vector3 {
                    x: ang_ax as f64,
                    y: ang_ay as f64,
                    z: ang_az as f64,
                },
                linear_acceleration: geometry_msgs::Vector3 {
                    x: lin_ax as f64,
                    y: lin_ay as f64,
                    z: lin_az as f64,
                },
            });

            let mut last = last_sample_.lock().unwrap();
            *last = (Instant::now(), true);
        };

    driver.imu_driver.add_sensor_report_callback(
        SENSOR_REPORTID_ROTATION_VECTOR,
        String::from("report_update_cb"),
        report_update_cb,
    );
    let start = Instant::now();
    let elapsed = loop {
        // Check for shutdown signal
        if SHUTDOWN.load(Ordering::SeqCst) {
            info!("Shutdown signal received in run_imu loop");
            break start.elapsed();
        }

        // The publisher sets the stop flag if it cannot publish at all (for
        // example the publisher could not be declared). Sampling without a
        // consumer is pointless, and the sample watchdog would never fire, so
        // end the run and let the caller retry.
        if stop.load(Ordering::SeqCst) {
            error!("Publisher stopped; ending IMU run");
            break start.elapsed();
        }

        let _msg_count = driver.imu_driver.handle_messages(2, 10);
        let lock = last_sample.lock().unwrap();
        let last_sample_time = lock.0;
        let started = lock.1;
        drop(lock);
        let elapsed = last_sample_time.elapsed();

        let time_limit = if started {
            fail_time_limit
        } else {
            // 5x higher time limit for reading the first message
            fail_time_limit * 5
        };

        if elapsed > time_limit {
            error!("Last sample was read {:?} ago. Resetting IMU...", elapsed);
            break start.elapsed();
        }
        // Don't need to sleep in this loop because handle_messages uses a sleep
        // for the message polling, so if there is no message the
        // handle_messages function will sleep the thread
    };

    // Let the publisher drain what is already queued, then join it so the
    // process can exit cleanly (required for coverage instrumentation).
    stop.store(true, Ordering::SeqCst);
    queue.wake_consumer();
    if let Err(e) = publisher_thread.join() {
        warn!("Publisher thread panicked: {:?}", e);
    }

    elapsed
}

fn timestamp() -> Result<builtin_interfaces::Time, TimestampError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(TimestampError::BeforeEpoch)?;

    let secs = duration.as_secs();
    if secs > i32::MAX as u64 {
        return Err(TimestampError::Overflow);
    }

    Ok(builtin_interfaces::Time {
        sec: secs as i32,
        nanosec: duration.subsec_nanos(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgefirst_schemas::sensor_msgs::Imu;

    /// The values a sensor callback copies out of the driver must survive
    /// the queue and the in-place encoding unchanged.
    #[test]
    fn sample_survives_queue_and_encoding() {
        let queue = SampleQueue::new(4);
        queue.push_overwrite(ImuSample {
            stamp: builtin_interfaces::Time {
                sec: 12,
                nanosec: 34,
            },
            orientation: geometry_msgs::Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            angular_velocity: geometry_msgs::Vector3 {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
            linear_acceleration: geometry_msgs::Vector3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        });

        let sample = queue.pop().expect("queued sample");
        let mut pool = BufferPool::new(PUBLISH_BUFFERS);
        let mut buf = pool.acquire();
        write_sample(&mut buf, &sample).expect("encode sample");

        let decoded = Imu::from_cdr(buf.as_ref()).expect("decode Imu");
        assert_eq!(decoded.stamp().sec, 12);
        assert_eq!(decoded.stamp().nanosec, 34);
        assert_eq!(decoded.orientation().w, 1.0);
        assert_eq!(decoded.angular_velocity().x, 0.1);
        assert_eq!(decoded.linear_acceleration().z, 3.0);
        assert_eq!(decoded.orientation_covariance()[0], -1.0);
    }
}
