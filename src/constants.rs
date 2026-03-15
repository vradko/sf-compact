/// Output format identifiers.
pub const FORMAT_YAML: &str = "yaml";
pub const FORMAT_YAML_ORDERED: &str = "yaml-ordered";
pub const FORMAT_JSON: &str = "json";

/// Valid format values for validation.
pub const VALID_FORMATS: &[&str] = &[FORMAT_YAML, FORMAT_YAML_ORDERED, FORMAT_JSON];

/// Reserved keys used in compact representations.
pub const KEY_TAG: &str = "_tag";
pub const KEY_NS: &str = "_ns";
pub const KEY_ATTRS: &str = "_attrs";
pub const KEY_TEXT: &str = "_text";
pub const KEY_CHILDREN: &str = "_children";
pub const KEY_COMMENT: &str = "_comment";

/// Default paths.
pub const DEFAULT_SOURCE: &str = "force-app";
pub const DEFAULT_OUTPUT: &str = ".sf-compact";

/// File suffixes for metadata files.
pub const SUFFIX_META_XML: &str = "-meta.xml";
pub const SUFFIX_META_YAML: &str = "-meta.yaml";
pub const SUFFIX_META_JSON: &str = "-meta.json";

/// Tracking.
pub const TRACKING_DIR: &str = ".tracking";
pub const TRACKING_BRANCH_SEPARATOR: &str = "--";
pub const TRACKING_DEFAULT_BRANCH: &str = "unknown";
