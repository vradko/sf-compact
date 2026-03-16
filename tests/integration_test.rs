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

    // Verify JSON files were created (json is default format)
    let json_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.json");
    assert!(json_profile.exists(), "JSON profile not created");

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

    // Check that JSON files are inside packed dir (json is default format)
    let json_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.json");
    assert!(
        json_profile.exists(),
        "JSON should be inside output dir, not alongside source"
    );
}

// ─── Config tests ───────────────────────────────────────────────

#[test]
fn config_init_creates_file() {
    let dir = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config init");
    assert!(
        output.status.success(),
        "config init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = dir.path().join(".sfcompact.yaml");
    assert!(config_path.exists(), ".sfcompact.yaml should be created");

    let content = std::fs::read_to_string(&config_path).unwrap();
    // Should have smart defaults: JSON default, order-insensitive types get yaml
    assert!(
        content.contains("default_format: json"),
        "config should have json as default_format, got: {content}"
    );
    // Profile is order-insensitive, should be yaml for readability
    assert!(
        content.contains("Profile: yaml"),
        "Profile should be set to yaml in smart defaults, got: {content}"
    );
}

#[test]
fn config_set_single_type() {
    let dir = tempfile::tempdir().unwrap();

    // First init config
    let output = sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config init");
    assert!(output.status.success());

    // Set a single type
    let output = sf_compact()
        .args(["config", "set", "flow", "json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config set");
    assert!(
        output.status.success(),
        "config set failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(dir.path().join(".sfcompact.yaml")).unwrap();
    assert!(
        content.contains("flow: json"),
        "config should contain flow: json, got: {content}"
    );
}

#[test]
fn config_set_batch() {
    let dir = tempfile::tempdir().unwrap();

    // Init config first
    sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Set multiple types in one call
    let output = sf_compact()
        .args(["config", "set", "flow", "json", "profile", "yaml"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config set batch");
    assert!(
        output.status.success(),
        "config set batch failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(dir.path().join(".sfcompact.yaml")).unwrap();
    assert!(
        content.contains("flow: json"),
        "config should contain flow: json, got: {content}"
    );
    assert!(
        content.contains("profile: yaml"),
        "config should contain profile: yaml, got: {content}"
    );
}

#[test]
fn config_set_default() {
    let dir = tempfile::tempdir().unwrap();

    // Init config first
    sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Change the default format
    let output = sf_compact()
        .args(["config", "set", "default", "json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config set default");
    assert!(
        output.status.success(),
        "config set default failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(dir.path().join(".sfcompact.yaml")).unwrap();
    assert!(
        content.contains("default_format: json"),
        "config should have default_format: json, got: {content}"
    );
}

#[test]
fn config_skip_type() {
    let dir = tempfile::tempdir().unwrap();

    // Init config first
    sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add to skip list
    let output = sf_compact()
        .args(["config", "skip", "customMetadata"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config skip");
    assert!(
        output.status.success(),
        "config skip failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(dir.path().join(".sfcompact.yaml")).unwrap();
    assert!(
        content.contains("customMetadata"),
        "config should contain customMetadata in skip list, got: {content}"
    );
}

#[test]
fn config_show_displays_config() {
    let dir = tempfile::tempdir().unwrap();

    // Init config first
    sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = sf_compact()
        .args(["config", "show"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config show");
    assert!(
        output.status.success(),
        "config show failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("default_format"),
        "config show should display default_format, got: {stdout}"
    );
}

#[test]
fn pack_respects_skip_config() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Create a config that skips "flow" type
    let config_content = "default_format: yaml\nformats: {}\nskip:\n- flow\n";
    std::fs::write(dir.path().join(".sfcompact.yaml"), config_content).unwrap();

    // Copy test fixtures into the temp dir
    let fixtures = std::path::Path::new("tests/fixtures");
    copy_dir_recursive(fixtures, &dir.path().join("tests/fixtures"));

    let output = sf_compact()
        .args([
            "pack",
            dir.path().join("tests/fixtures").to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run pack with skip config");
    assert!(
        output.status.success(),
        "pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Without skip, we pack 5 files. With flow skipped, we should pack 4.
    assert!(
        stdout.contains("Packed 4 files"),
        "should pack 4 files (skipping flow), got: {stdout}"
    );
}

#[test]
fn manifest_includes_order_sensitive() {
    let output = sf_compact()
        .args(["manifest"])
        .output()
        .expect("failed to run manifest");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("manifest output is not valid JSON");

    let metadata = parsed["supported_metadata"].as_array().unwrap();

    // Check that Flow is order_sensitive: true
    let flow_entry = metadata
        .iter()
        .find(|e| e["type"] == "Flow")
        .expect("Flow entry not found in manifest");
    assert_eq!(
        flow_entry["order_sensitive"], true,
        "Flow should be order_sensitive"
    );

    // Check that Profile is order_sensitive: false
    let profile_entry = metadata
        .iter()
        .find(|e| e["type"] == "Profile")
        .expect("Profile entry not found in manifest");
    assert_eq!(
        profile_entry["order_sensitive"], false,
        "Profile should not be order_sensitive"
    );

    // Check that supported_formats is present
    let formats = flow_entry["supported_formats"].as_array().unwrap();
    assert!(
        formats.contains(&serde_json::json!("yaml")),
        "supported_formats should include yaml"
    );
    assert!(
        formats.contains(&serde_json::json!("yaml-ordered")),
        "supported_formats should include yaml-ordered"
    );
    assert!(
        formats.contains(&serde_json::json!("json")),
        "supported_formats should include json"
    );
}

// ─── JSON format tests ──────────────────────────────────────────

#[test]
fn pack_json_format() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run pack with --format json");
    assert!(
        output.status.success(),
        "pack --format json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 5 files"),
        "unexpected pack output: {stdout}"
    );

    // Verify .json files were created (not .yaml)
    let json_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.json");
    assert!(
        json_profile.exists(),
        "JSON profile file not created at {}",
        json_profile.display()
    );

    // Verify it's valid JSON
    let content = std::fs::read_to_string(&json_profile).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Output is not valid JSON");
    assert_eq!(
        parsed.get("_tag").and_then(|v| v.as_str()),
        Some("Profile"),
        "JSON should have _tag: Profile"
    );
}

#[test]
fn json_roundtrip() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack as JSON
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to pack as JSON");
    assert!(
        output.status.success(),
        "pack json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Unpack back to XML
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack JSON");
    assert!(
        output.status.success(),
        "unpack JSON failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 5 files"),
        "unexpected unpack output: {stdout}"
    );

    // Verify restored XML
    let xml_profile = unpacked
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.xml");
    assert!(xml_profile.exists(), "XML profile not restored from JSON");

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
fn json_preserves_element_order() {
    // The flow fixture has two <assignments> elements in a specific order.
    // JSON format should preserve that order via _children arrays.
    let flow_xml = std::fs::read_to_string(
        "tests/fixtures/force-app/main/default/flows/Case_Assignment.flow-meta.xml",
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let xml_path = dir.path().join("Case_Assignment.flow-meta.xml");
    std::fs::write(&xml_path, &flow_xml).unwrap();

    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack as JSON
    let output = sf_compact()
        .args([
            "pack",
            xml_path.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to pack flow as JSON");
    assert!(output.status.success(), "pack json failed");

    // Verify JSON has _children array preserving order
    let json_path = packed.path().join("Case_Assignment.flow-meta.json");
    assert!(json_path.exists(), "JSON flow file not created");
    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();

    // _children should be an array
    let children = parsed
        .get("_children")
        .and_then(|v| v.as_array())
        .expect("JSON should have _children array");

    // Find the assignments in order — High_Priority_Assignment should come before Low_Priority_Assignment
    let assignment_names: Vec<&str> = children
        .iter()
        .filter(|c| c.get("_tag").and_then(|t| t.as_str()) == Some("assignments"))
        .filter_map(|c| {
            c.get("_children")
                .and_then(|ch| ch.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|item| {
                        if item.get("_tag").and_then(|t| t.as_str()) == Some("name") {
                            item.get("_text").and_then(|v| v.as_str())
                        } else {
                            None
                        }
                    })
                })
        })
        .collect();

    assert_eq!(
        assignment_names,
        vec!["High_Priority_Assignment", "Low_Priority_Assignment"],
        "JSON should preserve element order"
    );

    // Roundtrip back to XML and verify order
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack");
    assert!(output.status.success());

    let restored = unpacked.path().join("Case_Assignment.flow-meta.xml");
    let restored_content = std::fs::read_to_string(&restored).unwrap();

    // Verify that the <assignments> blocks appear in the correct order.
    // We look for <name>High_Priority_Assignment</name> and <name>Low_Priority_Assignment</name>
    // within the assignments context (not the defaultConnector references).
    let high_assignment_pos = restored_content
        .find("<name>High_Priority_Assignment</name>")
        .expect("High_Priority_Assignment assignment not found");
    let low_assignment_pos = restored_content
        .find("<name>Low_Priority_Assignment</name>")
        .expect("Low_Priority_Assignment assignment not found");
    assert!(
        high_assignment_pos < low_assignment_pos,
        "Element order not preserved in JSON roundtrip: High at {}, Low at {}",
        high_assignment_pos,
        low_assignment_pos
    );
}

#[test]
fn pack_uses_config_format() {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();

    // Create config that sets flow to json (everything else yaml)
    let config_content = "default_format: yaml\nformats:\n  Flow: json\nskip: []\n";
    std::fs::write(dir.path().join(".sfcompact.yaml"), config_content).unwrap();

    // Copy test fixtures into the temp dir
    let fixtures = std::path::Path::new("tests/fixtures");
    copy_dir_recursive(fixtures, &dir.path().join("tests/fixtures"));

    let output = sf_compact()
        .args([
            "pack",
            dir.path().join("tests/fixtures").to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run pack with config format");
    assert!(
        output.status.success(),
        "pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Flow should be .json
    let flow_json = packed
        .path()
        .join("force-app/main/default/flows/Case_Assignment.flow-meta.json");
    assert!(
        flow_json.exists(),
        "Flow should be packed as .json per config"
    );

    // Profile should still be .yaml
    let profile_yaml = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.yaml");
    assert!(
        profile_yaml.exists(),
        "Profile should still be .yaml per default config"
    );
}

#[test]
fn numeric_strings_preserved_json_roundtrip() {
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

    // Pack as JSON
    let output = sf_compact()
        .args([
            "pack",
            xml_path.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to pack as JSON");
    assert!(output.status.success(), "pack json failed");

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

    let restored = unpacked.path().join("Test.cls-meta.xml");
    assert!(restored.exists(), "restored file not found");
    let content = std::fs::read_to_string(&restored).unwrap();
    assert!(
        content.contains(">59.0<"),
        "apiVersion 59.0 was corrupted in JSON roundtrip: {content}"
    );
    assert!(
        content.contains(">0012<"),
        "leading-zero string 0012 was corrupted in JSON roundtrip: {content}"
    );
}

// ─── YAML Ordered format tests ──────────────────────────────────

#[test]
fn pack_yaml_ordered_format() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "yaml-ordered",
        ])
        .output()
        .expect("failed to run pack with --format yaml-ordered");
    assert!(
        output.status.success(),
        "pack --format yaml-ordered failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Packed 5 files"),
        "unexpected pack output: {stdout}"
    );

    // Verify .yaml files were created (yaml-ordered still uses .yaml extension)
    let yaml_profile = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.yaml");
    assert!(
        yaml_profile.exists(),
        "YAML profile file not created for yaml-ordered format"
    );

    // Verify it contains _children key (ordered format)
    let flow_yaml = packed
        .path()
        .join("force-app/main/default/flows/Case_Assignment.flow-meta.yaml");
    assert!(flow_yaml.exists(), "YAML flow file not created");
    let content = std::fs::read_to_string(&flow_yaml).unwrap();
    assert!(
        content.contains("_children:"),
        "yaml-ordered output should contain _children key, got: {content}"
    );
    assert!(
        content.contains("_tag: Flow"),
        "yaml-ordered output should contain _tag, got: {content}"
    );
}

#[test]
fn yaml_ordered_roundtrip() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack as yaml-ordered
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "yaml-ordered",
        ])
        .output()
        .expect("failed to pack as yaml-ordered");
    assert!(
        output.status.success(),
        "pack yaml-ordered failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Unpack back to XML
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack yaml-ordered");
    assert!(
        output.status.success(),
        "unpack yaml-ordered failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 5 files"),
        "unexpected unpack output: {stdout}"
    );

    // Verify restored XML
    let xml_profile = unpacked
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.xml");
    assert!(
        xml_profile.exists(),
        "XML profile not restored from yaml-ordered"
    );

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
fn yaml_ordered_preserves_element_order() {
    // The flow fixture has two <assignments> elements in a specific order.
    // yaml-ordered format should preserve that order via _children arrays.
    let flow_xml = std::fs::read_to_string(
        "tests/fixtures/force-app/main/default/flows/Case_Assignment.flow-meta.xml",
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let xml_path = dir.path().join("Case_Assignment.flow-meta.xml");
    std::fs::write(&xml_path, &flow_xml).unwrap();

    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack as yaml-ordered
    let output = sf_compact()
        .args([
            "pack",
            xml_path.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "yaml-ordered",
        ])
        .output()
        .expect("failed to pack flow as yaml-ordered");
    assert!(output.status.success(), "pack yaml-ordered failed");

    // Verify YAML has _children
    let yaml_path = packed.path().join("Case_Assignment.flow-meta.yaml");
    assert!(yaml_path.exists(), "YAML flow file not created");
    let yaml_content = std::fs::read_to_string(&yaml_path).unwrap();
    assert!(
        yaml_content.contains("_children:"),
        "yaml-ordered should have _children"
    );

    // Roundtrip back to XML and verify order
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack");
    assert!(output.status.success());

    let restored = unpacked.path().join("Case_Assignment.flow-meta.xml");
    let restored_content = std::fs::read_to_string(&restored).unwrap();

    let high_assignment_pos = restored_content
        .find("<name>High_Priority_Assignment</name>")
        .expect("High_Priority_Assignment not found");
    let low_assignment_pos = restored_content
        .find("<name>Low_Priority_Assignment</name>")
        .expect("Low_Priority_Assignment not found");
    assert!(
        high_assignment_pos < low_assignment_pos,
        "Element order not preserved in yaml-ordered roundtrip: High at {}, Low at {}",
        high_assignment_pos,
        low_assignment_pos
    );
}

#[test]
fn config_init_uses_yaml_ordered_for_order_sensitive() {
    let dir = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run config init");
    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join(".sfcompact.yaml")).unwrap();

    // Order-insensitive types should use yaml for readability; order-sensitive use json default
    assert!(
        content.contains("Profile: yaml"),
        "Profile should be yaml (order-insensitive), got: {content}"
    );
    assert!(
        content.contains("PermissionSet: yaml"),
        "PermissionSet should be yaml (order-insensitive), got: {content}"
    );
    // Order-sensitive types like Flow should NOT have an override (they use json default)
    assert!(
        !content.contains("Flow:"),
        "Flow should not have an override (uses json default), got: {content}"
    );
}

#[test]
fn unpack_handles_both_yaml_formats() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Create a grouped YAML file (standard format)
    let grouped_yaml = "_tag: ApexClass\n_ns: http://soap.sforce.com/2006/04/metadata\napiVersion: '59.0'\nstatus: Active\n";
    std::fs::write(dir.path().join("Grouped.cls-meta.yaml"), grouped_yaml).unwrap();

    // Create an ordered YAML file (yaml-ordered format with _children)
    let ordered_yaml = r#"_tag: ApexClass
_ns: http://soap.sforce.com/2006/04/metadata
_children:
- apiVersion: '59.0'
- status: Active
"#;
    std::fs::write(dir.path().join("Ordered.cls-meta.yaml"), ordered_yaml).unwrap();

    let output = sf_compact()
        .args([
            "unpack",
            dir.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to unpack");
    assert!(
        output.status.success(),
        "unpack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 2 files"),
        "should unpack 2 files, got: {stdout}"
    );

    // Verify both were restored correctly
    let grouped_xml = unpacked.path().join("Grouped.cls-meta.xml");
    assert!(grouped_xml.exists(), "Grouped XML not restored");
    let content = std::fs::read_to_string(&grouped_xml).unwrap();
    assert!(content.contains("<ApexClass"), "grouped XML missing tag");
    assert!(content.contains(">59.0<"), "grouped XML missing apiVersion");

    let ordered_xml = unpacked.path().join("Ordered.cls-meta.xml");
    assert!(ordered_xml.exists(), "Ordered XML not restored");
    let content = std::fs::read_to_string(&ordered_xml).unwrap();
    assert!(content.contains("<ApexClass"), "ordered XML missing tag");
    assert!(content.contains(">59.0<"), "ordered XML missing apiVersion");
}

// ─── Bug fix tests ──────────────────────────────────────────────

#[test]
fn unpack_skips_invalid_sf_compact_files() {
    let dir = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Create a file that matches -meta.yaml naming but has no _tag (not sf-compact)
    std::fs::write(
        dir.path().join("SomeObject.object-meta.yaml"),
        "description: just a random YAML\nfields:\n- name: foo\n",
    )
    .unwrap();

    // Also create a valid sf-compact file
    std::fs::write(
        dir.path().join("Test.cls-meta.yaml"),
        "_tag: ApexClass\n_ns: http://soap.sforce.com/2006/04/metadata\napiVersion: '59.0'\nstatus: Active\n",
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
        .expect("failed to run unpack");

    // Should succeed (not crash) even with invalid file present
    assert!(
        output.status.success(),
        "unpack should not crash on invalid sf-compact files: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 1 files"),
        "should only unpack valid sf-compact file, got: {stdout}"
    );

    // Should have a warning on stderr about the skipped file
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Warning: skipping"),
        "should warn about skipped file, stderr: {stderr}"
    );
}

// ─── Diff tests ─────────────────────────────────────────────────

#[test]
fn diff_shows_no_changes_after_pack() {
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
        .expect("failed to pack");
    assert!(output.status.success());

    // Diff should show no changes
    let output = sf_compact()
        .args([
            "diff",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to diff");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No changes"),
        "should show no changes after fresh pack, got: {stdout}"
    );
}

#[test]
fn diff_detects_new_files() {
    let packed = tempfile::tempdir().unwrap();

    // Don't pack anything — all files should be "new"
    let output = sf_compact()
        .args([
            "diff",
            "tests/fixtures",
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to diff");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("new"),
        "should show new files when nothing packed, got: {stdout}"
    );
    assert!(
        stdout.contains("Run `sf-compact pack`"),
        "should suggest running pack, got: {stdout}"
    );
}

#[test]
fn diff_detects_modified_files() {
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
        .expect("failed to pack");
    assert!(output.status.success());

    // Corrupt a packed file to simulate stale pack
    let profile_json = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.json");
    assert!(profile_json.exists());
    std::fs::write(&profile_json, "# corrupted\n").unwrap();

    // Diff should detect the modification
    let output = sf_compact()
        .args([
            "diff",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to diff");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("modified"),
        "should detect modified file, got: {stdout}"
    );
}

// ─── Unpack --include test ──────────────────────────────────────

#[test]
fn unpack_with_include_filter() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack everything first
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to pack");
    assert!(output.status.success());

    // Unpack only profiles
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
            "--include",
            "*.profile-meta.json",
        ])
        .output()
        .expect("failed to unpack with --include");
    assert!(
        output.status.success(),
        "unpack with --include failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unpacked 1 files"),
        "should unpack only 1 profile, got: {stdout}"
    );
}

#[test]
fn invalid_format_rejected() {
    let output = sf_compact()
        .args(["pack", "tests/fixtures", "--format", "foobar"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "should reject invalid format");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid format") && stderr.contains("foobar"),
        "should show error about invalid format: {stderr}"
    );
}

// ─── Lint tests ────────────────────────────────────────────────

#[test]
fn lint_passes_when_up_to_date() {
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
        .expect("failed to pack");
    assert!(output.status.success());

    // Lint should pass (exit 0)
    let output = sf_compact()
        .args([
            "lint",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to lint");
    assert!(
        output.status.success(),
        "lint should pass when up to date, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK"), "should say OK, got: {stdout}");
}

#[test]
fn lint_fails_when_stale() {
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
        .expect("failed to pack");
    assert!(output.status.success());

    // Corrupt a packed file
    let profile_json = packed
        .path()
        .join("force-app/main/default/profiles/Admin.profile-meta.json");
    assert!(profile_json.exists());
    std::fs::write(&profile_json, "# corrupted\n").unwrap();

    // Lint should fail (exit 1)
    let output = sf_compact()
        .args([
            "lint",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to lint");
    assert!(
        !output.status.success(),
        "lint should fail when files are stale"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stale"),
        "should report stale files, got: {stderr}"
    );
}

#[test]
fn lint_fails_when_not_packed() {
    let fixtures = Path::new("tests/fixtures");
    let empty_packed = tempfile::tempdir().unwrap();

    // Lint against empty packed dir — all files are "new"
    let output = sf_compact()
        .args([
            "lint",
            fixtures.to_str().unwrap(),
            "-o",
            empty_packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to lint");
    assert!(
        !output.status.success(),
        "lint should fail when nothing is packed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not packed"),
        "should report unpacked files, got: {stderr}"
    );
}

// ─── Config validation tests ───────────────────────────────────

#[test]
fn config_set_rejects_invalid_format() {
    let tmp = tempfile::tempdir().unwrap();
    // Create a minimal config
    std::fs::write(tmp.path().join(".sfcompact.yaml"), "default_format: yaml\n").unwrap();

    let output = sf_compact()
        .args(["config", "set", "default", "banana"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(
        !output.status.success(),
        "should reject invalid format 'banana'"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid format") && stderr.contains("banana"),
        "should show error about invalid format: {stderr}"
    );
}

// ─── Stats respects config format ──────────────────────────────

#[test]
fn stats_respects_config_format() {
    let fixtures = Path::new("tests/fixtures");
    let tmp = tempfile::tempdir().unwrap();

    // Copy fixtures into tmp so we can put a config next to them
    copy_dir_recursive(fixtures, &tmp.path().join("force-app"));

    // Create config with json format
    std::fs::write(tmp.path().join(".sfcompact.yaml"), "default_format: json\n").unwrap();

    // Run stats from the tmp dir (where config lives)
    let output = sf_compact()
        .args(["stats", "force-app"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run stats");
    assert!(
        output.status.success(),
        "stats should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Also pack with json to compare
    let packed = tempfile::tempdir().unwrap();
    let pack_output = sf_compact()
        .args([
            "pack",
            "force-app",
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("failed to pack");
    assert!(pack_output.status.success());

    // Verify stats shows correct labels and data
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compact (after)"),
        "stats should show 'compact (after)' column header: {stdout}"
    );
    assert!(
        stdout.contains("Bytes") && stdout.contains("Tokens"),
        "stats should show Bytes and Tokens rows: {stdout}"
    );
    assert!(
        !stdout.contains("YAML bytes"),
        "stats should not say 'YAML bytes': {stdout}"
    );
}

// ─── Chaos test fixes ──────────────────────────────────────────

#[test]
fn pack_format_switch_cleans_stale_files() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack as yaml first
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "yaml",
        ])
        .output()
        .expect("failed to pack yaml");
    assert!(output.status.success());

    // Verify .yaml files exist
    let yaml_count = walkdir::WalkDir::new(packed.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
        .count();
    assert!(yaml_count > 0, "should have yaml files");

    // Re-pack as json
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to pack json");
    assert!(output.status.success());

    // Old .yaml files should be cleaned up
    let yaml_count = walkdir::WalkDir::new(packed.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
        .count();
    assert_eq!(yaml_count, 0, "stale yaml files should be removed");

    // .json files should exist
    let json_count = walkdir::WalkDir::new(packed.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert!(json_count > 0, "should have json files");
}

#[test]
fn diff_single_file_no_false_deleted() {
    let fixtures = Path::new("tests/fixtures");
    let packed = tempfile::tempdir().unwrap();

    // Pack all files
    let output = sf_compact()
        .args([
            "pack",
            fixtures.to_str().unwrap(),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to pack");
    assert!(output.status.success());

    // Diff with a single file source — should NOT falsely report other
    // packed files as "deleted"
    let output = sf_compact()
        .args([
            "diff",
            &format!(
                "{}/force-app/main/default/profiles/Admin.profile-meta.xml",
                fixtures.display()
            ),
            "-o",
            packed.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to diff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not list any files as deleted (the "- file (packed file has no source XML)" lines)
    assert!(
        !stdout.contains("(packed file has no source"),
        "single-file diff should not report false deletions, got: {stdout}"
    );
    // The summary should show 0 deleted
    assert!(
        stdout.contains("0 deleted") || !stdout.contains("deleted"),
        "single-file diff should have 0 deleted, got: {stdout}"
    );
}

#[test]
fn config_set_rejects_empty_key() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".sfcompact.yaml"), "default_format: yaml\n").unwrap();

    let output = sf_compact()
        .args(["config", "set", "", "yaml"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "should reject empty key");
}

#[test]
fn config_skip_rejects_empty_type() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".sfcompact.yaml"), "default_format: yaml\n").unwrap();

    let output = sf_compact()
        .args(["config", "skip", ""])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "should reject empty type name");
}

#[test]
fn config_invalid_format_in_file_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    copy_dir_recursive(Path::new("tests/fixtures"), &tmp.path().join("force-app"));

    // Write config with invalid format
    std::fs::write(
        tmp.path().join(".sfcompact.yaml"),
        "default_format: banana\n",
    )
    .unwrap();

    let output = sf_compact()
        .args(["pack", "force-app"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(
        !output.status.success(),
        "should reject invalid format in config file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid") && stderr.contains("banana"),
        "should mention the invalid format: {stderr}"
    );
}

/// Helper to recursively copy a directory.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            std::fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}
