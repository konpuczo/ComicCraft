use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use eframe::egui;

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
        name: String,
        notes: String,
        download_url: String,
        zip_url: Option<String>,
    },
    Downloading {
        progress: f32,
    },
    ReadyToRestart {
        path: PathBuf,
    },
    Error(String),
}

#[derive(Clone)]
pub struct AppUpdater {
    pub status: Arc<Mutex<UpdateStatus>>,
    pub is_dialog_open: Arc<AtomicBool>,
}

impl Default for AppUpdater {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            is_dialog_open: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AppUpdater {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_dialog(&self) {
        self.is_dialog_open.store(true, Ordering::SeqCst);
    }

    pub fn close_dialog(&self) {
        self.is_dialog_open.store(false, Ordering::SeqCst);
    }

    pub fn is_open(&self) -> bool {
        self.is_dialog_open.load(Ordering::SeqCst)
    }

    pub fn get_status(&self) -> UpdateStatus {
        self.status.lock().clone()
    }

    /// Spawns a background worker to query GitHub Releases API for ComicCraft updates
    pub fn check_for_updates(&self, ctx: egui::Context) {
        let status_arc = self.status.clone();
        *status_arc.lock() = UpdateStatus::Checking;
        ctx.request_repaint();

        std::thread::spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("ComicCraft-Studio-Updater")
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(e.to_string());
                    ctx.request_repaint();
                    return;
                }
            };

            let url = "https://api.github.com/repos/konpuczo/ComicCraft/releases/latest";
            let resp = match client.get(url).send() {
                Ok(r) => r,
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(format!("Network error: {}", e));
                    ctx.request_repaint();
                    return;
                }
            };

            // A 403 rate-limit or 404 (no releases yet) still returns parseable JSON,
            // just without `tag_name`. Without this guard that empty tag flows into
            // is_newer_semver and used to render a bogus "update available" dialog.
            if !resp.status().is_success() {
                *status_arc.lock() = UpdateStatus::Error(format!(
                    "GitHub returned HTTP {} when checking for updates",
                    resp.status().as_u16()
                ));
                ctx.request_repaint();
                return;
            }

            if let Ok(v) = resp.json::<serde_json::Value>() {
                let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or("").trim_start_matches('v');
                let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("Latest Release").to_string();
                let body = v.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string();
                let html_url = v.get("html_url").and_then(|u| u.as_str()).unwrap_or("https://github.com/konpuczo/ComicCraft/releases").to_string();

                let mut zip_url = None;
                if let Some(assets) = v.get("assets").and_then(|a| a.as_array()) {
                    for asset in assets {
                        if let Some(aname) = asset.get("name").and_then(|n| n.as_str()) {
                            if aname.ends_with(".zip") || aname.ends_with(".exe") {
                                zip_url = asset.get("browser_download_url").and_then(|u| u.as_str()).map(|s| s.to_string());
                                break;
                            }
                        }
                    }
                }

                let current_ver = env!("CARGO_PKG_VERSION");
                if is_newer_semver(tag, current_ver) {
                    *status_arc.lock() = UpdateStatus::Available {
                        version: format!("v{}", tag),
                        name,
                        notes: body,
                        download_url: html_url,
                        zip_url,
                    };
                } else {
                    *status_arc.lock() = UpdateStatus::UpToDate;
                }
            } else {
                *status_arc.lock() = UpdateStatus::Error("Failed to parse release information from GitHub".to_string());
            }
            ctx.request_repaint();
        });
    }

    /// Downloads the update package into a local temp directory
    pub fn download_update(&self, download_url: String, ctx: egui::Context) {
        let status_arc = self.status.clone();
        *status_arc.lock() = UpdateStatus::Downloading { progress: 0.05 };
        ctx.request_repaint();

        std::thread::spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(180))
                .user_agent("ComicCraft-Studio-Updater")
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(e.to_string());
                    ctx.request_repaint();
                    return;
                }
            };

            let mut resp = match client.get(&download_url).send() {
                Ok(r) => r,
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(format!("Download failed: {}", e));
                    ctx.request_repaint();
                    return;
                }
            };

            let total_size = resp.content_length().unwrap_or(15_000_000);
            let temp_dir = std::env::temp_dir().join("comiccraft_updates");
            let _ = std::fs::create_dir_all(&temp_dir);
            let file_name = download_url.split('/').last().unwrap_or("ComicCraft_Update.zip");
            let out_path = temp_dir.join(file_name);

            let mut file = match std::fs::File::create(&out_path) {
                Ok(f) => f,
                Err(e) => {
                    *status_arc.lock() = UpdateStatus::Error(format!("Cannot create file: {}", e));
                    ctx.request_repaint();
                    return;
                }
            };

            use std::io::{Read, Write};
            let mut buffer = [0u8; 65536];
            let mut downloaded = 0u64;

            loop {
                match resp.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        downloaded += n as u64;
                        let _ = file.write_all(&buffer[..n]);
                        let prog = (downloaded as f32 / total_size as f32).clamp(0.05, 0.99);
                        *status_arc.lock() = UpdateStatus::Downloading { progress: prog };
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        *status_arc.lock() = UpdateStatus::Error(format!("Read error during download: {}", e));
                        ctx.request_repaint();
                        return;
                    }
                }
            }

            *status_arc.lock() = UpdateStatus::ReadyToRestart { path: out_path };
            ctx.request_repaint();
        });
    }
}

pub fn is_newer_semver(remote: &str, current: &str) -> bool {
    let parse_nums = |s: &str| -> Vec<u32> {
        let clean = s.split('-').next().unwrap_or(s).trim_start_matches('v');
        clean.split('.').filter_map(|p| p.parse::<u32>().ok()).collect()
    };
    let r = parse_nums(remote);
    let c = parse_nums(current);
    if r.is_empty() {
        return false;
    }
    if c.is_empty() {
        return true; 
    }
    for (rv, cv) in r.iter().zip(c.iter()) {
        if rv > cv {
            return true;
        }
        if rv < cv {
            return false;
        }
    }
    r.len() > c.len()
}

#[cfg(test)]
mod tests {
    use super::is_newer_semver;

    #[test]
    fn semver_comparison_table() {
        const CASES: &[(&str, &str, bool)] = &[
            // Malformed / missing tags must NEVER claim an update (regression: empty
            // tag previously hit the `remote != current` fallback and returned true).
            ("", "0.1.1", false),
            ("v", "0.1.1", false),
            ("not-a-version", "0.1.1", false),
            ("release-2026", "0.1.1", false),
            // Equal versions are never "newer".
            ("0.1.1", "0.1.1", false),
            ("v0.1.1", "0.1.1", false),
            // Real upgrades.
            ("0.1.2", "0.1.1", true),
            ("v0.2.0", "0.1.1", true),
            ("1.0.0", "0.1.1", true),
            ("0.1.1.1", "0.1.1", true),
            ("1.0", "0.1.1", true),
            // Downgrades / shorter-but-equal prefixes.
            ("0.1.0", "0.1.1", false),
            ("0.1", "0.1.1", false),
            ("0.0.9", "0.1.0", false),
            // Prerelease suffix is stripped before comparison.
            ("0.1.1-rc1", "0.1.0", true),
            // Corrupt local version with a valid remote tag → offer the update.
            ("0.1.1", "", true),
        ];
        for (remote, current, expected) in CASES {
            assert_eq!(
                is_newer_semver(remote, current),
                *expected,
                "is_newer_semver({remote:?}, {current:?})"
            );
        }
    }
}
