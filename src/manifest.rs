use serde::Serialize;

use crate::convert::SF_META_EXTENSIONS;

#[derive(Serialize)]
pub struct Manifest {
    pub version: String,
    pub supported_metadata: Vec<MetadataEntry>,
}

#[derive(Serialize)]
pub struct MetadataEntry {
    pub extension: String,
    #[serde(rename = "type")]
    pub meta_type: String,
    pub category: String,
}

/// Map an extension to a human-readable type name and category.
fn classify(ext: &str) -> (&str, &str) {
    match ext {
        "profile-meta.xml" => ("Profile", "security"),
        "permissionset-meta.xml" => ("PermissionSet", "security"),
        "permissionsetgroup-meta.xml" => ("PermissionSetGroup", "security"),
        "object-meta.xml" => ("CustomObject", "schema"),
        "field-meta.xml" => ("CustomField", "schema"),
        "validationRule-meta.xml" => ("ValidationRule", "schema"),
        "flow-meta.xml" => ("Flow", "automation"),
        "layout-meta.xml" => ("Layout", "ui"),
        "labels-meta.xml" => ("CustomLabels", "ui"),
        "cls-meta.xml" => ("ApexClass", "code"),
        "trigger-meta.xml" => ("ApexTrigger", "code"),
        "component-meta.xml" => ("ApexComponent", "code"),
        "page-meta.xml" => ("ApexPage", "code"),
        "js-meta.xml" => ("LightningComponentBundle", "code"),
        "css-meta.xml" => ("LightningComponentBundle", "code"),
        "html-meta.xml" => ("LightningComponentBundle", "code"),
        "xml-meta.xml" => ("LightningComponentBundle", "code"),
        "email-meta.xml" => ("EmailTemplate", "content"),
        "workflow-meta.xml" => ("Workflow", "automation"),
        "app-meta.xml" => ("CustomApplication", "ui"),
        "tab-meta.xml" => ("CustomTab", "ui"),
        "flexipage-meta.xml" => ("FlexiPage", "ui"),
        "site-meta.xml" => ("CustomSite", "ui"),
        "remoteSite-meta.xml" => ("RemoteSiteSetting", "security"),
        "cspTrustedSite-meta.xml" => ("CspTrustedSite", "security"),
        "connectedApp-meta.xml" => ("ConnectedApp", "security"),
        "customMetadata-meta.xml" => ("CustomMetadata", "schema"),
        "globalValueSet-meta.xml" => ("GlobalValueSet", "schema"),
        "standardValueSet-meta.xml" => ("StandardValueSet", "schema"),
        "quickAction-meta.xml" => ("QuickAction", "ui"),
        "reportType-meta.xml" => ("ReportType", "analytics"),
        "report-meta.xml" => ("Report", "analytics"),
        "dashboard-meta.xml" => ("Dashboard", "analytics"),
        "pathAssistant-meta.xml" => ("PathAssistant", "ui"),
        "listView-meta.xml" => ("ListView", "ui"),
        "recordType-meta.xml" => ("RecordType", "schema"),
        "compactLayout-meta.xml" => ("CompactLayout", "ui"),
        "webLink-meta.xml" => ("WebLink", "ui"),
        "sharingRules-meta.xml" => ("SharingRules", "security"),
        "assignmentRules-meta.xml" => ("AssignmentRules", "automation"),
        "autoResponseRules-meta.xml" => ("AutoResponseRules", "automation"),
        "escalationRules-meta.xml" => ("EscalationRules", "automation"),
        "matchingRule-meta.xml" => ("MatchingRule", "schema"),
        "duplicateRule-meta.xml" => ("DuplicateRule", "schema"),
        _ => ("Unknown", "other"),
    }
}

pub fn build_manifest() -> Manifest {
    let entries = SF_META_EXTENSIONS
        .iter()
        .map(|ext| {
            let (meta_type, category) = classify(ext);
            MetadataEntry {
                extension: ext.to_string(),
                meta_type: meta_type.to_string(),
                category: category.to_string(),
            }
        })
        .collect();

    Manifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        supported_metadata: entries,
    }
}
