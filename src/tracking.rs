use crate::constants;
use crate::convert;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Default)]
pub struct TrackingState {
    pub branch: String,
    pub global: BTreeMap<String, TrackedFile>,
    pub deployment: BTreeMap<String, TrackedFile>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackedFile {
    pub compact_path: String,
    pub packed_at_epoch: u64,
    pub compact_mtime_epoch: u64,
}

pub struct ChangedFile {
    pub xml_relative_path: String,
    pub compact_path: String,
}

pub enum ResetScope {
    Global,
    SinceDeploy,
}

pub fn get_current_branch() -> String {
    std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| constants::TRACKING_DEFAULT_BRANCH.to_string())
}

fn tracking_dir(compact_dir: &Path) -> PathBuf {
    compact_dir.join(constants::TRACKING_DIR)
}

fn branch_to_filename(branch: &str) -> String {
    format!(
        "{}.json",
        branch.replace('/', constants::TRACKING_BRANCH_SEPARATOR)
    )
}

fn tracking_file_path(compact_dir: &Path, branch: &str) -> PathBuf {
    tracking_dir(compact_dir).join(branch_to_filename(branch))
}

pub fn load_tracking(compact_dir: &Path) -> Result<TrackingState> {
    let branch = get_current_branch();
    let path = tracking_file_path(compact_dir, &branch);

    if !path.exists() {
        return Ok(TrackingState {
            branch,
            ..Default::default()
        });
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Warning: could not read tracking file {}: {e}",
                path.display()
            );
            return Ok(TrackingState {
                branch,
                ..Default::default()
            });
        }
    };

    match serde_json::from_str::<TrackingState>(&content) {
        Ok(mut state) => {
            state.branch = branch;
            Ok(state)
        }
        Err(e) => {
            eprintln!("Warning: corrupted tracking file {}: {e}", path.display());
            Ok(TrackingState {
                branch,
                ..Default::default()
            })
        }
    }
}

pub fn save_tracking(state: &TrackingState, compact_dir: &Path) -> Result<()> {
    let dir = tracking_dir(compact_dir);
    fs::create_dir_all(&dir)?;
    let path = tracking_file_path(compact_dir, &state.branch);
    let json = serde_json::to_string_pretty(state)?;
    fs::write(&path, json)?;
    Ok(())
}

fn file_mtime_epoch(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Record packed files into tracking state. Accepts `convert::PackedFileInfo` directly.
pub fn record_packed_files(compact_dir: &Path, files: &[convert::PackedFileInfo]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let mut state = load_tracking(compact_dir)?;
    let now = now_epoch();

    for f in files {
        let mtime = file_mtime_epoch(&f.compact_absolute_path).unwrap_or(now);
        let entry = TrackedFile {
            compact_path: f
                .compact_absolute_path
                .strip_prefix(compact_dir)
                .unwrap_or(&f.compact_absolute_path)
                .to_string_lossy()
                .to_string(),
            packed_at_epoch: now,
            compact_mtime_epoch: mtime,
        };
        state
            .global
            .insert(f.xml_relative_path.clone(), entry.clone());
        state.deployment.insert(f.xml_relative_path.clone(), entry);
    }

    save_tracking(&state, compact_dir)?;
    Ok(())
}

/// Convenience: record tracking from ConvertStats after a successful pack.
/// Non-fatal — logs warning on error.
pub fn record_pack_result(output: &Path, stats: &convert::ConvertStats) {
    if stats.packed_files.is_empty() {
        return;
    }
    if let Err(e) = record_packed_files(output, &stats.packed_files) {
        eprintln!("Warning: failed to update tracking: {e}");
    }
}

pub fn detect_changes(compact_dir: &Path, since_deploy: bool) -> Result<Vec<ChangedFile>> {
    if !compact_dir.exists() {
        return Ok(Vec::new());
    }

    let state = load_tracking(compact_dir)?;
    let map = if since_deploy {
        &state.deployment
    } else {
        &state.global
    };

    let mut changed = Vec::new();

    // 1. Check tracked files for modifications
    for (xml_path, tracked) in map {
        let full_compact = compact_dir.join(&tracked.compact_path);
        if !full_compact.exists() {
            continue;
        }
        if let Some(current_mtime) = file_mtime_epoch(&full_compact) {
            if current_mtime > tracked.compact_mtime_epoch {
                changed.push(ChangedFile {
                    xml_relative_path: xml_path.clone(),
                    compact_path: tracked.compact_path.clone(),
                });
            }
        }
    }

    // 2. Scan for new compact files not known to tracking.
    // Only scan when tracking has entries (skip after reset to avoid false positives).
    let all_known: std::collections::HashSet<&str> = state
        .global
        .values()
        .chain(state.deployment.values())
        .map(|t| t.compact_path.as_str())
        .collect();

    if all_known.is_empty() {
        return Ok(changed);
    }

    for entry in walkdir::WalkDir::new(compact_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !name.ends_with(constants::SUFFIX_META_YAML)
            && !name.ends_with(constants::SUFFIX_META_JSON)
        {
            continue;
        }
        // Skip tracking directory itself
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str() == constants::TRACKING_DIR)
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(compact_dir)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();
        if !all_known.contains(relative.as_str()) {
            // Derive XML relative path from compact relative path
            let xml_rel = relative
                .replace(constants::SUFFIX_META_YAML, constants::SUFFIX_META_XML)
                .replace(constants::SUFFIX_META_JSON, constants::SUFFIX_META_XML);
            changed.push(ChangedFile {
                xml_relative_path: xml_rel,
                compact_path: relative,
            });
        }
    }

    Ok(changed)
}

pub fn reset_tracking(compact_dir: &Path, scope: ResetScope) -> Result<()> {
    let mut state = load_tracking(compact_dir)?;

    match scope {
        ResetScope::Global => {
            state.global.clear();
            state.deployment.clear();
        }
        ResetScope::SinceDeploy => {
            // Snapshot current file mtimes as the new baseline so future
            // modifications are detected relative to "now", not lost entirely.
            let now = now_epoch();
            state.deployment = state
                .global
                .iter()
                .map(|(key, tracked)| {
                    let current_mtime = file_mtime_epoch(&compact_dir.join(&tracked.compact_path))
                        .unwrap_or(now);
                    (
                        key.clone(),
                        TrackedFile {
                            compact_path: tracked.compact_path.clone(),
                            packed_at_epoch: now,
                            compact_mtime_epoch: current_mtime,
                        },
                    )
                })
                .collect();
        }
    }

    save_tracking(&state, compact_dir)?;
    Ok(())
}
