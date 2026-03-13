//! Entity and special character tests: verify XML entities (&quot;, &amp;, &lt;, &gt;)
//! are preserved correctly through pack/unpack roundtrip across all formats.

use std::process::Command;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

// ─── &quot; entity preservation ────────────────────────────────

#[test]
fn entity_quot_in_formula_yaml() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>INCLUDES(Business_Unit__c, &quot;D&amp;I&quot;)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &[],
        &["&quot;D&amp;I&quot;"],
    );
}

#[test]
fn entity_quot_in_formula_json() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>INCLUDES(Business_Unit__c, &quot;D&amp;I&quot;)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &["--format", "json"],
        &["&quot;D&amp;I&quot;"],
    );
}

#[test]
fn entity_quot_in_formula_ordered() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>INCLUDES(Business_Unit__c, &quot;D&amp;I&quot;)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &["--format", "yaml-ordered"],
        &["&quot;D&amp;I&quot;"],
    );
}

// ─── &amp; entity preservation ──────────────────────────────────

#[test]
fn entity_amp_in_description() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <description>Sales &amp; Marketing department</description>
</CustomField>"#,
        "Test.field-meta.xml",
        &[],
        &["Sales &amp; Marketing"],
    );
}

#[test]
fn entity_amp_in_label() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>R&amp;D Team</label>
</CustomField>"#,
        "Test.field-meta.xml",
        &[],
        &["R&amp;D Team"],
    );
}

// ─── &lt; and &gt; entity preservation ──────────────────────────

#[test]
fn entity_lt_gt_in_formula() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>IF(Amount &gt; 1000 &amp;&amp; Amount &lt; 5000, true, false)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &[],
        &["&gt; 1000", "&lt; 5000"],
    );
}

#[test]
fn entity_lt_gt_in_validation_message() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <errorMessage>Value must be &gt; 0 and &lt; 100</errorMessage>
</ValidationRule>"#,
        "Test.validationRule-meta.xml",
        &[],
        &["&gt; 0", "&lt; 100"],
    );
}

// ─── Multiple entities in one value ─────────────────────────────

#[test]
fn entity_multiple_in_one_value() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>IF(Name = &quot;A &amp; B&quot;, &quot;C &lt; D&quot;, &quot;E &gt; F&quot;)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &[],
        &[
            "&quot;A &amp; B&quot;",
            "&quot;C &lt; D&quot;",
            "&quot;E &gt; F&quot;",
        ],
    );
}

#[test]
fn entity_multiple_json_roundtrip() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <formula>IF(Name = &quot;A &amp; B&quot;, &quot;C &lt; D&quot;, &quot;E &gt; F&quot;)</formula>
</CustomField>"#,
        "Test.field-meta.xml",
        &["--format", "json"],
        &[
            "&quot;A &amp; B&quot;",
            "&quot;C &lt; D&quot;",
            "&quot;E &gt; F&quot;",
        ],
    );
}

// ─── Self-closing elements ──────────────────────────────────────

#[test]
fn self_closing_elements_roundtrip() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<SharingRules xmlns="http://soap.sforce.com/2006/04/metadata"/>"#,
        "Test.sharingRules-meta.xml",
        &[],
        &["<SharingRules"],
    );
}

#[test]
fn self_closing_in_children() {
    roundtrip_entity_test(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>Test</label>
    <enableActivities/>
    <enableHistory/>
</CustomObject>"#,
        "Test.object-meta.xml",
        &[],
        &["<label>Test</label>"],
    );
}

// ─── Numeric string preservation ────────────────────────────────

#[test]
fn numeric_leading_zeros_all_formats() {
    for format in &["yaml", "yaml-ordered", "json"] {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ApexClass xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>59.0</apiVersion>
    <status>0012</status>
</ApexClass>"#;

        let dir = tempfile::tempdir().unwrap();
        let packed = tempfile::tempdir().unwrap();
        let unpacked = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("Test.cls-meta.xml"), xml).unwrap();

        let output = sf_compact()
            .args([
                "pack",
                dir.path().to_str().unwrap(),
                "-o",
                packed.path().to_str().unwrap(),
                "--format",
                format,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "pack {format} failed");

        let output = sf_compact()
            .args([
                "unpack",
                packed.path().to_str().unwrap(),
                "-o",
                unpacked.path().to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "unpack {format} failed");

        let xml_path = unpacked.path().join("Test.cls-meta.xml");
        let content = std::fs::read_to_string(&xml_path).unwrap();
        assert!(
            content.contains(">59.0<"),
            "{format}: apiVersion 59.0 corrupted: {content}"
        );
        assert!(
            content.contains(">0012<"),
            "{format}: leading zeros 0012 corrupted: {content}"
        );
    }
}

// ─── Validation rule with complex entities ──────────────────────

#[test]
fn validation_rule_complex_entities() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ValidationRule xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Amount_Validation</fullName>
    <active>true</active>
    <errorConditionFormula>Amount &lt; 0 || Amount &gt; 999999</errorConditionFormula>
    <errorMessage>Amount must be between 0 &amp; 999,999. Use &quot;0&quot; for no amount.</errorMessage>
</ValidationRule>"#;

    for format in &["yaml", "yaml-ordered", "json"] {
        let dir = tempfile::tempdir().unwrap();
        let packed = tempfile::tempdir().unwrap();
        let unpacked = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("Test.validationRule-meta.xml"), xml).unwrap();

        let output = sf_compact()
            .args([
                "pack",
                dir.path().to_str().unwrap(),
                "-o",
                packed.path().to_str().unwrap(),
                "--format",
                format,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "pack {format} failed");

        let output = sf_compact()
            .args([
                "unpack",
                packed.path().to_str().unwrap(),
                "-o",
                unpacked.path().to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "unpack {format} failed");

        let restored_path = unpacked.path().join("Test.validationRule-meta.xml");
        let restored = std::fs::read_to_string(&restored_path).unwrap();

        assert!(
            restored.contains("&lt; 0"),
            "{format}: &lt; lost: {restored}"
        );
        assert!(
            restored.contains("&gt; 999999"),
            "{format}: &gt; lost: {restored}"
        );
        assert!(
            restored.contains("0 &amp; 999"),
            "{format}: &amp; lost: {restored}"
        );
        assert!(
            restored.contains("&quot;0&quot;"),
            "{format}: &quot; lost: {restored}"
        );
    }
}

// ─── Helper ─────────────────────────────────────────────────────

fn roundtrip_entity_test(
    xml: &str,
    filename: &str,
    extra_args: &[&str],
    expected_fragments: &[&str],
) {
    let dir = tempfile::tempdir().unwrap();
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join(filename), xml).unwrap();

    let mut cmd = sf_compact();
    cmd.args([
        "pack",
        dir.path().to_str().unwrap(),
        "-o",
        packed.path().to_str().unwrap(),
    ]);
    for arg in extra_args {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("pack failed");
    assert!(
        output.status.success(),
        "pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
            "-o",
            unpacked.path().to_str().unwrap(),
        ])
        .output()
        .expect("unpack failed");
    assert!(
        output.status.success(),
        "unpack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Find the restored XML
    let restored_path = unpacked.path().join(filename);
    assert!(
        restored_path.exists(),
        "Restored file not found: {}",
        restored_path.display()
    );

    let restored = std::fs::read_to_string(&restored_path).unwrap();

    for fragment in expected_fragments {
        assert!(
            restored.contains(fragment),
            "Expected fragment '{}' not found in restored XML:\n{}",
            fragment,
            restored
        );
    }
}
