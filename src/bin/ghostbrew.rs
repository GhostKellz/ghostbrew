use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const APP_NAME: &str = "ghostbrew";
const DEFAULT_SCHEDULER_NAME: &str = "scx_ghostbrew";
const DEFAULT_BENCHMARK_REPORT_BASENAME: &str = "ghostbrew-benchmark";
const DOCS_REPORT_DIR: &str = "docs/benchmarks";

#[derive(Parser, Debug)]
#[command(name = APP_NAME)]
#[command(author = "ghostkellz <ckelley@ghostkellz.sh>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "GhostBrew utilities for running, benchmarking, and collecting support data")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the sched-ext GhostBrew scheduler
    Run {
        /// Arguments forwarded directly to scx_ghostbrew
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Generate a GhostBrew support bundle
    Support {
        /// Write machine-readable JSON next to the text bundle
        #[arg(long)]
        json: bool,

        /// Custom output path for the text bundle
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Run a benchmark workload and write structured reports
    Benchmark {
        /// Workload to run while GhostBrew stats are sampled
        #[arg(long, default_value = "cargo check -q")]
        workload: String,

        /// Path to the GhostBrew scheduler binary
        #[arg(long)]
        scheduler: Option<PathBuf>,

        /// Custom output directory for reports
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Skip rebuilding the scheduler if the binary is missing
        #[arg(long)]
        no_build: bool,

        /// Stats interval passed to scx_ghostbrew
        #[arg(long, default_value_t = 1)]
        stats_interval: u64,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Serialize)]
struct SupportBundleJson {
    timestamp: String,
    hostname: String,
    distribution: String,
    kernel: String,
    arch: String,
    version: String,
    sched_ext_state: String,
    control_file: String,
    cpu_model: String,
    lscpu_summary: String,
    prefcore: String,
    pstate_status: String,
    vcache_mode: String,
    die_cpus_list_cpu0: String,
    die_cpus_list_cpu8: String,
    cluster_id_cpu0: String,
    cluster_id_cpu8: String,
    config_paths: Vec<String>,
    profile_dirs: Vec<String>,
    benchmark_reports: Vec<EmbeddedReport>,
    recent_dmesg: Vec<String>,
}

#[derive(Serialize)]
struct EmbeddedReport {
    path: String,
    contents: String,
}

#[derive(Serialize)]
struct BenchmarkJson {
    timestamp: String,
    vcache_mode: String,
    workload: String,
    scheduler: String,
    enqueued: Option<String>,
    interactive_tasks: Option<String>,
    prefcore_placements: Option<String>,
    freq_ccd_placements: Option<String>,
    log_file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { args } => command_run(args),
        Commands::Support { json, output } => command_support(output, json),
        Commands::Benchmark {
            workload,
            scheduler,
            output_dir,
            no_build,
            stats_interval,
        } => command_benchmark(
            &workload,
            scheduler.as_deref(),
            output_dir,
            no_build,
            stats_interval,
        ),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, APP_NAME, &mut std::io::stdout());
            Ok(())
        }
    }
}

fn command_run(args: Vec<OsString>) -> Result<()> {
    let status = Command::new(DEFAULT_SCHEDULER_NAME)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to execute scx_ghostbrew")?;

    if status.success() {
        return Ok(());
    }

    match status.code() {
        Some(code) => std::process::exit(code),
        None => bail!("scx_ghostbrew terminated by signal"),
    }
}

fn command_support(output: Option<PathBuf>, write_json: bool) -> Result<()> {
    let timestamp = now();
    let state_root = state_root();
    let support_dir = state_root
        .join("support")
        .join(date_dir_component(&timestamp));
    fs::create_dir_all(&support_dir)
        .with_context(|| format!("failed to create {}", support_dir.display()))?;

    let text_output = output.unwrap_or_else(|| {
        support_dir.join(timestamped_name("ghostbrew-support", "txt", &timestamp))
    });
    if let Some(parent) = text_output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let bundle = collect_support_bundle(&timestamp)?;
    fs::write(&text_output, render_support_bundle(&bundle))
        .with_context(|| format!("failed to write {}", text_output.display()))?;
    println!("Wrote support bundle to {}", text_output.display());

    if write_json {
        let json_output = text_output.with_extension("json");
        fs::write(&json_output, serde_json::to_string_pretty(&bundle)? + "\n")
            .with_context(|| format!("failed to write {}", json_output.display()))?;
        println!("Wrote JSON support bundle to {}", json_output.display());
    }

    Ok(())
}

fn command_benchmark(
    workload: &str,
    scheduler: Option<&Path>,
    output_dir: Option<PathBuf>,
    no_build: bool,
    stats_interval: u64,
) -> Result<()> {
    let repo_root = detect_repo_root().ok();
    let timestamp = now();
    let results_dir = output_dir.unwrap_or_else(|| {
        state_root()
            .join("benchmarks")
            .join(date_dir_component(&timestamp))
    });
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;

    let scheduler_path = resolve_scheduler_binary(scheduler, repo_root.as_deref())?;

    if !scheduler_path.exists() {
        if no_build {
            bail!("scheduler binary not found at {}", scheduler_path.display());
        }

        let Some(repo_root) = repo_root.as_ref() else {
            bail!(
                "scheduler binary not found at {} and no repository checkout is available for cargo build",
                scheduler_path.display()
            );
        };

        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(repo_root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("failed to run cargo build --release")?;
        if !status.success() {
            bail!("cargo build --release failed");
        }
    }

    let log_file = results_dir.join(timestamped_name(
        DEFAULT_BENCHMARK_REPORT_BASENAME,
        "log",
        &timestamp,
    ));
    let mut scheduler_child = Command::new("sudo")
        .arg(&scheduler_path)
        .arg("--stats")
        .arg("--stats-interval")
        .arg(stats_interval.to_string())
        .stdout(Stdio::from(fs::File::create(&log_file).with_context(
            || format!("failed to create {}", log_file.display()),
        )?))
        .stderr(Stdio::from(
            fs::File::options()
                .append(true)
                .open(&log_file)
                .with_context(|| format!("failed to open {}", log_file.display()))?,
        ))
        .spawn()
        .context("failed to launch scx_ghostbrew benchmark run")?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    let workload_status = Command::new("sh")
        .arg("-lc")
        .arg(workload)
        .current_dir(
            repo_root
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        )
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run workload `{workload}`"))?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    let _ = Command::new("sudo")
        .arg("kill")
        .arg(scheduler_child.id().to_string())
        .status();
    let _ = scheduler_child.wait();

    if !workload_status.success() {
        bail!("benchmark workload failed: {workload}");
    }

    let payload = collect_benchmark_report(&timestamp, workload, &scheduler_path, &log_file)?;
    let text_output = results_dir.join(timestamped_name(
        DEFAULT_BENCHMARK_REPORT_BASENAME,
        "txt",
        &timestamp,
    ));
    let json_output = results_dir.join(timestamped_name(
        DEFAULT_BENCHMARK_REPORT_BASENAME,
        "json",
        &timestamp,
    ));
    let latest_text = results_dir.join("latest.txt");
    let latest_json = results_dir.join("latest.json");

    let rendered_text = render_benchmark_report(&payload);
    fs::write(&text_output, &rendered_text)
        .with_context(|| format!("failed to write {}", text_output.display()))?;
    fs::write(&json_output, serde_json::to_string_pretty(&payload)? + "\n")
        .with_context(|| format!("failed to write {}", json_output.display()))?;
    fs::write(&latest_text, &rendered_text)
        .with_context(|| format!("failed to write {}", latest_text.display()))?;
    fs::write(&latest_json, serde_json::to_string_pretty(&payload)? + "\n")
        .with_context(|| format!("failed to write {}", latest_json.display()))?;

    println!("Wrote benchmark report to {}", text_output.display());
    println!("Wrote benchmark JSON to {}", json_output.display());

    Ok(())
}

fn collect_support_bundle(timestamp: &str) -> Result<SupportBundleJson> {
    let config_paths = [
        PathBuf::from("/etc/ghostbrew/config.toml"),
        home_dir().join(".config/ghostbrew/config.toml"),
    ];
    let profile_dirs = [
        PathBuf::from("/etc/ghostbrew/profiles"),
        home_dir().join(".config/ghostbrew/profiles"),
    ];

    Ok(SupportBundleJson {
        timestamp: timestamp.to_string(),
        hostname: read_command("hostname", &[]).unwrap_or_else(|_| "unavailable".to_string()),
        distribution: distribution_name(),
        kernel: read_command("uname", &["-r"]).unwrap_or_else(|_| "unavailable".to_string()),
        arch: read_command("uname", &["-m"]).unwrap_or_else(|_| "unavailable".to_string()),
        version: env!("CARGO_PKG_VERSION").to_string(),
        sched_ext_state: read_file_trim("/sys/kernel/sched_ext/state"),
        control_file: read_command("ls", &["-l", "/run/ghostbrew/control"])
            .unwrap_or_else(|_| "missing".to_string()),
        cpu_model: first_cpu_model(),
        lscpu_summary: read_command("lscpu", &[])
            .map(|s| s.replace('\n', " | "))
            .unwrap_or_else(|_| "unavailable".to_string()),
        prefcore: read_file_trim("/sys/devices/system/cpu/amd_pstate/prefcore"),
        pstate_status: read_file_trim("/sys/devices/system/cpu/amd_pstate/status"),
        vcache_mode: detect_vcache_mode(),
        die_cpus_list_cpu0: read_file_trim("/sys/devices/system/cpu/cpu0/topology/die_cpus_list"),
        die_cpus_list_cpu8: read_file_trim("/sys/devices/system/cpu/cpu8/topology/die_cpus_list"),
        cluster_id_cpu0: read_file_trim("/sys/devices/system/cpu/cpu0/topology/cluster_id"),
        cluster_id_cpu8: read_file_trim("/sys/devices/system/cpu/cpu8/topology/cluster_id"),
        config_paths: config_paths
            .into_iter()
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .collect(),
        profile_dirs: profile_dirs
            .into_iter()
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .collect(),
        benchmark_reports: collect_embedded_reports(),
        recent_dmesg: collect_recent_dmesg(),
    })
}

fn collect_benchmark_report(
    timestamp: &str,
    workload: &str,
    scheduler: &Path,
    log_file: &Path,
) -> Result<BenchmarkJson> {
    let text = fs::read_to_string(log_file)
        .with_context(|| format!("failed to read {}", log_file.display()))?;
    let last = text
        .split("--- GhostBrew Stats ---")
        .filter(|chunk| !chunk.trim().is_empty())
        .last()
        .unwrap_or_default()
        .to_string();

    Ok(BenchmarkJson {
        timestamp: timestamp.to_string(),
        vcache_mode: detect_vcache_mode(),
        workload: workload.to_string(),
        scheduler: scheduler.display().to_string(),
        enqueued: stat_line(&last, "Enqueued"),
        interactive_tasks: stat_line(&last, "Interactive tasks"),
        prefcore_placements: stat_line(&last, "Prefcore placements"),
        freq_ccd_placements: stat_line(&last, "Freq CCD placements"),
        log_file: log_file.display().to_string(),
    })
}

fn render_support_bundle(bundle: &SupportBundleJson) -> String {
    let mut out = String::new();
    out.push_str("ghostbrew support bundle\n");
    out.push_str("=======================\n\n");
    out.push_str("[system]\n");
    out.push_str(&format!("timestamp={}\n", bundle.timestamp));
    out.push_str(&format!("hostname={}\n", bundle.hostname));
    out.push_str(&format!("distribution={}\n", bundle.distribution));
    out.push_str(&format!("kernel={}\n", bundle.kernel));
    out.push_str(&format!("arch={}\n\n", bundle.arch));
    out.push_str("[ghostbrew]\n");
    out.push_str(&format!("version={}\n", bundle.version));
    out.push_str(&format!("sched_ext_state={}\n", bundle.sched_ext_state));
    out.push_str(&format!("control_file={}\n\n", bundle.control_file));
    out.push_str("[cpu]\n");
    out.push_str(&format!("model={}\n", bundle.cpu_model));
    out.push_str(&format!("lscpu_summary={}\n\n", bundle.lscpu_summary));
    out.push_str("[amd_x3d]\n");
    out.push_str(&format!("prefcore={}\n", bundle.prefcore));
    out.push_str(&format!("pstate_status={}\n", bundle.pstate_status));
    out.push_str(&format!("vcache_mode={}\n", bundle.vcache_mode));
    out.push_str(&format!(
        "die_cpus_list_cpu0={}\n",
        bundle.die_cpus_list_cpu0
    ));
    out.push_str(&format!(
        "die_cpus_list_cpu8={}\n",
        bundle.die_cpus_list_cpu8
    ));
    out.push_str(&format!("cluster_id_cpu0={}\n", bundle.cluster_id_cpu0));
    out.push_str(&format!("cluster_id_cpu8={}\n\n", bundle.cluster_id_cpu8));
    out.push_str("[config]\n");
    for path in &bundle.config_paths {
        out.push_str(&format!("config_path={}\n", path));
    }
    out.push('\n');
    out.push_str("[profiles]\n");
    for dir in &bundle.profile_dirs {
        out.push_str(&format!("profiles_dir={}\n", dir));
    }
    out.push('\n');
    out.push_str("[benchmark_reports]\n");
    for report in &bundle.benchmark_reports {
        out.push_str(&format!("file={}\n", report.path));
        for line in report.contents.lines() {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str("[recent_dmesg]\n");
    for line in &bundle.recent_dmesg {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn render_benchmark_report(payload: &BenchmarkJson) -> String {
    [
        "GhostBrew benchmark".to_string(),
        format!("timestamp={}", payload.timestamp),
        format!("vcache_mode={}", payload.vcache_mode),
        format!("workload={}", payload.workload),
        format!("scheduler={}", payload.scheduler),
        format!(
            "enqueued={}",
            payload.enqueued.as_deref().unwrap_or("unavailable")
        ),
        format!(
            "interactive_tasks={}",
            payload
                .interactive_tasks
                .as_deref()
                .unwrap_or("unavailable")
        ),
        format!(
            "prefcore_placements={}",
            payload
                .prefcore_placements
                .as_deref()
                .unwrap_or("unavailable")
        ),
        format!(
            "freq_ccd_placements={}",
            payload
                .freq_ccd_placements
                .as_deref()
                .unwrap_or("unavailable")
        ),
        format!("log_file={}", payload.log_file),
        String::new(),
    ]
    .join("\n")
}

fn state_root() -> PathBuf {
    if let Some(state_dir) = dirs::state_dir() {
        return state_dir.join(APP_NAME);
    }
    home_dir().join(".local/state").join(APP_NAME)
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn now() -> String {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown-time".to_string())
}

fn date_dir_component(timestamp: &str) -> &str {
    timestamp.split('T').next().unwrap_or("unknown-date")
}

fn timestamped_name(prefix: &str, ext: &str, timestamp: &str) -> String {
    let safe = timestamp.replace(':', "").replace('+', "-");
    format!("{prefix}-{safe}.{ext}")
}

fn read_file_trim(path: &str) -> String {
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unavailable".to_string())
}

fn read_command(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        bail!("command failed: {command}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn distribution_name() -> String {
    let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
    content
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|line| line.trim_matches('"').to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn first_cpu_model() -> String {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    cpuinfo
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(key, _)| key.trim() == "model name")
        })
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn detect_vcache_mode() -> String {
    let sysfs = Path::new("/sys/bus/platform/drivers/amd_x3d_vcache");
    let Ok(entries) = fs::read_dir(sysfs) else {
        return "unavailable".to_string();
    };

    for entry in entries.flatten() {
        let mode_path = entry.path().join("amd_x3d_mode");
        if mode_path.is_file() {
            return fs::read_to_string(mode_path)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "unavailable".to_string());
        }
    }

    "unavailable".to_string()
}

fn collect_embedded_reports() -> Vec<EmbeddedReport> {
    let mut reports = Vec::new();
    let mut candidates = find_report_files(&state_root().join("benchmarks"));

    if let Ok(repo_root) = detect_repo_root() {
        candidates.extend(find_report_files(&repo_root.join(DOCS_REPORT_DIR)));
    }

    candidates.sort();
    candidates.dedup();

    for file in candidates.into_iter().take(6) {
        if file.exists()
            && let Ok(contents) = fs::read_to_string(&file)
        {
            reports.push(EmbeddedReport {
                path: file.display().to_string(),
                contents,
            });
        }
    }

    reports
}

fn resolve_scheduler_binary(explicit: Option<&Path>, repo_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }

    if let Ok(path) = which::which(DEFAULT_SCHEDULER_NAME) {
        return Ok(path);
    }

    if let Some(repo_root) = repo_root {
        for candidate in [
            repo_root.join("target/x86_64-unknown-linux-gnu/release/scx_ghostbrew"),
            repo_root.join("target/release/scx_ghostbrew"),
        ] {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        return Ok(repo_root.join("target/x86_64-unknown-linux-gnu/release/scx_ghostbrew"));
    }

    Ok(PathBuf::from(DEFAULT_SCHEDULER_NAME))
}

fn find_report_files(root: &Path) -> Vec<PathBuf> {
    let mut reports = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return reports;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            reports.extend(find_report_files(&path));
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if (name.ends_with(".txt") || name.ends_with(".json"))
            && (name.contains(DEFAULT_BENCHMARK_REPORT_BASENAME)
                || name.contains("9950x3d-dev-report"))
        {
            reports.push(path);
        }
    }

    reports.sort_by(|a, b| b.cmp(a));
    reports
}

fn collect_recent_dmesg() -> Vec<String> {
    let Ok(output) = Command::new("sudo").arg("dmesg").output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("sched_ext")
                || lower.contains("scx_")
                || lower.contains("ghostbrew")
                || lower.contains("bpf")
                || lower.contains("verifier")
        })
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(ToOwned::to_owned)
        .collect()
}

fn detect_repo_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("failed to get current working directory")?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("src").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    bail!("could not locate GhostBrew repository root from current directory")
}

fn stat_line(stats_block: &str, name: &str) -> Option<String> {
    for line in stats_block.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix(&format!("{name}:")) {
            return Some(value.trim().to_string());
        }
    }
    None
}
