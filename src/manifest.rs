use serde::Serialize;

use crate::convert::SF_META_EXTENSIONS;

#[derive(Serialize)]
pub struct Manifest {
    pub version: String,
    pub supported_metadata: Vec<MetadataEntry>,
}

#[derive(Serialize, Clone)]
pub struct MetadataEntry {
    pub extension: String,
    #[serde(rename = "type")]
    pub meta_type: String,
    pub category: String,
    pub order_sensitive: bool,
    pub supported_formats: Vec<String>,
}

/// Map an extension to (type_name, category, order_sensitive).
fn classify(ext: &str) -> (&str, &str, bool) {
    match ext {
        "profile-meta.xml" => ("Profile", "security", false),
        "permissionset-meta.xml" => ("PermissionSet", "security", false),
        "permissionsetgroup-meta.xml" => ("PermissionSetGroup", "security", false),
        "object-meta.xml" => ("CustomObject", "schema", false),
        "field-meta.xml" => ("CustomField", "schema", false),
        "validationRule-meta.xml" => ("ValidationRule", "schema", false),
        "flow-meta.xml" => ("Flow", "automation", true),
        "layout-meta.xml" => ("Layout", "ui", true),
        "labels-meta.xml" => ("CustomLabels", "ui", false),
        "cls-meta.xml" => ("ApexClass", "code", false),
        "trigger-meta.xml" => ("ApexTrigger", "code", false),
        "component-meta.xml" => ("ApexComponent", "code", false),
        "page-meta.xml" => ("ApexPage", "code", false),
        "js-meta.xml" => ("LightningComponentBundle", "code", false),
        "css-meta.xml" => ("LightningComponentBundle", "code", false),
        "html-meta.xml" => ("LightningComponentBundle", "code", false),
        "xml-meta.xml" => ("LightningComponentBundle", "code", false),
        "email-meta.xml" => ("EmailTemplate", "content", false),
        "workflow-meta.xml" => ("Workflow", "automation", false),
        "app-meta.xml" => ("CustomApplication", "ui", false),
        "tab-meta.xml" => ("CustomTab", "ui", false),
        "flexipage-meta.xml" => ("FlexiPage", "ui", true),
        "site-meta.xml" => ("CustomSite", "ui", false),
        "remoteSite-meta.xml" => ("RemoteSiteSetting", "security", false),
        "cspTrustedSite-meta.xml" => ("CspTrustedSite", "security", false),
        "connectedApp-meta.xml" => ("ConnectedApp", "security", false),
        "customMetadata-meta.xml" => ("CustomMetadata", "schema", false),
        "globalValueSet-meta.xml" => ("GlobalValueSet", "schema", false),
        "standardValueSet-meta.xml" => ("StandardValueSet", "schema", false),
        "quickAction-meta.xml" => ("QuickAction", "ui", false),
        "reportType-meta.xml" => ("ReportType", "analytics", false),
        "report-meta.xml" => ("Report", "analytics", false),
        "dashboard-meta.xml" => ("Dashboard", "analytics", false),
        "pathAssistant-meta.xml" => ("PathAssistant", "ui", false),
        "listView-meta.xml" => ("ListView", "ui", false),
        "recordType-meta.xml" => ("RecordType", "schema", false),
        "compactLayout-meta.xml" => ("CompactLayout", "ui", false),
        "webLink-meta.xml" => ("WebLink", "ui", false),
        "sharingRules-meta.xml" => ("SharingRules", "security", false),
        "assignmentRules-meta.xml" => ("AssignmentRules", "automation", false),
        "autoResponseRules-meta.xml" => ("AutoResponseRules", "automation", false),
        "escalationRules-meta.xml" => ("EscalationRules", "automation", false),
        "matchingRule-meta.xml" => ("MatchingRule", "schema", false),
        "duplicateRule-meta.xml" => ("DuplicateRule", "schema", false),
        _ => ("Unknown", "other", false),
    }
}

pub fn build_manifest() -> Manifest {
    let entries = SF_META_EXTENSIONS
        .iter()
        .map(|ext| {
            let (meta_type, category, order_sensitive) = classify(ext);
            MetadataEntry {
                extension: ext.to_string(),
                meta_type: meta_type.to_string(),
                category: category.to_string(),
                order_sensitive,
                supported_formats: vec!["yaml".to_string(), "json".to_string()],
            }
        })
        .collect();

    Manifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        supported_metadata: entries,
    }
}

/// Return a list of (type_name, order_sensitive, supported_formats) for config init.
pub fn metadata_info_for_config() -> Vec<(String, bool, Vec<String>)> {
    let manifest = build_manifest();
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for entry in manifest.supported_metadata {
        // Deduplicate by type name (e.g. LightningComponentBundle appears multiple times)
        if seen.insert(entry.meta_type.clone()) {
            result.push((
                entry.meta_type,
                entry.order_sensitive,
                entry.supported_formats,
            ));
        }
    }
    result
}
