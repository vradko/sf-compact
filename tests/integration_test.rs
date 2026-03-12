use std::path::Path;
use std::process::Command;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

#[test]
fn pack_and_unpack_roundtrip() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack XML → YAML
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 5 files"),
        "unexpected pack output: {stdout}"
    );

    // Verify YAML files were created
    let yaml_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.yaml");
    assert!(yaml_profile.exists(), "YAML profile not created");

    // Unpack YAML → XML
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run unpack");
    assert!(
        output.status.success(),
        "unpack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 5 files"),
        "unexpected unpack output: {stdout}"
    );

    // Verify XML files were restored
    let xml_profile = unpacked
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.xml");
    assert!(xml_profile.exists(), "XML profile not restored");

    // Verify content is valid XML with expected root element
    let content = std::fs::read_to_string(&xml_profile).unwrap();
    assert!(
        content.contains("<Profile"),
        "restored XML missing Profile tag"
    );
    assert!(
        content.contains("http://soap.sforce.com/2006/04/metadata"),
        "restored XML missing namespace"
    );
}

#[test]
fn stats_shows_token_savings() {
    let output = sf_compact()
        .args(["stats", "tests/fixtures"])
        .output()
        .expect("failed to run stats");
    assert!(
        output.status.success(),
        "stats failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Preview:"), "missing preview header");
    assert!(stdout.contains("Would save"), "missing savings summary");
    assert!(stdout.contains("profile"), "missing profile type breakdown");
}

#[test]
fn stats_with_include_filter() {
    let output = sf_compact()
        .args(["stats", "tests/fixtures", "--include", "*.profile-meta.xml"])
        .output()
        .expect("failed to run stats with include");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 files"),
        "should only match 1 profile file"
    );
    assert!(stdout.contains("profile"), "should show profile type");
    // Should NOT contain other types
    assert!(!stdout.contains("  flow "), "should not contain flow type");
}

#[test]
fn stats_per_file_breakdown() {
    let output = sf_compact()
        .args(["stats", "tests/fixtures", "--files"])
        .output()
        .expect("failed to run stats with --files");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Per file:"), "missing per-file header");
    assert!(
        stdout.contains("Admin.profile-meta.xml"),
        "missing profile in per-file list"
    );
}

#[test]
fn pack_specific_file() {
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            "tests/fixtures/force-app/main/default/profiles/Admin.profile-meta.xml",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run pack on single file");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 1 files"),
        "should pack exactly 1 file"
    );
}

#[test]
fn manifest_outputs_json() {
    let output = sf_compact()
        .args(["manifest"])
        .output()
        .expect("failed to run manifest");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output is not valid JSON");
    assert!(parsed.get("version").is_some(), "missing version");
    assert!(
        parsed.get("supported_metadata").is_some(),
        "missing supported_metadata"
    );
}

#[test]
fn empty_directory_produces_zero_stats() {
    let empty = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args(["stats", empty.path().to_str().unwrap()])
        .output()
        .expect("failed to run stats on empty dir");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 files"), "should report 0 files");
}

#[test]
fn nonexistent_path_fails() {
    let output = sf_compact()
        .args(["pack", "/nonexistent/path/that/does/not/exist"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "should fail on nonexistent path");
}

#[test]
fn numeric_strings_preserved_in_roundtrip() {
    // Create a minimal XML with numeric-like strings (leading zeros, decimals)
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ApexClass xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>59.0</apiVersion>
    <status>0012</status>
</ApexClass>"#;

    let dir = tempfile::tempdir().unwrap();
    let xml_path = dir.path().join("Test.cls-meta.xml");
    std::fs::write(&xml_path, xml).unwrap();

    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack
    let output = sf_compact()
        .args([
            "pack",
            xml_path.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to pack");
    assert!(output.status.success(), "pack failed");

    // Unpack
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack");
    assert!(output.status.success(), "unpack failed");

    // Find the restored XML and check values
    let restored = unpacked.path().join("Test.cls-meta.xml");
    assert!(restored.exists(), "restored file not found");
    let content = std::fs::read_to_string(&restored).unwrap();
    assert!(
        content.contains(">59.0<"),
        "apiVersion 59.0 was corrupted: {content}"
    );
    assert!(
        content.contains(">0012<"),
        "leading-zero string 0012 was corrupted: {content}"
    );
}

#[test]
fn unpack_ignores_non_sf_compact_yaml() {
    let dir = tempfile::tempdir().unwrap();

    // Create a random YAML file that is NOT sf-compact format
    std::fs::write(dir.path().join("random.yaml"), "key: value\n").unwrap();
    // Also create a proper sf-compact YAML
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(
        dir.path().join("sub/Test.cls-meta.yaml"),
        "_tag: ApexClass\n_ns: http://soap.sforce.com/2006/04/metadata\napiVersion: '59.0'\nstatus: Active\n",
    )
    .unwrap();

    let unpacked = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run unpack");
    assert!(
        output.status.success(),
        "unpack should not crash on dir with mixed YAML: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 1 files"),
        "should only unpack sf-compact YAML, got: {stdout}"
    );
}

#[test]
fn pack_does_not_show_token_count() {
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            "tests/fixtures",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run pack");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("tokens saved"),
        "pack should not print token count: {stdout}"
    );
}

#[test]
fn pack_with_absolute_paths() {
    let fixtures = std::fs::canonicalize("tests/fixtures").unwrap();
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run pack with absolute path");
    assert!(
        output.status.success(),
        "pack with absolute path failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify files ended up inside the output directory, not alongside sources
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 5 files"),
        "unexpected output: {stdout}"
    );

    // Check that YAML files are inside packed dir
    let yaml_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.yaml");
    assert!(
        yaml_profile.exists(),
        "YAML should be inside output dir, not alongside source"
    );
}
