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
