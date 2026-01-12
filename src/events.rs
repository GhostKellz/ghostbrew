// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Event Streaming Module
//
// Consumes events from the BPF ringbuf for real-time visibility
// into scheduler decisions: gaming detection, migrations, latency spikes, etc.
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use libbpf_rs::{RingBuffer, RingBufferBuilder};
use log::{debug, info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Event types matching BPF side
pub const EVENT_GAMING_DETECTED: u32 = 1;
pub const EVENT_VCACHE_MIGRATION: u32 = 2;
pub const EVENT_PREEMPT_KICK: u32 = 3;
pub const EVENT_HIGH_LATENCY: u32 = 4;
pub const EVENT_CCD_IMBALANCE: u32 = 5;
pub const EVENT_PROFILE_MATCH: u32 = 6;

/// Event structure matching BPF sched_event
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SchedEvent {
    pub timestamp_ns: u64,
    pub event_type: u32,
    pub pid: u32,
    pub cpu: u32,
    pub ccd: u32,
    pub value1: u64,
    pub value2: u64,
    pub comm: [u8; 16],
}

impl SchedEvent {
    /// Get comm as a string
    pub fn comm_str(&self) -> String {
        let null_pos = self.comm.iter().position(|&c| c == 0).unwrap_or(16);
        String::from_utf8_lossy(&self.comm[..null_pos]).to_string()
    }

    /// Get event type name
    #[allow(dead_code)]
    pub fn event_name(&self) -> &'static str {
        match self.event_type {
            EVENT_GAMING_DETECTED => "GamingDetected",
            EVENT_VCACHE_MIGRATION => "VCacheMigration",
            EVENT_PREEMPT_KICK => "PreemptKick",
            EVENT_HIGH_LATENCY => "HighLatency",
            EVENT_CCD_IMBALANCE => "CCDImbalance",
            EVENT_PROFILE_MATCH => "ProfileMatch",
            _ => "Unknown",
        }
    }

    /// Format event for display
    pub fn format(&self) -> String {
        match self.event_type {
            EVENT_GAMING_DETECTED => {
                let gaming_type = if self.value1 == 2 { "Proton" } else { "Gaming" };
                let gpu = if self.value2 == 1 { " (GPU)" } else { "" };
                format!(
                    "{} task detected: {} (PID {}) on CPU {}{}",
                    gaming_type,
                    self.comm_str(),
                    self.pid,
                    self.cpu,
                    gpu
                )
            }
            EVENT_VCACHE_MIGRATION => {
                let from_ccd = self.value1;
                format!(
                    "V-Cache migration: PID {} CPU {} (CCD {} -> CCD {})",
                    self.pid, self.cpu, from_ccd, self.ccd
                )
            }
            EVENT_PREEMPT_KICK => {
                format!(
                    "Preempt kick: PID {} kicked CPU {} on CCD {}",
                    self.pid, self.cpu, self.ccd
                )
            }
            EVENT_HIGH_LATENCY => {
                let latency_us = self.value1;
                let threshold_us = self.value2;
                format!(
                    "High latency: PID {} on CPU {} - {}us (threshold {}us)",
                    self.pid, self.cpu, latency_us, threshold_us
                )
            }
            EVENT_CCD_IMBALANCE => {
                let heavy_load = self.value1;
                let light_load = self.value2;
                format!(
                    "CCD imbalance: CCD {} has {} tasks vs {} tasks",
                    self.ccd, heavy_load, light_load
                )
            }
            EVENT_PROFILE_MATCH => {
                format!(
                    "Profile matched: {} (PID {}) on CPU {}",
                    self.comm_str(),
                    self.pid,
                    self.cpu
                )
            }
            _ => format!("Unknown event type {}", self.event_type),
        }
    }
}

/// Event counters for summary statistics
#[derive(Default)]
pub struct EventCounters {
    pub gaming_detected: AtomicU64,
    pub vcache_migrations: AtomicU64,
    pub preempt_kicks: AtomicU64,
    pub high_latency: AtomicU64,
    pub ccd_imbalance: AtomicU64,
    pub profile_matches: AtomicU64,
    #[allow(dead_code)]
    pub dropped: AtomicU64,
}

impl EventCounters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, event: &SchedEvent) {
        match event.event_type {
            EVENT_GAMING_DETECTED => self.gaming_detected.fetch_add(1, Ordering::Relaxed),
            EVENT_VCACHE_MIGRATION => self.vcache_migrations.fetch_add(1, Ordering::Relaxed),
            EVENT_PREEMPT_KICK => self.preempt_kicks.fetch_add(1, Ordering::Relaxed),
            EVENT_HIGH_LATENCY => self.high_latency.fetch_add(1, Ordering::Relaxed),
            EVENT_CCD_IMBALANCE => self.ccd_imbalance.fetch_add(1, Ordering::Relaxed),
            EVENT_PROFILE_MATCH => self.profile_matches.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    pub fn summary(&self) -> String {
        format!(
            "Events: gaming={}, migrations={}, kicks={}, latency={}, imbalance={}",
            self.gaming_detected.load(Ordering::Relaxed),
            self.vcache_migrations.load(Ordering::Relaxed),
            self.preempt_kicks.load(Ordering::Relaxed),
            self.high_latency.load(Ordering::Relaxed),
            self.ccd_imbalance.load(Ordering::Relaxed),
        )
    }
}

/// Event handler that processes incoming events
pub struct EventHandler {
    pub counters: Arc<EventCounters>,
    pub verbose: bool,
}

impl EventHandler {
    pub fn new(verbose: bool) -> Self {
        Self {
            counters: Arc::new(EventCounters::new()),
            verbose,
        }
    }

    /// Process a single event
    pub fn handle_event(&self, data: &[u8]) -> i32 {
        if data.len() < std::mem::size_of::<SchedEvent>() {
            warn!("Received truncated event: {} bytes", data.len());
            return 0;
        }

        // Safety: We verified the length above and SchedEvent is repr(C)
        let event = unsafe { &*(data.as_ptr() as *const SchedEvent) };

        // Record in counters
        self.counters.record(event);

        // Log if verbose
        if self.verbose {
            info!("[EVENT] {}", event.format());
        } else {
            debug!("[EVENT] {}", event.format());
        }

        0 // Continue processing
    }
}

/// Build a ringbuf consumer for the events map
pub fn build_ringbuf<'a>(
    events_map: &'a libbpf_rs::Map,
    handler: Arc<EventHandler>,
) -> Result<RingBuffer<'a>, libbpf_rs::Error> {
    let handler_clone = handler.clone();

    let mut builder = RingBufferBuilder::new();
    builder.add(events_map, move |data: &[u8]| {
        handler_clone.handle_event(data)
    })?;

    builder.build()
}

/// Poll the ringbuf for events (non-blocking)
pub fn poll_events(ringbuf: &RingBuffer, timeout: Duration) -> Result<(), libbpf_rs::Error> {
    ringbuf.poll(timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_names() {
        let event = SchedEvent {
            timestamp_ns: 0,
            event_type: EVENT_GAMING_DETECTED,
            pid: 1234,
            cpu: 0,
            ccd: 0,
            value1: 2,
            value2: 1,
            comm: *b"game.exe\0\0\0\0\0\0\0\0",
        };

        assert_eq!(event.event_name(), "GamingDetected");
        assert_eq!(event.comm_str(), "game.exe");
    }

    #[test]
    fn test_event_format() {
        let event = SchedEvent {
            timestamp_ns: 0,
            event_type: EVENT_HIGH_LATENCY,
            pid: 5678,
            cpu: 4,
            ccd: 0,
            value1: 2500, // 2500us latency
            value2: 1000, // 1000us threshold
            comm: [0; 16],
        };

        let formatted = event.format();
        assert!(formatted.contains("2500us"));
        assert!(formatted.contains("1000us"));
    }
}
