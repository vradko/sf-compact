/// Single source of truth for all supported Salesforce metadata types.
/// Every other module derives its metadata knowledge from this array.
pub struct MetadataTypeDef {
    pub extension: &'static str,
    pub meta_type: &'static str,
    pub category: &'static str,
    pub order_sensitive: bool,
}

pub static METADATA_TYPES: &[MetadataTypeDef] = &[
    MetadataTypeDef {
        extension: "profile-meta.xml",
        meta_type: "Profile",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "permissionset-meta.xml",
        meta_type: "PermissionSet",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "permissionsetgroup-meta.xml",
        meta_type: "PermissionSetGroup",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "object-meta.xml",
        meta_type: "CustomObject",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "field-meta.xml",
        meta_type: "CustomField",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "validationRule-meta.xml",
        meta_type: "ValidationRule",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "flow-meta.xml",
        meta_type: "Flow",
        category: "automation",
        order_sensitive: true,
    },
    MetadataTypeDef {
        extension: "layout-meta.xml",
        meta_type: "Layout",
        category: "ui",
        order_sensitive: true,
    },
    MetadataTypeDef {
        extension: "labels-meta.xml",
        meta_type: "CustomLabels",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "cls-meta.xml",
        meta_type: "ApexClass",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "trigger-meta.xml",
        meta_type: "ApexTrigger",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "component-meta.xml",
        meta_type: "ApexComponent",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "page-meta.xml",
        meta_type: "ApexPage",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "js-meta.xml",
        meta_type: "LightningComponentBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "css-meta.xml",
        meta_type: "LightningComponentBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "html-meta.xml",
        meta_type: "LightningComponentBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "xml-meta.xml",
        meta_type: "LightningComponentBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "email-meta.xml",
        meta_type: "EmailTemplate",
        category: "content",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "workflow-meta.xml",
        meta_type: "Workflow",
        category: "automation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "app-meta.xml",
        meta_type: "CustomApplication",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "tab-meta.xml",
        meta_type: "CustomTab",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "flexipage-meta.xml",
        meta_type: "FlexiPage",
        category: "ui",
        order_sensitive: true,
    },
    MetadataTypeDef {
        extension: "site-meta.xml",
        meta_type: "CustomSite",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "remoteSite-meta.xml",
        meta_type: "RemoteSiteSetting",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "cspTrustedSite-meta.xml",
        meta_type: "CspTrustedSite",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "connectedApp-meta.xml",
        meta_type: "ConnectedApp",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "customMetadata-meta.xml",
        meta_type: "CustomMetadata",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "globalValueSet-meta.xml",
        meta_type: "GlobalValueSet",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "standardValueSet-meta.xml",
        meta_type: "StandardValueSet",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "quickAction-meta.xml",
        meta_type: "QuickAction",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "reportType-meta.xml",
        meta_type: "ReportType",
        category: "analytics",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "report-meta.xml",
        meta_type: "Report",
        category: "analytics",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "dashboard-meta.xml",
        meta_type: "Dashboard",
        category: "analytics",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "pathAssistant-meta.xml",
        meta_type: "PathAssistant",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "listView-meta.xml",
        meta_type: "ListView",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "recordType-meta.xml",
        meta_type: "RecordType",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "compactLayout-meta.xml",
        meta_type: "CompactLayout",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "webLink-meta.xml",
        meta_type: "WebLink",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "sharingRules-meta.xml",
        meta_type: "SharingRules",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "assignmentRules-meta.xml",
        meta_type: "AssignmentRules",
        category: "automation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "autoResponseRules-meta.xml",
        meta_type: "AutoResponseRules",
        category: "automation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "escalationRules-meta.xml",
        meta_type: "EscalationRules",
        category: "automation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "matchingRule-meta.xml",
        meta_type: "MatchingRule",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "duplicateRule-meta.xml",
        meta_type: "DuplicateRule",
        category: "schema",
        order_sensitive: false,
    },
    // Additional types found in real-world Salesforce projects
    MetadataTypeDef {
        extension: "settings-meta.xml",
        meta_type: "Settings",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "objectTranslation-meta.xml",
        meta_type: "CustomObjectTranslation",
        category: "translation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "fieldTranslation-meta.xml",
        meta_type: "CustomFieldTranslation",
        category: "translation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "cmp-meta.xml",
        meta_type: "AuraDefinitionBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "evt-meta.xml",
        meta_type: "AuraDefinitionBundle",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "md-meta.xml",
        meta_type: "CustomMetadata",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "fieldSet-meta.xml",
        meta_type: "FieldSet",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "installedPackage-meta.xml",
        meta_type: "InstalledPackage",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "externalServiceRegistration-meta.xml",
        meta_type: "ExternalServiceRegistration",
        category: "integration",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "namedCredential-meta.xml",
        meta_type: "NamedCredential",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "externalCredential-meta.xml",
        meta_type: "ExternalCredential",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "messageChannel-meta.xml",
        meta_type: "LightningMessageChannel",
        category: "code",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "customPermission-meta.xml",
        meta_type: "CustomPermission",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "topicsForObjects-meta.xml",
        meta_type: "TopicsForObjects",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "notiftype-meta.xml",
        meta_type: "CustomNotificationType",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "managedContentType-meta.xml",
        meta_type: "ManagedContentType",
        category: "content",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "cleanDataService-meta.xml",
        meta_type: "CleanDataService",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "indx-meta.xml",
        meta_type: "CustomIndex",
        category: "schema",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "role-meta.xml",
        meta_type: "Role",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "group-meta.xml",
        meta_type: "Group",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "appMenu-meta.xml",
        meta_type: "AppMenu",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "homePageLayout-meta.xml",
        meta_type: "HomePageLayout",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "letter-meta.xml",
        meta_type: "Letterhead",
        category: "content",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "community-meta.xml",
        meta_type: "Community",
        category: "ui",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "authprovider-meta.xml",
        meta_type: "AuthProvider",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "samlssoconfig-meta.xml",
        meta_type: "SamlSsoConfig",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "rule-meta.xml",
        meta_type: "WorkflowRule",
        category: "automation",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "resource-meta.xml",
        meta_type: "StaticResource",
        category: "content",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "notifications-meta.xml",
        meta_type: "NotificationTypeConfig",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "iframeWhiteListUrlSettings-meta.xml",
        meta_type: "IframeWhiteListUrlSettings",
        category: "security",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "config-meta.xml",
        meta_type: "PlatformEventChannelMember",
        category: "config",
        order_sensitive: false,
    },
    MetadataTypeDef {
        extension: "crt-meta.xml",
        meta_type: "Certificate",
        category: "security",
        order_sensitive: false,
    },
];

/// Classify an extension into (type_name, category, order_sensitive).
pub fn classify(ext: &str) -> (&'static str, &'static str, bool) {
    METADATA_TYPES
        .iter()
        .find(|t| t.extension == ext)
        .map(|t| (t.meta_type, t.category, t.order_sensitive))
        .unwrap_or(("Unknown", "other", false))
}

/// Return deduplicated list of (type_name, order_sensitive, supported_formats) for config init.
pub fn metadata_info_for_config() -> Vec<(String, bool, Vec<String>)> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for t in METADATA_TYPES {
        if seen.insert(t.meta_type) {
            result.push((
                t.meta_type.to_string(),
                t.order_sensitive,
                vec![
                    "yaml".to_string(),
                    "yaml-ordered".to_string(),
                    "json".to_string(),
                ],
            ));
        }
    }
    result
}

/// Check if a filename matches any known SF metadata extension.
pub fn is_sf_metadata_filename(name: &str) -> bool {
    METADATA_TYPES.iter().any(|t| name.ends_with(t.extension))
}

/// Look up the manifest type name for a short extension type (e.g., "flow" -> "Flow").
pub fn manifest_type_for_short(short_type: &str) -> String {
    let ext = format!("{short_type}-meta.xml");
    METADATA_TYPES
        .iter()
        .find(|t| t.extension == ext)
        .map(|t| t.meta_type.to_string())
        .unwrap_or_else(|| short_type.to_string())
}
