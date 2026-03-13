//! Mutation tests: modify packed files, then unpack and verify the changes
//! are reflected in the output XML. This validates that AI tools can safely
//! edit the compact format and get correct XML back.

use std::process::Command;

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

// ─── YAML Mutation Tests ────────────────────────────────────────

#[test]
fn mutation_yaml_change_field_value() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Industry</fullName>
    <label>Industry</label>
    <type>Picklist</type>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &[]);

    // Mutate: change label
    let yaml_path = find_file(&packed_dir, "field-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content.replace("label: Industry", "label: Business Industry");
    std::fs::write(&yaml_path, &mutated).unwrap();

    // Unpack mutated
    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<label>Business Industry</label>"),
        "Mutation should be reflected in XML: {restored}"
    );
    assert!(
        !restored.contains("<label>Industry</label>"),
        "Old value should be gone: {restored}"
    );
}

#[test]
fn mutation_yaml_add_new_element() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Revenue</fullName>
    <label>Revenue</label>
    <type>Currency</type>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &[]);

    // Mutate: add description
    let yaml_path = find_file(&packed_dir, "field-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content.replace(
        "label: Revenue",
        "label: Revenue\ndescription: Annual revenue in USD",
    );
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<description>Annual revenue in USD</description>"),
        "Added element should appear in XML: {restored}"
    );
}

#[test]
fn mutation_yaml_delete_element() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Revenue</fullName>
    <label>Revenue</label>
    <description>Delete me</description>
    <type>Currency</type>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &[]);

    // Mutate: remove description line
    let yaml_path = find_file(&packed_dir, "field-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content
        .lines()
        .filter(|l| !l.contains("description:"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        !restored.contains("<description>"),
        "Deleted element should be gone: {restored}"
    );
    assert!(
        restored.contains("<label>Revenue</label>"),
        "Other elements should remain: {restored}"
    );
}

#[test]
fn mutation_yaml_change_boolean() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Active</fullName>
    <required>false</required>
    <trackFeedHistory>false</trackFeedHistory>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &[]);

    // Mutate: flip boolean
    let yaml_path = find_file(&packed_dir, "field-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content.replace("required: false", "required: true");
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<required>true</required>"),
        "Boolean mutation should work: {restored}"
    );
}

#[test]
fn mutation_yaml_add_array_element() {
    // Use two fieldPermissions so YAML generates array syntax with `- `
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Profile xmlns="http://soap.sforce.com/2006/04/metadata">
    <fieldPermissions>
        <editable>true</editable>
        <field>Account.Name</field>
        <readable>true</readable>
    </fieldPermissions>
    <fieldPermissions>
        <editable>false</editable>
        <field>Account.Email</field>
        <readable>true</readable>
    </fieldPermissions>
</Profile>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.profile-meta.xml", &[]);

    // Mutate: add a third fieldPermission to the array
    let yaml_path = find_file(&packed_dir, "profile-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = format!(
        "{}\n- editable: false\n  field: Account.Phone\n  readable: true\n",
        content.trim_end()
    );
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "profile-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<field>Account.Phone</field>"),
        "Added array element should appear: {restored}"
    );
    assert!(
        restored.contains("<field>Account.Name</field>"),
        "Original should remain: {restored}"
    );
}

#[test]
fn mutation_yaml_remove_array_element() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Profile xmlns="http://soap.sforce.com/2006/04/metadata">
    <fieldPermissions>
        <editable>true</editable>
        <field>Account.Name</field>
        <readable>true</readable>
    </fieldPermissions>
    <fieldPermissions>
        <editable>false</editable>
        <field>Account.Phone</field>
        <readable>true</readable>
    </fieldPermissions>
</Profile>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.profile-meta.xml", &[]);

    // Mutate: remove second fieldPermission
    let yaml_path = find_file(&packed_dir, "profile-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content.replace(
        "\n- editable: false\n  field: Account.Phone\n  readable: true",
        "",
    );
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "profile-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        !restored.contains("Account.Phone"),
        "Removed array element should be gone: {restored}"
    );
    assert!(
        restored.contains("Account.Name"),
        "Remaining element should stay: {restored}"
    );
}

// ─── JSON Mutation Tests ────────────────────────────────────────

#[test]
fn mutation_json_change_value() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Name</fullName>
    <label>Name</label>
    <type>Text</type>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &["--format", "json"]);

    // Mutate JSON: change the _text of the label child
    let json_path = find_file(&packed_dir, "field-meta.json");
    let content = std::fs::read_to_string(&json_path).unwrap();
    let mut parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    if let Some(children) = parsed.get_mut("_children").and_then(|c| c.as_array_mut()) {
        for child in children.iter_mut() {
            if child.get("_tag").and_then(|t| t.as_str()) == Some("label") {
                child["_text"] = serde_json::json!("Full Name");
            }
        }
    }

    std::fs::write(&json_path, serde_json::to_string(&parsed).unwrap()).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<label>Full Name</label>"),
        "JSON mutation should work: {restored}"
    );
}

#[test]
fn mutation_json_add_element() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CustomField xmlns="http://soap.sforce.com/2006/04/metadata">
    <fullName>Account.Name</fullName>
    <type>Text</type>
</CustomField>"#;

    let (packed_dir, unpacked_dir) = pack_xml(xml, "Test.field-meta.xml", &["--format", "json"]);

    // Mutate: add element via JSON
    let json_path = find_file(&packed_dir, "field-meta.json");
    let content = std::fs::read_to_string(&json_path).unwrap();
    let mut parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Add new child to _children array
    if let Some(children) = parsed.get_mut("_children").and_then(|c| c.as_array_mut()) {
        children.push(serde_json::json!({"_tag": "label", "_text": "Account Name"}));
    }

    std::fs::write(&json_path, serde_json::to_string(&parsed).unwrap()).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "field-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<label>Account Name</label>"),
        "Added JSON element should appear: {restored}"
    );
}

// ─── YAML-Ordered Mutation Tests ────────────────────────────────

#[test]
fn mutation_ordered_change_value() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>My Flow</label>
    <status>Active</status>
</Flow>"#;

    let (packed_dir, unpacked_dir) =
        pack_xml(xml, "Test.flow-meta.xml", &["--format", "yaml-ordered"]);

    let yaml_path = find_file(&packed_dir, "flow-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    let mutated = content.replace("status: Active", "status: Draft");
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "flow-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();
    assert!(
        restored.contains("<status>Draft</status>"),
        "Ordered YAML mutation should work: {restored}"
    );
}

#[test]
fn mutation_ordered_reorder_children() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Flow xmlns="http://soap.sforce.com/2006/04/metadata">
    <label>My Flow</label>
    <status>Active</status>
    <apiVersion>59.0</apiVersion>
</Flow>"#;

    let (packed_dir, unpacked_dir) =
        pack_xml(xml, "Test.flow-meta.xml", &["--format", "yaml-ordered"]);

    // Mutate: reorder children (move apiVersion before status)
    let yaml_path = find_file(&packed_dir, "flow-meta.yaml");
    let content = std::fs::read_to_string(&yaml_path).unwrap();
    assert!(content.contains("_children:"));

    // Replace _children section
    let mutated = content.replace(
        "_children:\n- label: My Flow\n- status: Active\n- apiVersion: '59.0'",
        "_children:\n- label: My Flow\n- apiVersion: '59.0'\n- status: Active",
    );
    std::fs::write(&yaml_path, &mutated).unwrap();

    unpack_to(&packed_dir, &unpacked_dir);

    let xml_path = find_file(&unpacked_dir, "flow-meta.xml");
    let restored = std::fs::read_to_string(&xml_path).unwrap();

    // apiVersion should now come before status
    let api_pos = restored.find("<apiVersion>").unwrap();
    let status_pos = restored.find("<status>").unwrap();
    assert!(
        api_pos < status_pos,
        "Reordered children should be preserved: api at {api_pos}, status at {status_pos}"
    );
}

// ─── Helpers ────────────────────────────────────────────────────

fn pack_xml(
    xml: &str,
    filename: &str,
    extra_args: &[&str],
) -> (tempfile::TempDir, tempfile::TempDir) {
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
    let output = cmd.output().expect("failed to pack");
    assert!(
        output.status.success(),
        "pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Keep dir alive by leaking it (it'll be cleaned up with packed/unpacked)
    std::mem::forget(dir);

    (packed, unpacked)
}

fn unpack_to(packed: &tempfile::TempDir, unpacked: &tempfile::TempDir) {
    let output = sf_compact()
        .args([
            "unpack",
            packed.path().to_str().unwrap(),
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
}

fn find_file(dir: &tempfile::TempDir, suffix: &str) -> std::path::PathBuf {
    for entry in walkdir::WalkDir::new(dir.path()) {
        let entry = entry.unwrap();
        if entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.ends_with(suffix))
        {
            return entry.path().to_path_buf();
        }
    }
    panic!(
        "No file ending in {suffix} found in {}",
        dir.path().display()
    );
}
