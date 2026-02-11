// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Scheduler Benchmarks
//
// Criterion-based benchmarks for scheduler performance analysis.
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Benchmark task classification decisions
///
/// Simulates the workload classification logic to measure decision latency.
fn bench_task_classification(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_classification");

    // Simulate burst history values
    let burst_values: Vec<u64> = vec![
        500_000,    // 0.5ms - interactive
        2_500_000,  // 2.5ms - medium
        10_000_000, // 10ms - batch
        100_000,    // 0.1ms - highly interactive
        50_000_000, // 50ms - heavy batch
    ];

    for &burst in burst_values.iter() {
        group.bench_with_input(
            BenchmarkId::new("burst_classify", format!("{}ns", burst)),
            &burst,
            |b, &burst| {
                b.iter(|| {
                    // Simulate burst-based classification
                    let threshold = 2_000_000u64; // 2ms
                    let is_interactive = burst < threshold;
                    let priority = if is_interactive {
                        0 // High priority
                    } else {
                        (burst / 1_000_000).min(10) as u32 // Scale by ms
                    };
                    black_box((is_interactive, priority))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark CPU selection for gaming tasks
///
/// Simulates the CPU selection logic for V-Cache/P-core preference.
fn bench_cpu_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_selection");

    // Simulate different CPU counts
    let cpu_counts: Vec<usize> = vec![8, 16, 32, 64];

    for &nr_cpus in &cpu_counts {
        // Create simulated CPU context data
        let cpu_data: Vec<(u32, bool, bool)> = (0..nr_cpus)
            .map(|i| {
                let ccd = (i / 8) as u32;
                let is_vcache = ccd == 0;
                let is_idle = i % 3 == 0; // Simulate some idle CPUs
                (ccd, is_vcache, is_idle)
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("vcache_selection", nr_cpus),
            &cpu_data,
            |b, data| {
                b.iter(|| {
                    // Find first idle V-Cache CPU
                    let result = data
                        .iter()
                        .enumerate()
                        .find(|(_, (_, is_vcache, is_idle))| *is_vcache && *is_idle)
                        .map(|(cpu, _)| cpu);
                    black_box(result)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("any_idle_selection", nr_cpus),
            &cpu_data,
            |b, data| {
                b.iter(|| {
                    // Find any idle CPU (fallback)
                    let result = data
                        .iter()
                        .enumerate()
                        .find(|(_, (_, _, is_idle))| *is_idle)
                        .map(|(cpu, _)| cpu);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Intel hybrid CPU selection
fn bench_intel_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("intel_hybrid");

    // Simulate 14900K: 8 P-cores (16 threads) + 16 E-cores
    let pcore_cpus: Vec<u32> = (0..16).collect();
    let ecore_cpus: Vec<u32> = (16..32).collect();
    let idle_mask: u64 = 0b1010101010101010_1010101010101010; // Alternating idle

    group.bench_function("pcore_selection", |b| {
        b.iter(|| {
            // Find idle P-core
            let result = pcore_cpus
                .iter()
                .find(|&&cpu| (idle_mask >> cpu) & 1 == 1)
                .copied();
            black_box(result)
        });
    });

    group.bench_function("ecore_fallback", |b| {
        b.iter(|| {
            // Find idle E-core (when P-cores busy)
            let result = ecore_cpus
                .iter()
                .find(|&&cpu| (idle_mask >> cpu) & 1 == 1)
                .copied();
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark DSQ dispatch decision
fn bench_dsq_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dsq_dispatch");

    // Simulate DSQ with varying queue depths
    let queue_depths: Vec<usize> = vec![1, 10, 50, 100];

    for &depth in &queue_depths {
        // Simulate tasks in queue with priorities
        let queue: Vec<(u32, u64)> = (0..depth)
            .map(|i| (i as u32, (i * 100_000) as u64)) // (pid, vtime)
            .collect();

        group.bench_with_input(
            BenchmarkId::new("find_next_task", depth),
            &queue,
            |b, queue| {
                b.iter(|| {
                    // Find task with lowest vtime
                    let result = queue.iter().min_by_key(|(_, vtime)| *vtime);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark gaming PID lookup simulation
fn bench_gaming_pid_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("gaming_pid_lookup");

    // Simulate gaming PID set of varying sizes
    let set_sizes: Vec<usize> = vec![10, 50, 100, 500];

    for &size in &set_sizes {
        use std::collections::HashSet;
        let gaming_pids: HashSet<u32> = (0..size as u32).map(|i| i * 1000).collect();
        let test_pids: Vec<u32> = (0..1000).map(|i| i * 500).collect();

        group.bench_with_input(
            BenchmarkId::new("hashset_lookup", size),
            &(gaming_pids.clone(), test_pids.clone()),
            |b, (gaming_pids, test_pids)| {
                b.iter(|| {
                    let count = test_pids
                        .iter()
                        .filter(|pid| gaming_pids.contains(pid))
                        .count();
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark CCD locality calculation
fn bench_ccd_locality(c: &mut Criterion) {
    let mut group = c.benchmark_group("ccd_locality");

    // Simulate multi-CCD system (2 CCDs, 16 CPUs each)
    let cpu_to_ccd: Vec<u32> = (0..32).map(|i| if i < 16 { 0 } else { 1 }).collect();

    group.bench_function("ccd_match_check", |b| {
        let prev_cpu = 5u32;
        let target_ccd = cpu_to_ccd[prev_cpu as usize];

        b.iter(|| {
            // Find all CPUs in same CCD
            let same_ccd: Vec<usize> = cpu_to_ccd
                .iter()
                .enumerate()
                .filter(|&(_, ccd)| *ccd == target_ccd)
                .map(|(cpu, _)| cpu)
                .collect();
            black_box(same_ccd)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_task_classification,
    bench_cpu_selection,
    bench_intel_selection,
    bench_dsq_dispatch,
    bench_gaming_pid_lookup,
    bench_ccd_locality,
);

criterion_main!(benches);
