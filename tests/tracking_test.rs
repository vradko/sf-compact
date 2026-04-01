use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

// ── Unit-level: tracking file creation after pack ──────────────────

#[test]
fn pack_creates_tracking_file() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run pack");
    assert!(
        output.status.success(),
        "pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // .tracking directory should exist
    let tracking_dir = packed.path().join(".tracking");
    assert!(
        tracking_dir.exists(),
        ".tracking directory not created after pack"
    );

    // Should have exactly one JSON file
    let entries: Vec<_> = fs::read_dir(&tracking_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert!(!entries.is_empty(), "no tracking JSON file created");

    // Parse tracking file
    let content = fs::read_to_string(entries[0].path()).unwrap();
    let state: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(state["branch"].is_string(), "branch field missing");
    assert!(state["global"].is_object(), "global map missing");
    assert!(state["deployment"].is_object(), "deployment map missing");

    // Should have entries for packed files
    let global = state["global"].as_object().unwrap();
    assert!(
        !global.is_empty(),
        "global tracking should have entries after pack"
    );

    // Each entry should have required fields
    for (_key, entry) in global {
        assert!(entry["compact_path"].is_string(), "compact_path missing");
        assert!(entry["packed_at_epoch"].is_u64(), "packed_at_epoch missing");
        assert!(
            entry["compact_mtime_epoch"].is_u64(),
            "compact_mtime_epoch missing"
        );
    }
}

// ── changes command: no changes right after pack ───────────────────

#[test]
fn changes_empty_after_fresh_pack() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack first
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Check changes — should be empty
    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No compact files modified"),
        "expected no changes after fresh pack, got: {stdout}"
    );
}

// ── changes detects modified compact file ──────────────────────────

#[test]
fn changes_detects_modified_file() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Wait a moment so mtime differs
    thread::sleep(Duration::from_millis(1100));

    // Find a compact file and modify it
    let compact_file = find_first_compact_file(packed.path());
    assert!(compact_file.is_some(), "no compact file found after pack");
    let compact_file = compact_file.unwrap();

    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n")).unwrap();

    // Check changes — should show modified file
    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("modified globally"),
        "expected changes detected, got: {stdout}"
    );
    assert!(stdout.contains("M "), "expected M prefix for modified file");
}

// ── changes detects new compact file added after pack ────────────

#[test]
fn changes_detects_new_compact_file() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Add a new compact file that wasn't part of the pack
    let new_file = packed
        .path()
        .join("main/default/classes/NewClass.cls-meta.json");
    fs::create_dir_all(new_file.parent().unwrap()).unwrap();
    fs::write(
        &new_file,
        r#"{"_tag":"ApexClass","_ns":"http://soap.sforce.com/2006/04/metadata","apiVersion":"59.0","status":"Active"}"#,
    )
    .unwrap();

    // changes should detect the new file
    let output = sf_compact()
        .args(["changes", "--json", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    let arr = parsed["global"].as_array().unwrap();
    assert!(
        arr.iter().any(|e| e["compact_path"]
            .as_str()
            .is_some_and(|p| p.contains("NewClass"))),
        "expected new file in changes output, got: {stdout}"
    );
}

// ── changes --json output ──────────────────────────────────────────

#[test]
fn changes_json_output() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Wait and modify
    thread::sleep(Duration::from_millis(1100));
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n")).unwrap();

    // Check changes --json
    let output = sf_compact()
        .args(["changes", "--json", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON output: {e}\n{stdout}"));
    assert!(
        parsed["global"].is_array(),
        "expected global array in JSON output"
    );
    let arr = parsed["global"].as_array().unwrap();
    assert!(!arr.is_empty(), "expected at least one change in JSON");
    assert!(
        arr[0]["xml_path"].is_string(),
        "xml_path missing from JSON entry"
    );
    assert!(
        arr[0]["compact_path"].is_string(),
        "compact_path missing from JSON entry"
    );
}

// ── changes --since-deploy ─────────────────────────────────────────

#[test]
fn changes_since_deploy_tracks_independently() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Modify a file
    thread::sleep(Duration::from_millis(1100));
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n")).unwrap();

    // Both global and since-deploy should show changes
    let output = sf_compact()
        .args([
            "changes",
            "--since-deploy",
            "--json",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        !parsed["deployment"].as_array().unwrap().is_empty(),
        "deployment should have changes"
    );

    // Reset deployment only
    let output = sf_compact()
        .args([
            "changes",
            "-o",
            packed.path().to_str().unwrap(),
            "reset",
            "--since-deploy",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Reset deployment tracking"),
        "unexpected reset output: {stdout}"
    );

    // since-deploy should now be empty
    let output = sf_compact()
        .args([
            "changes",
            "--since-deploy",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No compact files modified"),
        "deployment should be empty after reset, got: {stdout}"
    );

    // But global should still show changes
    let output = sf_compact()
        .args(["changes", "--json", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        !parsed["global"].as_array().unwrap().is_empty(),
        "global should still have changes after deploy reset"
    );
}

// ── reset --global clears everything ───────────────────────────────

#[test]
fn reset_global_clears_all() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack and modify
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    thread::sleep(Duration::from_millis(1100));
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n")).unwrap();

    // Reset global
    let output = sf_compact()
        .args([
            "changes",
            "-o",
            packed.path().to_str().unwrap(),
            "reset",
            "--global",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Reset global tracking"),
        "unexpected output: {stdout}"
    );

    // Both should be empty
    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No compact files modified"),
        "global should be empty: {stdout}"
    );

    let output = sf_compact()
        .args([
            "changes",
            "--since-deploy",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No compact files modified"),
        "deployment should be empty: {stdout}"
    );
}

// ── reset requires scope flag ──────────────────────────────────────

#[test]
fn reset_without_scope_fails() {
    let packed = tempfile::tempdir().unwrap();
    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap(), "reset"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "reset without --global or --since-deploy should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--global") || stderr.contains("--since-deploy"),
        "error should mention required flags: {stderr}"
    );
}

// ── changes on nonexistent compact dir ─────────────────────────────

#[test]
fn changes_nonexistent_dir_shows_empty() {
    let output = sf_compact()
        .args(["changes", "-o", "/tmp/sf-compact-nonexistent-dir-test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No compact files modified"),
        "should show empty: {stdout}"
    );
}

// ── incremental pack only tracks actually packed files ─────────────

#[test]
fn incremental_pack_only_tracks_new_files() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Full pack
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Read tracking state, count entries
    let tracking_dir = packed.path().join(".tracking");
    let first_state = read_tracking_state(&tracking_dir);
    let initial_count = first_state["global"].as_object().unwrap().len();
    assert!(initial_count > 0);

    // Reset and do incremental pack (nothing changed → 0 files packed)
    sf_compact()
        .args([
            "changes",
            "-o",
            packed.path().to_str().unwrap(),
            "reset",
            "--global",
        ])
        .output()
        .unwrap();

    let output = sf_compact()
        .args([
            "pack",
            "--incremental",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 0 files"),
        "incremental should skip unchanged files: {stdout}"
    );

    // Tracking should remain empty (no files were actually packed)
    let state = read_tracking_state(&tracking_dir);
    assert!(
        state["global"].as_object().unwrap().is_empty(),
        "tracking should be empty after incremental pack with no changes"
    );
}

// ── suggest deploy/retrieve commands ───────────────────────────────

#[test]
fn changes_suggests_deploy_commands() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    thread::sleep(Duration::from_millis(1100));
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n")).unwrap();

    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("sf project deploy start"),
        "should suggest deploy command: {stdout}"
    );
    assert!(
        stdout.contains("sf project retrieve start"),
        "should suggest retrieve command: {stdout}"
    );
}

// ── deleted compact file doesn't crash ─────────────────────────────

#[test]
fn changes_handles_deleted_compact_file() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Delete a compact file
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    fs::remove_file(&compact_file).unwrap();

    // changes should not crash
    let output = sf_compact()
        .args(["changes", "-o", packed.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "changes should handle deleted files gracefully"
    );
}

// ── tracking file structure ────────────────────────────────────────

#[test]
fn tracking_file_has_correct_structure() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let state = read_tracking_state(&packed.path().join(".tracking"));

    // Branch should be a non-empty string
    let branch = state["branch"].as_str().unwrap();
    assert!(!branch.is_empty(), "branch should not be empty");

    // Global and deployment should have the same entries after fresh pack
    let global_keys: Vec<&str> = state["global"]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    let deploy_keys: Vec<&str> = state["deployment"]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        global_keys, deploy_keys,
        "global and deployment should match after fresh pack"
    );

    // Keys should be XML relative paths
    for key in &global_keys {
        assert!(
            key.ends_with("-meta.xml"),
            "tracking key should be XML path: {key}"
        );
    }
}

// ── regression: modify existing file after reset --since-deploy ───

#[test]
fn changes_since_deploy_detects_modifications_after_reset() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack
    sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Reset deployment tracking
    sf_compact()
        .args([
            "changes",
            "-o",
            packed.path().to_str().unwrap(),
            "reset",
            "--since-deploy",
        ])
        .output()
        .unwrap();

    // Wait so mtime differs
    thread::sleep(Duration::from_millis(1100));

    // Modify an existing compact file
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = fs::read_to_string(&compact_file).unwrap();
    fs::write(&compact_file, format!("{content}\n# modified")).unwrap();

    // changes --since-deploy should detect the modification
    let output = sf_compact()
        .args([
            "changes",
            "--since-deploy",
            "--json",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    let arr = parsed["deployment"].as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "since-deploy should detect modifications after reset, got: {stdout}"
    );
}

// ── helpers ────────────────────────────────────────────────────────

fn find_first_compact_file(dir: &Path) -> Option<std::path::PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && {
                let name = e.file_name().to_string_lossy();
                (name.ends_with("-meta.json") || name.ends_with("-meta.yaml"))
                    && !e.path().components().any(|c| c.as_os_str() == ".tracking")
            }
        })
        .map(|e| e.path().to_path_buf())
        .next()
}

fn read_tracking_state(tracking_dir: &Path) -> serde_json::Value {
    let entry = fs::read_dir(tracking_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .expect("no tracking file found");
    let content = fs::read_to_string(entry.path()).unwrap();
    serde_json::from_str(&content).unwrap()
}
