//! Corruption tests: verify sf-compact handles broken/invalid input gracefully.
//! These test intentionally malformed packed files, invalid XML, truncated content,
//! and other edge cases that should produce clear errors instead of crashes.

use std::process::Command;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

// ─── Invalid packed files (unpack should skip gracefully) ───────

#[test]
fn corrupt_yaml_missing_tag() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // YAML without _tag field
    std::fs::write(
        dir.path().join("Test.cls-meta.yaml"),
        "apiVersion: '59.0'\nstatus: Active\n",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on missing _tag: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: skipping"),
        "should warn about skipped file: {stderr}"
    );
}

#[test]
fn corrupt_yaml_invalid_syntax() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Invalid YAML syntax
    std::fs::write(
        dir.path().join("Test.cls-meta.yaml"),
        "{{{{ not valid yaml at all ]]]]",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on invalid YAML: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_yaml_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join("Test.cls-meta.yaml"), "").unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on empty YAML: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_json_invalid_syntax() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join("Test.cls-meta.json"), "{not valid json").unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on invalid JSON: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_json_missing_tag() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    std::fs::write(
        dir.path().join("Test.cls-meta.json"),
        r#"{"apiVersion":"59.0","status":"Active"}"#,
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on JSON missing _tag: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_json_empty_object() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join("Test.cls-meta.json"), "{}").unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on empty JSON object: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ─── Invalid XML (pack should skip gracefully) ─────────────────

#[test]
fn corrupt_xml_hidden_content() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Managed package "(hidden)" file
    std::fs::write(dir.path().join("Test.cls-meta.xml"), "(hidden)").unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on (hidden) XML: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: skipping"),
        "should warn about invalid XML: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 0 files"),
        "should pack 0 valid files: {stdout}"
    );
}

#[test]
fn corrupt_xml_truncated() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Truncated XML
    std::fs::write(
        dir.path().join("Test.cls-meta.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ApexClass xmlns=\"http://soap",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on truncated XML: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_xml_unclosed_tags() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    std::fs::write(
        dir.path().join("Test.cls-meta.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ApexClass xmlns=\"http://soap.sforce.com/2006/04/metadata\">\n    <apiVersion>59.0</apiVersion>\n",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on unclosed tags: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_xml_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join("Test.cls-meta.xml"), "").unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on empty XML: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn corrupt_xml_binary_content() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Binary garbage
    std::fs::write(
        dir.path().join("Test.cls-meta.xml"),
        b"\x00\x01\x02\xff\xfe",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should not crash on binary content: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ─── Mixed valid and corrupt files ─────────────────────────────

#[test]
fn corrupt_mixed_valid_and_invalid_pack() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Valid file
    std::fs::write(
        dir.path().join("Good.cls-meta.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ApexClass xmlns=\"http://soap.sforce.com/2006/04/metadata\">\n    <apiVersion>59.0</apiVersion>\n    <status>Active</status>\n</ApexClass>",
    )
    .unwrap();

    // Hidden file
    std::fs::write(dir.path().join("Hidden.cls-meta.xml"), "(hidden)").unwrap();

    // Truncated file
    std::fs::write(
        dir.path().join("Bad.cls-meta.xml"),
        "<?xml version=\"1.0\"?>\n<broken",
    )
    .unwrap();

    let output = sf_compact()
        .args([
            "pack",
            dir.path().to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should succeed with mixed files: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 1 files"),
        "should pack only valid file: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning"),
        "should have warnings for invalid files: {stderr}"
    );
}

#[test]
fn corrupt_mixed_valid_and_invalid_unpack() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Valid sf-compact YAML
    std::fs::write(
        dir.path().join("Good.cls-meta.yaml"),
        "_tag: ApexClass\n_ns: http://soap.sforce.com/2006/04/metadata\napiVersion: '59.0'\nstatus: Active\n",
    )
    .unwrap();

    // Invalid YAML
    std::fs::write(
        dir.path().join("Bad.cls-meta.yaml"),
        "not: valid: sf: compact\n",
    )
    .unwrap();

    // Valid JSON
    std::fs::write(
        dir.path().join("Also.field-meta.json"),
        r#"{"_tag":"CustomField","_ns":"http://soap.sforce.com/2006/04/metadata","_children":[{"_tag":"fullName","_text":"Test"}]}"#,
    )
    .unwrap();

    // Invalid JSON
    std::fs::write(dir.path().join("Broken.field-meta.json"), "{broken json}").unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "should succeed with mixed valid/invalid: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 2 files"),
        "should unpack 2 valid files: {stdout}"
    );
}

// ─── Stats on corrupt files ────────────────────────────────────

#[test]
fn corrupt_stats_skips_invalid() {
    let dir = tempfile::tempdir().unwrap();

    std::fs::write(
        dir.path().join("Good.cls-meta.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ApexClass xmlns=\"http://soap.sforce.com/2006/04/metadata\">\n    <apiVersion>59.0</apiVersion>\n</ApexClass>",
    )
    .unwrap();

    std::fs::write(dir.path().join("Bad.cls-meta.xml"), "(hidden)").unwrap();

    let output = sf_compact()
        .args(["stats", dir.path().to_str().unwrap()])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stats should not crash: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 files"),
        "should count only valid file: {stdout}"
    );
}
