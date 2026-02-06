//! Offline telemetry buffer for storing messages when MQTT is disconnected.
//!
//! This module provides a bounded buffer that stores telemetry messages
//! when the MQTT connection is unavailable, using FIFO eviction.

use std::collections::VecDeque;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::telemetry::Telemetry;

/// A telemetry message with buffering metadata.
#[derive(Debug, Clone)]
pub struct BufferedTelemetry {
    /// The telemetry data
    pub telemetry: Telemetry,
    /// Timestamp when the message was buffered
    pub buffered_at: DateTime<Utc>,
}

impl BufferedTelemetry {
    /// Create a new buffered telemetry message.
    pub fn new(telemetry: Telemetry) -> Self {
        Self {
            telemetry,
            buffered_at: Utc::now(),
        }
    }

    /// Check if this message has expired.
    pub fn is_expired(&self, max_age: Duration) -> bool {
        let age = Utc::now().signed_duration_since(self.buffered_at);
        age > chrono::Duration::from_std(max_age).unwrap_or(chrono::Duration::MAX)
    }
}

/// Offline buffer for telemetry messages with FIFO eviction.
#[derive(Debug)]
pub struct OfflineTelemetryBuffer {
    /// Buffer storage
    buffer: VecDeque<BufferedTelemetry>,
    /// Maximum number of messages to buffer
    max_messages: usize,
    /// Maximum age of buffered messages
    max_age: Duration,
}

impl Default for OfflineTelemetryBuffer {
    fn default() -> Self {
        Self {
            buffer: VecDeque::new(),
            max_messages: 100,
            max_age: Duration::from_secs(60),
        }
    }
}

impl OfflineTelemetryBuffer {
    /// Create a new buffer with custom limits.
    pub fn new(max_messages: usize, max_age: Duration) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_messages),
            max_messages,
            max_age,
        }
    }

    /// Push a telemetry message to the buffer.
    ///
    /// This will:
    /// 1. Evict expired messages
    /// 2. Evict oldest message if buffer is full (FIFO)
    /// 3. Add the new message
    pub fn push(&mut self, telemetry: Telemetry) {
        // Evict expired messages first
        self.evict_expired();

        // Evict oldest if full (FIFO)
        while self.buffer.len() >= self.max_messages {
            self.buffer.pop_front();
        }

        // Add new message
        self.buffer.push_back(BufferedTelemetry::new(telemetry));
    }

    /// Drain all messages from the buffer in chronological order (oldest first).
    ///
    /// Returns all buffered messages and clears the buffer.
    pub fn drain(&mut self) -> Vec<BufferedTelemetry> {
        self.buffer.drain(..).collect()
    }

    /// Evict all expired messages from the buffer.
    pub fn evict_expired(&mut self) {
        let max_age = self.max_age;
        self.buffer.retain(|msg| !msg.is_expired(max_age));
    }

    /// Get the number of buffered messages.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get the maximum number of messages the buffer can hold.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Get the maximum age of buffered messages.
    pub fn max_age(&self) -> Duration {
        self.max_age
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn create_test_telemetry() -> Telemetry {
        Telemetry {
            timestamp: Utc::now().to_rfc3339(),
            latitude: 37.7749,
            longitude: -122.4194,
            door_locked: true,
            door_open: false,
            parking_session_active: true,
        }
    }

    #[test]
    fn test_default_buffer() {
        let buffer = OfflineTelemetryBuffer::default();
        assert_eq!(buffer.max_messages(), 100);
        assert_eq!(buffer.max_age(), Duration::from_secs(60));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_push_and_drain() {
        let mut buffer = OfflineTelemetryBuffer::new(10, Duration::from_secs(60));

        buffer.push(create_test_telemetry());
        buffer.push(create_test_telemetry());

        assert_eq!(buffer.len(), 2);

        let messages = buffer.drain();
        assert_eq!(messages.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_fifo_eviction() {
        let mut buffer = OfflineTelemetryBuffer::new(3, Duration::from_secs(60));

        // Add 5 messages to a buffer of size 3
        for i in 0..5 {
            let mut telem = create_test_telemetry();
            telem.latitude = i as f64;
            buffer.push(telem);
        }

        // Should only have 3 messages (the last 3)
        assert_eq!(buffer.len(), 3);

        let messages = buffer.drain();
        // Check that we have the last 3 messages (FIFO eviction)
        assert_eq!(messages[0].telemetry.latitude, 2.0);
        assert_eq!(messages[1].telemetry.latitude, 3.0);
        assert_eq!(messages[2].telemetry.latitude, 4.0);
    }

    #[test]
    fn test_chronological_order() {
        let mut buffer = OfflineTelemetryBuffer::new(10, Duration::from_secs(60));

        for i in 0..5 {
            let mut telem = create_test_telemetry();
            telem.latitude = i as f64;
            buffer.push(telem);
        }

        let messages = buffer.drain();

        // Messages should be in chronological order (oldest first)
        for i in 0..5 {
            assert_eq!(messages[i].telemetry.latitude, i as f64);
        }
    }

    // Property 20: Offline Telemetry Buffer Limits
    // Validates: Requirements 7.6
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_buffer_never_exceeds_max_messages(
            max_messages in 1usize..50,
            num_pushes in 1usize..200
        ) {
            let mut buffer = OfflineTelemetryBuffer::new(max_messages, Duration::from_secs(60));

            for _ in 0..num_pushes {
                buffer.push(create_test_telemetry());
            }

            // Buffer should never exceed max_messages
            prop_assert!(buffer.len() <= max_messages);
        }

        #[test]
        fn prop_buffer_respects_limits(
            max_messages in 10usize..100,
            max_age_secs in 30u64..120
        ) {
            let buffer = OfflineTelemetryBuffer::new(
                max_messages,
                Duration::from_secs(max_age_secs),
            );

            prop_assert_eq!(buffer.max_messages(), max_messages);
            prop_assert_eq!(buffer.max_age(), Duration::from_secs(max_age_secs));
        }
    }

    // Property 21: Offline Buffer FIFO Eviction
    // Validates: Requirements 7.7
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_fifo_eviction_removes_oldest(
            max_messages in 3usize..20,
            overflow_count in 1usize..10
        ) {
            let mut buffer = OfflineTelemetryBuffer::new(max_messages, Duration::from_secs(60));

            let total_messages = max_messages + overflow_count;

            // Add more messages than the buffer can hold
            for i in 0..total_messages {
                let mut telem = create_test_telemetry();
                telem.latitude = i as f64;
                buffer.push(telem);
            }

            // Buffer should be at max capacity
            prop_assert_eq!(buffer.len(), max_messages);

            // Drain and check that we have the newest messages
            let messages = buffer.drain();

            // The first message should be the one added at index (total - max)
            let expected_first = (total_messages - max_messages) as f64;
            prop_assert_eq!(messages[0].telemetry.latitude, expected_first);

            // The last message should be the one added at index (total - 1)
            let expected_last = (total_messages - 1) as f64;
            prop_assert_eq!(messages[max_messages - 1].telemetry.latitude, expected_last);
        }
    }

    // Property 22: Buffered Message Chronological Publishing
    // Validates: Requirements 7.8
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_drain_returns_chronological_order(
            num_messages in 1usize..50
        ) {
            let mut buffer = OfflineTelemetryBuffer::new(100, Duration::from_secs(60));

            // Add messages with unique latitudes as identifiers
            for i in 0..num_messages {
                let mut telem = create_test_telemetry();
                telem.latitude = i as f64;
                buffer.push(telem);
            }

            let messages = buffer.drain();

            // Messages should be in chronological order (oldest first = lowest index first)
            for i in 0..num_messages {
                prop_assert_eq!(messages[i].telemetry.latitude, i as f64);
            }

            // Buffer time should also be monotonically increasing
            for i in 1..num_messages {
                prop_assert!(messages[i].buffered_at >= messages[i - 1].buffered_at);
            }
        }
    }
}
