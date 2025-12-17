// SPDX-License-Identifier: GPL-2.0
//
// GhostBrew - Build script for BPF compilation
//
// Copyright (C) 2025 ghostkellz <ckelley@ghostkellz.sh>

use std::env;
use std::path::PathBuf;
use libbpf_cargo::SkeletonBuilder;

const BPF_SRC: &str = "src/bpf/ghostbrew.bpf.c";

fn main() {
    let out_dir = PathBuf::from(
        env::var("OUT_DIR").expect("OUT_DIR not set")
    );

    let skel_path = out_dir.join("ghostbrew.skel.rs");

    // Tell cargo to rerun if BPF source changes
    println!("cargo:rerun-if-changed={}", BPF_SRC);
    println!("cargo:rerun-if-changed=src/bpf/vmlinux.h");
    println!("cargo:rerun-if-changed=src/bpf/scx/common.bpf.h");
    println!("cargo:rerun-if-changed=build.rs");

    // Build the BPF skeleton
    SkeletonBuilder::new()
        .source(BPF_SRC)
        .clang_args([
            "-I", "src/bpf",           // For vmlinux.h
            "-I", "src/bpf/scx",       // For scx headers (if needed directly)
            "-D__TARGET_ARCH_x86",     // Target architecture
            "-g",                       // Debug info for BTF
            "-O2",                      // Optimization
        ])
        .build_and_generate(&skel_path)
        .expect("Failed to build BPF skeleton");

    println!("cargo:warning=BPF skeleton generated at: {}", skel_path.display());
}
