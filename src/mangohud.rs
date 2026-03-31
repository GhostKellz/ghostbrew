// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - MangoHud Integration
//
// Integrates with MangoHud for gaming benchmarks:
// - Detects MangoHud processes
// - Exports scheduler stats to MangoHud-compatible CSV
// - Reads MangoHud frame time logs for analysis
//
// Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

use anyhow::Result;
use log::{debug, info};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// MangoHud-compatible stats export
pub struct MangoHudExporter {
    output_dir: PathBuf,
    stats_file: Option<BufWriter<File>>,
    sample_count: u64,
}

impl MangoHudExporter {
    /// Create a new MangoHud exporter
    pub fn new() -> Self {
        let output_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("ghostbrew");

        Self {
            output_dir,
            stats_file: None,
            sample_count: 0,
        }
    }

    /// Initialize the exporter and create output directory
    pub fn init(&mut self) -> Result<()> {
        // Create output directory if it doesn't exist
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir)?;
            debug!("Created MangoHud export directory: {:?}", self.output_dir);
        }

        // Create stats file with CSV header
        let stats_path = self.output_dir.join("scheduler_stats.csv");
        let file = File::create(&stats_path)?;
        let mut writer = BufWriter::new(file);

        // Write CSV header (MangoHud-compatible format with v0.3.0 percentiles)
        writeln!(
            writer,
            "timestamp_ms,gaming_tasks,latency_avg_us,latency_max_us,latency_p50_us,latency_p95_us,latency_p99_us,jitter_us,late_pct,preemptions,ccd0_tasks,ccd1_tasks"
        )?;
        writer.flush()?;

        self.stats_file = Some(writer);
        info!("MangoHud stats export initialized: {:?}", stats_path);

        Ok(())
    }

    /// Write a stats sample
    pub fn write_sample(&mut self, stats: &SchedulerStats) -> Result<()> {
        if let Some(ref mut writer) = self.stats_file {
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                stats.timestamp_ms,
                stats.gaming_tasks,
                stats.latency_avg_us,
                stats.latency_max_us,
                stats.latency_p50_us,
                stats.latency_p95_us,
                stats.latency_p99_us,
                stats.jitter_us,
                stats.late_pct,
                stats.preemptions,
                stats.ccd0_tasks,
                stats.ccd1_tasks,
            )?;

            self.sample_count += 1;

            // Flush periodically
            if self.sample_count.is_multiple_of(10) {
                writer.flush()?;
            }
        }

        Ok(())
    }

    /// Get the output path
    pub fn output_path(&self) -> PathBuf {
        self.output_dir.join("scheduler_stats.csv")
    }

    /// Get sample count
    #[allow(dead_code)]
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Finalize and flush
    pub fn finalize(&mut self) -> Result<()> {
        if let Some(ref mut writer) = self.stats_file {
            writer.flush()?;
        }
        Ok(())
    }
}

impl Default for MangoHudExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MangoHudExporter {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}

/// Scheduler stats for export
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub timestamp_ms: u64,
    pub gaming_tasks: u64,
    pub latency_avg_us: u64,
    pub latency_max_us: u64,
    pub latency_p50_us: u64,
    pub latency_p95_us: u64,
    pub latency_p99_us: u64,
    pub jitter_us: u64,
    pub late_pct: u64,
    pub preemptions: u64,
    pub ccd0_tasks: u64,
    pub ccd1_tasks: u64,
}

/// Number of histogram buckets (must match BPF HIST_BUCKETS)
pub const HIST_BUCKETS: usize = 16;

/// Calculate latency percentiles from histogram buckets
///
/// The histogram uses buckets matching the BPF latency_to_bucket():
/// - Bucket 0: 0-100us       - Bucket 8: 10-15ms
/// - Bucket 1: 100-200us     - Bucket 9: 15-20ms
/// - Bucket 2: 200-400us     - Bucket 10: 20-30ms
/// - Bucket 3: 400-800us     - Bucket 11: 30-50ms
/// - Bucket 4: 0.8-1.6ms     - Bucket 12: 50-75ms
/// - Bucket 5: 1.6-3.2ms     - Bucket 13: 75-100ms
/// - Bucket 6: 3.2-6.4ms     - Bucket 14: 100-150ms
/// - Bucket 7: 6.4-10ms      - Bucket 15: >150ms
pub fn calculate_latency_percentiles(hist: &[u64; HIST_BUCKETS]) -> (u64, u64, u64) {
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return (0, 0, 0);
    }

    let p50_threshold = total / 2;
    let p95_threshold = (total * 95) / 100;
    let p99_threshold = (total * 99) / 100;

    // Bucket midpoints in microseconds (matching BPF latency_to_bucket)
    const BUCKET_MIDPOINTS: [u64; HIST_BUCKETS] = [
        50,     // 0-100us
        150,    // 100-200us
        300,    // 200-400us
        600,    // 400-800us
        1200,   // 0.8-1.6ms
        2400,   // 1.6-3.2ms
        4800,   // 3.2-6.4ms
        8200,   // 6.4-10ms
        12500,  // 10-15ms
        17500,  // 15-20ms
        25000,  // 20-30ms
        40000,  // 30-50ms
        62500,  // 50-75ms
        87500,  // 75-100ms
        125000, // 100-150ms
        200000, // >150ms (capped estimate)
    ];

    let mut cumulative = 0u64;
    let mut p50 = 0u64;
    let mut p95 = 0u64;
    let mut p99 = 0u64;

    for (i, &count) in hist.iter().enumerate() {
        cumulative += count;
        if p50 == 0 && cumulative >= p50_threshold {
            p50 = BUCKET_MIDPOINTS[i];
        }
        if p95 == 0 && cumulative >= p95_threshold {
            p95 = BUCKET_MIDPOINTS[i];
        }
        if p99 == 0 && cumulative >= p99_threshold {
            p99 = BUCKET_MIDPOINTS[i];
            break;
        }
    }

    (p50, p95, p99)
}

/// Check if MangoHud is running (by looking for mangohud processes)
pub fn is_mangohud_running() -> bool {
    // Check for MangoHud by looking for its socket or process
    let socket_path = dirs::runtime_dir()
        .map(|p| p.join("mangohud/socket"))
        .unwrap_or_else(|| PathBuf::from("/run/user/1000/mangohud/socket"));

    if socket_path.exists() {
        return true;
    }

    // Fallback: scan /proc for mangohud processes
    if let Ok(proc_dir) = fs::read_dir("/proc") {
        for entry in proc_dir.flatten() {
            let comm_path = entry.path().join("comm");
            if let Ok(comm) = fs::read_to_string(&comm_path)
                && comm.trim().contains("mangohud")
            {
                return true;
            }
        }
    }

    false
}

/// MangoHud log reader for frame time analysis
pub struct MangoHudLogReader {
    log_dir: PathBuf,
}

impl MangoHudLogReader {
    /// Create a new log reader
    pub fn new() -> Self {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("MangoHud");

        Self { log_dir }
    }

    /// Find the most recent MangoHud log file
    pub fn find_latest_log(&self) -> Option<PathBuf> {
        if !self.log_dir.exists() {
            return None;
        }

        let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;

        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "csv")
                    && let Ok(metadata) = path.metadata()
                    && let Ok(modified) = metadata.modified()
                {
                    match &latest {
                        Some((_, time)) if modified > *time => {
                            latest = Some((path, modified));
                        }
                        None => {
                            latest = Some((path, modified));
                        }
                        _ => {}
                    }
                }
            }
        }

        latest.map(|(path, _)| path)
    }

    /// Read frame times from a MangoHud log
    pub fn read_frame_times(&self, path: &PathBuf) -> Result<Vec<f64>> {
        let content = fs::read_to_string(path)?;
        let mut frame_times = Vec::new();

        // MangoHud CSV format has "frametime" column
        let mut frametime_col = None;

        for (i, line) in content.lines().enumerate() {
            let cols: Vec<&str> = line.split(',').collect();

            if i == 0 {
                // Find frametime column in header
                for (j, col) in cols.iter().enumerate() {
                    if col.trim().to_lowercase() == "frametime" {
                        frametime_col = Some(j);
                        break;
                    }
                }
            } else if let Some(col) = frametime_col
                && let Some(value) = cols.get(col)
                && let Ok(ft) = value.trim().parse::<f64>()
            {
                frame_times.push(ft);
            }
        }

        Ok(frame_times)
    }

    /// Calculate frame time statistics
    pub fn analyze_frame_times(frame_times: &[f64]) -> FrameTimeStats {
        if frame_times.is_empty() {
            return FrameTimeStats::default();
        }

        let count = frame_times.len() as f64;
        let sum: f64 = frame_times.iter().sum();
        let avg = sum / count;

        let min = frame_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = frame_times
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        // Calculate standard deviation (jitter)
        let variance: f64 = frame_times.iter().map(|ft| (ft - avg).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();

        // Calculate 1% and 0.1% lows
        let mut sorted = frame_times.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p99_idx = ((count * 0.99) as usize).min(sorted.len() - 1);
        let p999_idx = ((count * 0.999) as usize).min(sorted.len() - 1);

        // FPS = 1000 / frame_time_ms
        let fps_avg = 1000.0 / avg;
        let fps_1_low = 1000.0 / sorted[p99_idx];
        let fps_01_low = 1000.0 / sorted[p999_idx];

        FrameTimeStats {
            count: frame_times.len(),
            avg_ms: avg,
            min_ms: min,
            max_ms: max,
            std_dev_ms: std_dev,
            fps_avg,
            fps_1_low,
            fps_01_low,
        }
    }
}

impl Default for MangoHudLogReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame time statistics
#[derive(Debug, Clone, Default)]
pub struct FrameTimeStats {
    pub count: usize,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub std_dev_ms: f64,
    pub fps_avg: f64,
    pub fps_1_low: f64,
    pub fps_01_low: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mangohud_running() {
        // Just test that the function runs without panicking
        let _ = is_mangohud_running();
    }

    #[test]
    fn test_frame_time_analysis() {
        let frame_times = vec![
            16.67, 16.68, 16.66, 17.0, 16.5, 16.7, 16.65, 16.69, 16.67, 16.66,
        ];
        let stats = MangoHudLogReader::analyze_frame_times(&frame_times);

        assert!(stats.fps_avg > 59.0 && stats.fps_avg < 61.0); // ~60 FPS
        assert!(stats.std_dev_ms < 1.0); // Low jitter
    }
}
