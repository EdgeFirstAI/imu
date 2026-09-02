// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Decoupled sampling and publishing.
//!
//! The SPI thread must service the sensor's interrupt line promptly: the
//! BNO085/086 times out and retries when the host is slower than roughly
//! 1/10 of the fastest report period, which costs it processing time and
//! eventually stalls its output. Encoding and publishing to Zenoh on that
//! thread put network latency directly into the interrupt service path.
//!
//! This module moves publishing to its own thread. The sensor callback only
//! writes a fixed-size [`ImuSample`] into a lock-free [`SampleQueue`]; the
//! publisher thread pops samples, patches a pre-encoded CDR message in place
//! and hands it to Zenoh.
//!
//! # Buffer reuse
//!
//! Zenoh takes ownership of the payload, so a single buffer cannot be mutated
//! and republished: the previous publication may still reference it. Instead
//! [`BufferPool`] keeps a few `Bytes` messages and reclaims one with
//! `Bytes::try_into_mut`, which succeeds once Zenoh has dropped its reference.
//! In steady state the same handful of allocations is recycled forever, and
//! `Bytes` converts into a Zenoh payload with no copy and no allocation.
//!
//! Only four fields change per sample (stamp, orientation, angular velocity,
//! linear acceleration); the frame ID and the three covariance arrays are
//! written once when a buffer is first created.

use bytes::{Bytes, BytesMut};
use crossbeam_queue::ArrayQueue;
use edgefirst_schemas::{
    builtin_interfaces::Time,
    geometry_msgs::{Quaternion, Vector3},
    sensor_msgs::Imu,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
    thread::Thread,
};
use tracing::debug;

/// Covariance reported for every field. A leading -1 is the ROS convention
/// for "this quantity is not estimated".
pub const UNKNOWN_COVARIANCE: [f64; 9] = [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

/// Frame ID carried by every published message.
pub const FRAME_ID: &str = "";

/// One sensor reading, copied out of the driver by the sampling thread.
///
/// Deliberately `Copy` and free of heap data so that pushing it into the
/// queue cannot allocate or block the thread servicing the sensor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuSample {
    pub stamp: Time,
    pub orientation: Quaternion,
    pub angular_velocity: Vector3,
    pub linear_acceleration: Vector3,
}

/// A bounded queue of samples between the sampling and publishing threads.
///
/// When the queue is full the oldest sample is discarded: consumers care
/// about the current attitude, not about history, and the sampling thread
/// must never block. Discards are counted so a persistent backlog is visible.
pub struct SampleQueue {
    queue: ArrayQueue<ImuSample>,
    dropped: AtomicU64,
    consumer: OnceLock<Thread>,
}

impl SampleQueue {
    /// Create a queue holding at most `capacity` samples.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            dropped: AtomicU64::new(0),
            consumer: OnceLock::new(),
        }
    }

    /// Register the thread to wake when a sample arrives.
    ///
    /// Without this the consumer only notices new samples when its park
    /// timeout expires. Called once by the publisher thread at startup;
    /// later calls are ignored.
    pub fn set_consumer(&self, thread: Thread) {
        let _ = self.consumer.set(thread);
    }

    /// Wake the consumer if one is registered.
    ///
    /// Safe to call when the consumer is running: `unpark` then leaves a
    /// token that makes its next park return immediately, which closes the
    /// race between checking for an empty queue and parking.
    pub fn wake_consumer(&self) {
        if let Some(thread) = self.consumer.get() {
            thread.unpark();
        }
    }

    /// Push a sample, discarding the oldest one if the queue is full.
    ///
    /// Returns true if a sample had to be discarded to make room.
    pub fn push_overwrite(&self, sample: ImuSample) -> bool {
        let overwritten = match self.queue.force_push(sample) {
            Some(_) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                true
            }
            None => false,
        };
        self.wake_consumer();
        overwritten
    }

    /// Pop the oldest sample, if any.
    pub fn pop(&self) -> Option<ImuSample> {
        self.queue.pop()
    }

    /// Number of samples discarded because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Number of samples currently queued.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the queue currently holds no samples.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/// A pool of pre-encoded CDR messages reused across publications.
///
/// Buffers are handed out as [`BytesMut`] and returned as [`Bytes`] after
/// publication. A returned buffer can only be written again once every other
/// reference to it is gone, which is what `try_into_mut` checks.
pub struct BufferPool {
    free: Vec<Bytes>,
    capacity: usize,
    template: Vec<u8>,
    allocated: u64,
}

impl BufferPool {
    /// Build a pool of at most `capacity` buffers.
    ///
    /// The CDR layout is fixed, so one fully-encoded message is built here
    /// and cloned to seed each buffer; per-sample publishing only overwrites
    /// individual fields.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "buffer pool capacity must be non-zero");
        let template = Self::encode_template();
        Self {
            free: Vec::with_capacity(capacity),
            capacity,
            template,
            allocated: 0,
        }
    }

    /// Encode the invariant parts of the message once: frame ID and the three
    /// covariance arrays never change between samples.
    fn encode_template() -> Vec<u8> {
        let mut buf = Vec::new();
        Imu::builder()
            .stamp(Time { sec: 0, nanosec: 0 })
            .frame_id(FRAME_ID)
            .orientation(Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            })
            .orientation_covariance(UNKNOWN_COVARIANCE)
            .angular_velocity(Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            })
            .angular_velocity_covariance(UNKNOWN_COVARIANCE)
            .linear_acceleration(Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            })
            .linear_acceleration_covariance(UNKNOWN_COVARIANCE)
            .encode_into_vec(&mut buf)
            .expect("Imu CDR layout is fixed and valid");
        buf
    }

    /// Size in bytes of one encoded message.
    pub fn message_len(&self) -> usize {
        self.template.len()
    }

    /// Total buffers this pool has ever allocated.
    pub fn allocated(&self) -> u64 {
        self.allocated
    }

    /// Take a writable buffer, reusing a released one when possible.
    ///
    /// Buffers still referenced by an in-flight publication are kept and
    /// retried later; a fresh one is allocated only when none can be
    /// reclaimed, which in steady state does not happen.
    pub fn acquire(&mut self) -> BytesMut {
        let mut still_in_use = Vec::new();
        while let Some(buf) = self.free.pop() {
            match buf.try_into_mut() {
                Ok(mut reusable) => {
                    // A reclaimed buffer already holds a valid message whose
                    // invariant fields are correct and whose mutable fields
                    // are about to be overwritten, so nothing is rewritten
                    // here. Re-seed only if it somehow lost its length.
                    if reusable.len() != self.template.len() {
                        reusable.clear();
                        reusable.extend_from_slice(&self.template);
                    }
                    self.free.append(&mut still_in_use);
                    return reusable;
                }
                Err(in_flight) => still_in_use.push(in_flight),
            }
        }
        self.free.append(&mut still_in_use);
        self.allocated += 1;
        debug!(
            allocated = self.allocated,
            "allocating a new IMU publish buffer"
        );
        BytesMut::from(&self.template[..])
    }

    /// Return a published buffer to the pool.
    pub fn release(&mut self, buf: Bytes) {
        if self.free.len() < self.capacity {
            self.free.push(buf);
        }
    }
}

/// Patch a pre-encoded message in place with the values of `sample`.
///
/// The buffer must already hold a valid encoded message (as produced by
/// [`BufferPool::acquire`]); only the four varying fields are rewritten.
pub fn write_sample(
    buf: &mut [u8],
    sample: &ImuSample,
) -> Result<(), edgefirst_schemas::cdr::CdrError> {
    let mut msg = Imu::from_cdr(buf)?;
    msg.set_stamp(sample.stamp)?;
    msg.set_orientation(sample.orientation)?;
    msg.set_angular_velocity(sample.angular_velocity)?;
    msg.set_linear_acceleration(sample.linear_acceleration)?;
    Ok(())
}

/// Shared handle to the queue used by the sampling thread.
pub type SharedQueue = Arc<SampleQueue>;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: f64) -> ImuSample {
        ImuSample {
            stamp: Time {
                sec: n as i32,
                nanosec: 7,
            },
            orientation: Quaternion {
                x: n,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            angular_velocity: Vector3 {
                x: n,
                y: 0.1,
                z: 0.2,
            },
            linear_acceleration: Vector3 {
                x: n,
                y: 1.0,
                z: 2.0,
            },
        }
    }

    #[test]
    fn queue_returns_samples_in_order() {
        let q = SampleQueue::new(4);
        assert!(q.is_empty());
        assert!(!q.push_overwrite(sample(1.0)));
        assert!(!q.push_overwrite(sample(2.0)));
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop(), Some(sample(1.0)));
        assert_eq!(q.pop(), Some(sample(2.0)));
        assert_eq!(q.pop(), None);
        assert_eq!(q.dropped(), 0);
    }

    #[test]
    fn queue_drops_oldest_when_full() {
        let q = SampleQueue::new(2);
        assert!(!q.push_overwrite(sample(1.0)));
        assert!(!q.push_overwrite(sample(2.0)));
        // Full: the oldest sample is discarded to make room.
        assert!(q.push_overwrite(sample(3.0)));
        assert_eq!(q.dropped(), 1);
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop(), Some(sample(2.0)));
        assert_eq!(q.pop(), Some(sample(3.0)));
    }

    #[test]
    fn pool_reuses_released_buffer() {
        let mut pool = BufferPool::new(2);
        let first = pool.acquire();
        assert_eq!(pool.allocated(), 1);
        let published = first.freeze();
        let ptr = published.as_ptr();
        pool.release(published);

        // Nothing else references it, so the same allocation comes back.
        let second = pool.acquire();
        assert_eq!(pool.allocated(), 1, "must not allocate when one is free");
        assert_eq!(second.as_ptr(), ptr);
        assert_eq!(second.len(), pool.message_len());
    }

    #[test]
    fn pool_allocates_while_buffer_is_in_flight() {
        let mut pool = BufferPool::new(2);
        let published = pool.acquire().freeze();
        let in_flight = published.clone();
        pool.release(published);

        // The clone stands in for a publication Zenoh has not finished with.
        let _fresh = pool.acquire();
        assert_eq!(pool.allocated(), 2, "in-flight buffer must not be reused");

        // Once the consumer is done the buffer becomes reusable again.
        drop(in_flight);
        let _reused = pool.acquire();
        assert_eq!(pool.allocated(), 2, "released buffer should be reclaimed");
    }

    #[test]
    fn pool_release_respects_capacity() {
        let mut pool = BufferPool::new(1);
        let a = pool.acquire().freeze();
        let b = pool.acquire().freeze();
        pool.release(a);
        pool.release(b);
        assert!(pool.free.len() <= 1);
    }

    #[test]
    fn patched_buffer_decodes_to_written_values() {
        let mut pool = BufferPool::new(1);
        let mut buf = pool.acquire();
        let s = sample(3.5);
        write_sample(&mut buf, &s).expect("patch message");

        let decoded = Imu::from_cdr(buf.as_ref()).expect("decode patched message");
        assert_eq!(decoded.stamp().sec, 3);
        assert_eq!(decoded.stamp().nanosec, 7);
        assert_eq!(decoded.orientation().x, 3.5);
        assert_eq!(decoded.orientation().w, 1.0);
        assert_eq!(decoded.angular_velocity().x, 3.5);
        assert_eq!(decoded.angular_velocity().z, 0.2);
        assert_eq!(decoded.linear_acceleration().x, 3.5);
        assert_eq!(decoded.linear_acceleration().z, 2.0);
        // Invariant fields survive patching.
        assert_eq!(decoded.frame_id(), FRAME_ID);
        assert_eq!(decoded.orientation_covariance(), UNKNOWN_COVARIANCE);
        assert_eq!(decoded.angular_velocity_covariance(), UNKNOWN_COVARIANCE);
        assert_eq!(decoded.linear_acceleration_covariance(), UNKNOWN_COVARIANCE);
    }

    #[test]
    fn reused_buffer_carries_only_the_latest_values() {
        let mut pool = BufferPool::new(1);
        let mut buf = pool.acquire();
        write_sample(&mut buf, &sample(1.0)).expect("patch first");
        let published = buf.freeze();
        pool.release(published);

        let mut buf = pool.acquire();
        write_sample(&mut buf, &sample(9.0)).expect("patch second");
        let decoded = Imu::from_cdr(buf.as_ref()).expect("decode");
        assert_eq!(decoded.orientation().x, 9.0);
        assert_eq!(decoded.stamp().sec, 9);
    }
}
