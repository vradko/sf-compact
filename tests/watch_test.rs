use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

/// Copy fixture XML files to a temp source dir (watch needs a writable source).
fn setup_source(fixtures: &Path) -> tempfile::TempDir {
    let source = tempfile::tempdir().unwrap();
    copy_dir_recursive(fixtures, source.path());
    source
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).ok();
        } else {
            fs::create_dir_all(target.parent().unwrap()).ok();
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn find_compact_file(dir: &Path) -> Option<std::path::PathBuf> {
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

// ── watch performs initial pack ──────────────────────────────────────

#[test]
fn watch_initial_pack_creates_output() {
    let fixtures = Path::new("tests/fixtures/force-app");
    let source = setup_source(fixtures);
    let output = tempfile::tempdir().unwrap();

    let mut child = sf_compact()
        .args([
            "watch",
            source.path().to_str().unwrap(),
            "-o",
            output.path().to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for initial pack to complete
    thread::sleep(Duration::from_secs(3));

    // Kill the watcher
    child.kill().ok();
    child.wait().ok();

    // Verify compact files were created
    let compact = find_compact_file(output.path());
    assert!(
        compact.is_some(),
        "watch should create compact files on initial pack"
    );
}

// ── watch creates tracking state ─────────────────────────────────────

#[test]
fn watch_initial_pack_creates_tracking() {
    let fixtures = Path::new("tests/fixtures/force-app");
    let source = setup_source(fixtures);
    let output = tempfile::tempdir().unwrap();

    let mut child = sf_compact()
        .args([
            "watch",
            source.path().to_str().unwrap(),
            "-o",
            output.path().to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    thread::sleep(Duration::from_secs(3));

    child.kill().ok();
    child.wait().ok();

    // Verify tracking directory exists
    let tracking_dir = output.path().join(".tracking");
    assert!(
        tracking_dir.exists(),
        "watch should create .tracking/ after initial pack"
    );

    // Verify tracking file has entries
    let tracking_files: Vec<_> = fs::read_dir(&tracking_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert!(
        !tracking_files.is_empty(),
        "tracking dir should contain a branch tracking file"
    );
}

// ── watch repacks on file change ─────────────────────────────────────

#[test]
fn watch_repacks_on_xml_change() {
    let fixtures = Path::new("tests/fixtures/force-app");
    let source = setup_source(fixtures);
    let output = tempfile::tempdir().unwrap();

    let mut child = sf_compact()
        .args([
            "watch",
            source.path().to_str().unwrap(),
            "-o",
            output.path().to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for initial pack
    thread::sleep(Duration::from_secs(3));

    // Find the profile compact file and record its content
    let profile_compact = walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_type().is_file() && e.file_name().to_string_lossy().contains(".profile-meta.")
        })
        .map(|e| e.path().to_path_buf())
        .expect("profile compact file should exist after initial pack");
    let original_content = fs::read_to_string(&profile_compact).unwrap();

    // Find the corresponding source XML and modify it
    let xml_path = walkdir::WalkDir::new(source.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_type().is_file()
                && e.file_name()
                    .to_string_lossy()
                    .ends_with(".profile-meta.xml")
        })
        .map(|e| e.path().to_path_buf())
        .expect("profile XML should exist in source");

    let xml_content = fs::read_to_string(&xml_path).unwrap();
    let modified = xml_content.replace(
        "<userLicense>Salesforce</userLicense>",
        "<userLicense>Salesforce Platform</userLicense>",
    );
    fs::write(&xml_path, modified).unwrap();

    // Wait for debouncer (500ms) + repack time
    thread::sleep(Duration::from_secs(3));

    child.kill().ok();
    let wait_result = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&wait_result.stderr);

    // Verify repack happened
    assert!(
        stderr.contains("Repacked"),
        "watch should repack after XML change, stderr: {stderr}"
    );

    // Verify the compact file was updated
    let new_content = fs::read_to_string(&profile_compact).unwrap();
    assert_ne!(
        original_content, new_content,
        "compact file should be updated after XML change"
    );
}

// ── watch ignores non-XML changes ────────────────────────────────────

#[test]
fn watch_ignores_non_xml_changes() {
    let fixtures = Path::new("tests/fixtures/force-app");
    let source = setup_source(fixtures);
    let output = tempfile::tempdir().unwrap();

    let mut child = sf_compact()
        .args([
            "watch",
            source.path().to_str().unwrap(),
            "-o",
            output.path().to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for initial pack
    thread::sleep(Duration::from_secs(3));

    // Create a non-XML file in the source directory
    fs::write(source.path().join("random.txt"), "not xml").unwrap();

    // Wait for debouncer
    thread::sleep(Duration::from_secs(2));

    child.kill().ok();
    let wait_result = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&wait_result.stderr);

    // Count pack operations — should only be the initial one
    let repack_count = stderr.matches("Repacked").count();
    assert_eq!(
        repack_count, 0,
        "non-XML changes should not trigger repack, but got {} repacks. stderr: {}",
        repack_count, stderr
    );
}
