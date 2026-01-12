#!/bin/bash
# SPDX-License-Identifier: GPL-2.0
#
# GhostBrew System Benchmark Suite
# Tests real-world scheduler behavior on AMD Zen 5 X3D / Intel Hybrid
#
# Usage: ./system_bench.sh [gaming|work|auto|all]
#
# Copyright (C) 2025-2026 ghostkellz <ckelley@ghostkellz.sh>

set -e

GHOSTBREW_BIN="${GHOSTBREW_BIN:-/data/projects/ghostbrew/target/x86_64-unknown-linux-gnu/release/scx_ghostbrew}"
RESULTS_DIR="${RESULTS_DIR:-/tmp/ghostbrew_bench}"
DURATION="${DURATION:-10}"  # seconds per test

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $*"; }
ok() { echo -e "${GREEN}[OK]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
err() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check prerequisites
check_prereqs() {
    info "Checking prerequisites..."

    if [[ ! -x "$GHOSTBREW_BIN" ]]; then
        err "Ghostbrew binary not found: $GHOSTBREW_BIN"
        exit 1
    fi

    if [[ $EUID -ne 0 ]]; then
        err "This script must be run as root (sudo)"
        exit 1
    fi

    # Check for benchmark tools
    for cmd in stress-ng sysbench; do
        if ! command -v $cmd &>/dev/null; then
            warn "$cmd not found, some benchmarks will be skipped"
        fi
    done

    mkdir -p "$RESULTS_DIR"
    ok "Prerequisites OK"
}

# Get current CCD stats from scheduler
get_sched_stats() {
    local state_file="/sys/kernel/sched_ext/state"
    if [[ -f "$state_file" ]] && [[ "$(cat $state_file)" == "enabled" ]]; then
        echo "enabled"
    else
        echo "disabled"
    fi
}

# Start ghostbrew in background and capture PID
start_ghostbrew() {
    local mode="$1"
    local log_file="$RESULTS_DIR/ghostbrew_${mode}.log"

    info "Starting Ghostbrew in $mode mode..."

    case "$mode" in
        gaming)  "$GHOSTBREW_BIN" --gaming --stats --stats-interval 1 > "$log_file" 2>&1 & ;;
        work)    "$GHOSTBREW_BIN" --work --stats --stats-interval 1 > "$log_file" 2>&1 & ;;
        auto)    "$GHOSTBREW_BIN" --stats --stats-interval 1 > "$log_file" 2>&1 & ;;
        *)       err "Unknown mode: $mode"; return 1 ;;
    esac

    GHOSTBREW_PID=$!
    sleep 2  # Wait for scheduler to attach

    if [[ "$(get_sched_stats)" != "enabled" ]]; then
        err "Scheduler failed to attach"
        kill $GHOSTBREW_PID 2>/dev/null || true
        return 1
    fi

    ok "Ghostbrew started (PID: $GHOSTBREW_PID)"
}

# Stop ghostbrew
stop_ghostbrew() {
    if [[ -n "$GHOSTBREW_PID" ]]; then
        info "Stopping Ghostbrew..."
        kill -INT $GHOSTBREW_PID 2>/dev/null || true
        wait $GHOSTBREW_PID 2>/dev/null || true
        sleep 1
        ok "Ghostbrew stopped"
    fi
}

# Extract stats from log
extract_stats() {
    local log_file="$1"
    local stat_name="$2"

    # Get last occurrence of the stat
    grep -o "$stat_name: [0-9]*" "$log_file" | tail -1 | grep -o '[0-9]*' || echo "0"
}

# Benchmark: Gaming-like workload (bursty, latency-sensitive)
bench_gaming_workload() {
    local mode="$1"
    local result_file="$RESULTS_DIR/gaming_${mode}.txt"

    info "Running gaming-like workload benchmark..."

    # Simulate gaming: short bursts of CPU work with sleeps
    # This mimics frame rendering: burst of work, then wait for vsync
    local start_time=$(date +%s.%N)

    for i in $(seq 1 $((DURATION * 60))); do  # ~60 "frames" per second
        # Burst of work (1-2ms)
        timeout 0.002 dd if=/dev/zero of=/dev/null bs=4096 count=1000 2>/dev/null || true
        # Frame time remainder (~14ms for 60fps)
        sleep 0.014
    done

    local end_time=$(date +%s.%N)
    local elapsed=$(echo "$end_time - $start_time" | bc)

    echo "Gaming workload ($mode mode):" > "$result_file"
    echo "  Duration: ${elapsed}s" >> "$result_file"
    echo "  Simulated frames: $((DURATION * 60))" >> "$result_file"

    ok "Gaming workload complete: ${elapsed}s"
}

# Benchmark: Batch workload (sustained CPU)
bench_batch_workload() {
    local mode="$1"
    local result_file="$RESULTS_DIR/batch_${mode}.txt"

    info "Running batch workload benchmark..."

    if command -v stress-ng &>/dev/null; then
        # CPU-bound batch work on half the cores
        local nr_cpus=$(nproc)
        local workers=$((nr_cpus / 2))

        local start_time=$(date +%s.%N)
        stress-ng --cpu $workers --cpu-method matrixprod --timeout ${DURATION}s --metrics-brief 2>&1 | tee "$result_file"
        local end_time=$(date +%s.%N)
        local elapsed=$(echo "$end_time - $start_time" | bc)

        echo "" >> "$result_file"
        echo "Duration: ${elapsed}s" >> "$result_file"
        echo "Workers: $workers" >> "$result_file"

        ok "Batch workload complete: ${elapsed}s"
    else
        warn "stress-ng not available, using dd fallback"

        local start_time=$(date +%s.%N)
        for i in $(seq 1 4); do
            dd if=/dev/zero of=/dev/null bs=1M count=10000 2>/dev/null &
        done
        wait
        local end_time=$(date +%s.%N)
        local elapsed=$(echo "$end_time - $start_time" | bc)

        echo "Batch workload ($mode mode, dd fallback):" > "$result_file"
        echo "  Duration: ${elapsed}s" >> "$result_file"

        ok "Batch workload complete: ${elapsed}s"
    fi
}

# Benchmark: Mixed workload (gaming + background compile)
bench_mixed_workload() {
    local mode="$1"
    local result_file="$RESULTS_DIR/mixed_${mode}.txt"

    info "Running mixed workload benchmark..."

    # Background batch work
    if command -v stress-ng &>/dev/null; then
        stress-ng --cpu 4 --cpu-method matrixprod --timeout ${DURATION}s &
        local batch_pid=$!
    else
        for i in $(seq 1 4); do
            dd if=/dev/zero of=/dev/null bs=1M count=50000 2>/dev/null &
        done
        local batch_pid=$!
    fi

    # Foreground gaming-like work
    local start_time=$(date +%s.%N)
    for i in $(seq 1 $((DURATION * 60))); do
        timeout 0.002 dd if=/dev/zero of=/dev/null bs=4096 count=1000 2>/dev/null || true
        sleep 0.014
    done
    local end_time=$(date +%s.%N)
    local elapsed=$(echo "$end_time - $start_time" | bc)

    # Wait for background work
    wait $batch_pid 2>/dev/null || true

    echo "Mixed workload ($mode mode):" > "$result_file"
    echo "  Duration: ${elapsed}s" >> "$result_file"
    echo "  Gaming frames: $((DURATION * 60))" >> "$result_file"
    echo "  Background workers: 4" >> "$result_file"

    ok "Mixed workload complete: ${elapsed}s"
}

# Benchmark: Latency test
bench_latency() {
    local mode="$1"
    local result_file="$RESULTS_DIR/latency_${mode}.txt"

    info "Running latency benchmark..."

    if command -v sysbench &>/dev/null; then
        sysbench cpu --cpu-max-prime=5000 --threads=1 --time=$DURATION run 2>&1 | tee "$result_file"
        ok "Latency benchmark complete"
    else
        warn "sysbench not available, skipping latency test"
        echo "SKIPPED: sysbench not available" > "$result_file"
    fi
}

# Run full benchmark suite for a mode
run_benchmark_suite() {
    local mode="$1"

    echo ""
    echo "=========================================="
    echo "  Running benchmarks in $mode mode"
    echo "=========================================="
    echo ""

    start_ghostbrew "$mode" || return 1

    bench_gaming_workload "$mode"
    bench_batch_workload "$mode"
    bench_mixed_workload "$mode"
    bench_latency "$mode"

    # Capture final scheduler stats
    local log_file="$RESULTS_DIR/ghostbrew_${mode}.log"
    sleep 2  # Let stats update

    info "Scheduler stats for $mode mode:"
    echo "  V-Cache migrations: $(extract_stats "$log_file" "V-Cache migrations")"
    echo "  Freq CCD placements: $(extract_stats "$log_file" "Freq CCD placements")"
    echo "  CCD local: $(extract_stats "$log_file" "CCD local")"
    echo "  CCD cross: $(extract_stats "$log_file" "cross")"
    echo "  Gaming tasks: $(extract_stats "$log_file" "Gaming tasks")"
    echo "  Prefcore placements: $(extract_stats "$log_file" "Prefcore placements")"

    stop_ghostbrew

    echo ""
    ok "Benchmark suite for $mode mode complete"
}

# Generate summary report
generate_report() {
    local report_file="$RESULTS_DIR/summary.txt"

    info "Generating summary report..."

    echo "======================================" > "$report_file"
    echo "  GhostBrew Benchmark Summary" >> "$report_file"
    echo "  $(date)" >> "$report_file"
    echo "  Kernel: $(uname -r)" >> "$report_file"
    echo "  CPU: $(grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)" >> "$report_file"
    echo "======================================" >> "$report_file"
    echo "" >> "$report_file"

    for mode in gaming work auto; do
        local log_file="$RESULTS_DIR/ghostbrew_${mode}.log"
        if [[ -f "$log_file" ]]; then
            echo "[$mode mode]" >> "$report_file"
            echo "  V-Cache migrations: $(extract_stats "$log_file" "V-Cache migrations")" >> "$report_file"
            echo "  Freq CCD placements: $(extract_stats "$log_file" "Freq CCD placements")" >> "$report_file"
            echo "  Gaming tasks: $(extract_stats "$log_file" "Gaming tasks")" >> "$report_file"
            echo "  Prefcore placements: $(extract_stats "$log_file" "Prefcore placements")" >> "$report_file"
            echo "" >> "$report_file"
        fi
    done

    ok "Report saved to: $report_file"
    cat "$report_file"
}

# Main
main() {
    local mode="${1:-all}"

    echo ""
    echo "╔══════════════════════════════════════════╗"
    echo "║     GhostBrew System Benchmark Suite     ║"
    echo "║        AMD Zen 5 X3D Optimized           ║"
    echo "╚══════════════════════════════════════════╝"
    echo ""

    check_prereqs

    case "$mode" in
        gaming|work|auto)
            run_benchmark_suite "$mode"
            ;;
        all)
            run_benchmark_suite "gaming"
            run_benchmark_suite "work"
            run_benchmark_suite "auto"
            generate_report
            ;;
        *)
            echo "Usage: $0 [gaming|work|auto|all]"
            exit 1
            ;;
    esac

    echo ""
    ok "All benchmarks complete! Results in: $RESULTS_DIR"
}

# Cleanup on exit
cleanup() {
    stop_ghostbrew
}
trap cleanup EXIT

main "$@"
