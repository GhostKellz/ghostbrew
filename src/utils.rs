use crate::aur;
use crate::gpg;
use chrono::Utc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
/// Global cache for AUR/PKGBUILD data, used for async search and PKGBUILD preview
pub static AUR_CACHE: Lazy<Arc<Mutex<HashMap<String, aur::AurResult>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
// Example usage: cache AUR results in async_aur_search_cached and PKGBUILD preview
#[allow(dead_code)]
pub static PKGBUILD_CACHE: Lazy<Arc<Mutex<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Shared helpers (scaffold)
pub fn completion(shell: &str) {
    match shell {
        "bash" => println!("source <(ghostbrew completion bash)"),
        "zsh" => println!(
            "compdef _ghostbrew ghostbrew; ghostbrew completion zsh >| $fpath[1]/_ghostbrew"
        ),
        "fish" => println!("ghostbrew completion fish | source"),
        _ => println!("[ghostbrew] Supported shells: bash, zsh, fish"),
    }
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
    let risky = [
        "curl", "wget", "sudo", "rm -rf", "chmod", "chown", "dd", "mkfs", "mount", "scp", "nc",
        "ncat", "bash -c", "eval",
    ];
    for keyword in risky.iter() {
        if new_pkgb.contains(keyword) {
            println!("[AUDIT][RISK] Found risky command: {}", keyword);
        }
    }
    log_to_file(&format!(
        "Audited PKGBUILD for {} at {}",
        pkg,
        Utc::now().to_rfc3339()
    ));
    // TODO: Call Lua for custom audit rules
}

// Rollback to previous package versions
pub fn rollback(pkg: &str) {
    let backup_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/ghostbrew/backups");
    let pkg_backup = backup_dir.join(format!("{}-backup.tar.zst", pkg));
    if !pkg_backup.exists() {
        println!("[ghostbrew] No backup found for {}", pkg);
        return;
    }
    println!("[ghostbrew] Rolling back {} from backup...", pkg);
    let status = std::process::Command::new("sudo")
        .arg("tar")
        .arg("-xvf")
        .arg(&pkg_backup)
        .arg("-C")
        .arg("/")
        .status();
    if status.map(|s| s.success()).unwrap_or(false) {
        println!("[ghostbrew] Rollback complete for {}", pkg);
        log_to_file(&format!(
            "Rolled back {} at {}",
            pkg,
            Utc::now().to_rfc3339()
        ));
    } else {
        eprintln!("[ghostbrew] Rollback failed for {}", pkg);
    }
}

// Rollback PKGBUILD to previous version
pub fn rollback_pkgbuild(pkg: &str) {
    let history_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/ghostbrew/history/")
        .join(pkg);
    let last_pkgb_path = history_dir.join("PKGBUILD.last");
    let backup_pkgb_path = history_dir.join("PKGBUILD.backup");

    if !backup_pkgb_path.exists() {
        println!("[ghostbrew] No backup PKGBUILD found for {}.", pkg);
        return;
    }

    if let Err(e) = std::fs::copy(&backup_pkgb_path, &last_pkgb_path) {
        eprintln!("[ghostbrew] Failed to rollback PKGBUILD for {}: {}", pkg, e);
    } else {
        println!("[ghostbrew] Successfully rolled back PKGBUILD for {}.", pkg);
    }
}

// Example CLI usage for rollback_pkgbuild and set_keyserver
pub fn cli_rollback_pkgbuild(pkg: &str) {
    rollback_pkgbuild(pkg);
}

pub fn cli_set_keyserver(keyserver: &str) {
    gpg::set_keyserver(keyserver);
}

// Backup before install/upgrade
pub fn backup_package(pkg: &str) {
    let backup_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/ghostbrew/backups");
    let _ = fs::create_dir_all(&backup_dir);
    let backup_file = backup_dir.join(format!("{}-backup.tar.zst", pkg));
    let status = std::process::Command::new("sudo")
        .arg("tar")
        .arg("-cvf")
        .arg(&backup_file)
        .arg("/usr/bin/")
        .arg(pkg)
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

#[allow(dead_code)]
// AUR comments/votes/changelog in TUI
pub fn aur_metadata(pkg: &str) -> (String, String, String) {
    // Fetch AUR metadata (votes, popularity, maintainer) for a package
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg={}", pkg);
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(json) = resp.json::<serde_json::Value>() {
            let votes = json["results"][0]["NumVotes"]
                .as_i64()
                .unwrap_or(0)
                .to_string();
            let pop = json["results"][0]["Popularity"]
                .as_f64()
                .unwrap_or(0.0)
                .to_string();
            let maint = json["results"][0]["Maintainer"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return (votes, pop, maint);
        }
    }
    ("0".to_string(), "0.0".to_string(), "".to_string())
}
// Example usage: show AUR metadata in TUI/CLI details

#[allow(dead_code)]
// Flatpak/AppImage sandbox info in TUI
pub fn flatpak_sandbox_info(pkg: &str) -> String {
    let output = std::process::Command::new("flatpak")
        .arg("info")
        .arg(pkg)
        .output();
    if let Ok(out) = output {
        let info = String::from_utf8_lossy(&out.stdout);
        if info.contains("sandbox: none") {
            return format!("[ghostbrew] Warning: Flatpak {} is NOT sandboxed!", pkg);
        } else {
            return format!("[ghostbrew] Flatpak sandbox info for {}:\n{}", pkg, info);
        }
    }
    String::from("[ghostbrew] Could not get Flatpak sandbox info.")
}
// Example usage: show Flatpak sandbox info in TUI/CLI details

#[allow(dead_code)]
// Async Rust for all network/disk IO (example for AUR search)
pub fn async_aur_search(query: &str) -> Vec<aur::AurResult> {
    // Async AUR search for TUI responsiveness
    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        query
    );
    let client = reqwest::blocking::Client::new();
    if let Ok(resp) = client.get(&url).send() {
        if let Ok(json) = resp.json::<aur::AurResponse>() {
            return json.results;
        }
    }
    vec![]
}
// Example usage: use in TUI for async search

#[allow(dead_code)]
// Async AUR search with caching
pub async fn async_aur_search_cached(query: &str) -> Vec<aur::AurResult> {
    // Async AUR search with caching for TUI
    use crate::utils::AUR_CACHE;
    let cache = AUR_CACHE.lock().unwrap();
    if let Some(cached) = cache.get(query) {
        return vec![cached.clone()];
    }
    drop(cache);
    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        query
    );
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(json) = resp.json::<aur::AurResponse>() {
            let mut cache = AUR_CACHE.lock().unwrap();
            for result in &json.results {
                cache.insert(result.name.clone(), result.clone());
            }
            return json.results;
        }
    }
    vec![]
}
// Example usage: use in TUI for async search with caching

#[allow(dead_code)]
// Async PKGBUILD fetch with caching
pub async fn async_get_pkgbuild_cached(pkg: &str) -> String {
    {
        let cache = PKGBUILD_CACHE.lock().unwrap();
        if let Some(pkgb) = cache.get(pkg) {
            return pkgb.clone();
        }
    }
    let url = format!(
        "https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}",
        pkg
    );
    if let Ok(resp) = reqwest::get(&url).await {
        if let Ok(text) = resp.text().await {
            let mut cache = PKGBUILD_CACHE.lock().unwrap();
            cache.insert(pkg.to_string(), text.clone());
            return text;
        }
    }
    String::new()
}
