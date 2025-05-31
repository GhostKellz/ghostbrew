use std::fs;
use std::path::PathBuf;
use std::process::Command;
use chrono::Utc;
use crate::aur;
use tokio::runtime::Runtime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// Global cache for AUR/PKGBUILD data, used for async search and PKGBUILD preview
pub static AUR_CACHE: Lazy<Arc<Mutex<HashMap<String, aur::AurResult>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static PKGBUILD_CACHE: Lazy<Arc<Mutex<HashMap<String, String>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Shared helpers (scaffold)
pub fn completion(shell: &str) {
    println!("Generating completion for: {}", shell);
    // TODO: Output shell completion scripts
}

// PKGBUILD diff/audit before upgrade/install
pub fn pkgb_diff_audit(pkg: &str, new_pkgb: &str) {
    let history_dir = PathBuf::from(format!("$HOME/.local/share/ghostbrew/history/{}", pkg));
    let _ = fs::create_dir_all(&history_dir);
    let last_pkgb_path = history_dir.join("PKGBUILD.last");
    let last_pkgb = fs::read_to_string(&last_pkgb_path).unwrap_or_default();
    if !last_pkgb.is_empty() {
        println!("[ghostbrew] PKGBUILD diff for {}:", pkg);
        for diff in diff::lines(&last_pkgb, new_pkgb) {
            match diff {
                diff::Result::Left(l) => println!("- {}", l),
                diff::Result::Right(r) => println!("+ {}", r),
                diff::Result::Both(_, _) => {}
            }
        }
    }
    fs::write(&last_pkgb_path, new_pkgb).ok();
    // Audit for risky lines (existing logic)
    let risky = ["curl", "wget", "sudo", "rm -rf", "chmod", "chown", "dd", "mkfs", "mount", "scp", "nc", "ncat", "bash -c", "eval"];
    for keyword in risky.iter() {
        if new_pkgb.contains(keyword) {
            println!("[AUDIT][RISK] Found risky command: {}", keyword);
        }
    }
    log_to_file(&format!("Audited PKGBUILD for {} at {}", pkg, Utc::now().to_rfc3339()));
    // TODO: Call Lua for custom audit rules
}

// Rollback to previous package versions
pub fn rollback(pkg: &str) {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".local/share/ghostbrew/backups");
    let pkg_backup = backup_dir.join(format!("{}-backup.tar.zst", pkg));
    if !pkg_backup.exists() {
        println!("[ghostbrew] No backup found for {}", pkg);
        return;
    }
    println!("[ghostbrew] Rolling back {} from backup...", pkg);
    let status = std::process::Command::new("sudo")
        .arg("tar").arg("-xvf").arg(&pkg_backup)
        .arg("-C").arg("/")
        .status();
    if status.map(|s| s.success()).unwrap_or(false) {
        println!("[ghostbrew] Rollback complete for {}", pkg);
        log_to_file(&format!("Rolled back {} at {}", pkg, Utc::now().to_rfc3339()));
    } else {
        eprintln!("[ghostbrew] Rollback failed for {}", pkg);
    }
}

// Backup before install/upgrade
pub fn backup_package(pkg: &str) {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".local/share/ghostbrew/backups");
    let _ = fs::create_dir_all(&backup_dir);
    let backup_file = backup_dir.join(format!("{}-backup.tar.zst", pkg));
    let status = std::process::Command::new("sudo")
        .arg("tar").arg("-cvf").arg(&backup_file)
        .arg("/usr/bin/").arg(pkg)
        .status();
    if status.map(|s| s.success()).unwrap_or(false) {
        println!("[ghostbrew] Backup complete for {}", pkg);
        log_to_file(&format!("Backed up {} at {}", pkg, Utc::now().to_rfc3339()));
    } else {
        eprintln!("[ghostbrew] Backup failed for {}", pkg);
    }
}

// Logging utility
fn log_to_file(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Some(home) = dirs::home_dir() {
        let log_path = home.join(".local/share/ghostbrew/ghostbrew.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{}", msg);
        }
    }
}

// AUR comments/votes/changelog in TUI
pub fn aur_metadata(pkg: &str) -> (String, String, String) {
    // Query AUR RPC for comments/votes/changelog (stub)
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg={}", pkg);
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(json) = resp.json::<serde_json::Value>() {
            let votes = json["results"][0]["NumVotes"].to_string();
            let pop = json["results"][0]["Popularity"].to_string();
            let desc = json["results"][0]["Description"].to_string();
            return (votes, pop, desc);
        }
    }
    ("-".to_string(), "-".to_string(), "-".to_string())
}

// Flatpak/AppImage sandbox info in TUI
pub fn flatpak_sandbox_info(pkg: &str) -> String {
    let output = Command::new("flatpak").arg("info").arg(pkg).output();
    if let Ok(out) = output {
        let info = String::from_utf8_lossy(&out.stdout);
        if info.contains("sandbox") {
            return "[Flatpak] Sandboxed".to_string();
        } else {
            return "[Flatpak] Not sandboxed".to_string();
        }
    }
    "[Flatpak] Info unavailable".to_string()
}

// Async Rust for all network/disk IO (example for AUR search)
pub fn async_aur_search(query: &str) -> Vec<aur::AurResult> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let url = format!("https://aur.archlinux.org/rpc/?v=5&type=search&arg={}", query);
        if let Ok(resp) = reqwest::get(&url).await {
            if let Ok(json) = resp.json::<aur::AurResponse>().await {
                return json.results;
            }
        }
        vec![]
    })
}

// Async AUR search with caching
pub async fn async_aur_search_cached(query: &str) -> Vec<aur::AurResult> {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=search&arg={}", query);
    if let Ok(resp) = reqwest::get(&url).await {
        if let Ok(json) = resp.json::<aur::AurResponse>().await {
            let mut cache = AUR_CACHE.lock().unwrap();
            for pkg in &json.results {
                cache.insert(pkg.name.clone(), pkg.clone());
            }
            return json.results;
        }
    }
    vec![]
}

// Async PKGBUILD fetch with caching
pub async fn async_get_pkgbuild_cached(pkg: &str) -> String {
    {
        let cache = PKGBUILD_CACHE.lock().unwrap();
        if let Some(pkgb) = cache.get(pkg) {
            return pkgb.clone();
        }
    }
    let url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", pkg);
    if let Ok(resp) = reqwest::get(&url).await {
        if let Ok(text) = resp.text().await {
            let mut cache = PKGBUILD_CACHE.lock().unwrap();
            cache.insert(pkg.to_string(), text.clone());
            return text;
        }
    }
    String::new()
}
