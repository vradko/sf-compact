use crate::xml_parser;
use crate::yaml_writer;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SF_META_EXTENSIONS: &[&str] = &[
    "profile-meta.xml",
    "permissionset-meta.xml",
    "permissionsetgroup-meta.xml",
    "object-meta.xml",
    "field-meta.xml",
    "validationRule-meta.xml",
    "flow-meta.xml",
    "layout-meta.xml",
    "labels-meta.xml",
    "cls-meta.xml",
    "trigger-meta.xml",
    "component-meta.xml",
    "page-meta.xml",
    "js-meta.xml",
    "css-meta.xml",
    "html-meta.xml",
    "xml-meta.xml",
    "email-meta.xml",
    "workflow-meta.xml",
    "app-meta.xml",
    "tab-meta.xml",
    "flexipage-meta.xml",
    "site-meta.xml",
    "remoteSite-meta.xml",
    "cspTrustedSite-meta.xml",
    "connectedApp-meta.xml",
    "customMetadata-meta.xml",
    "globalValueSet-meta.xml",
    "standardValueSet-meta.xml",
    "quickAction-meta.xml",
    "reportType-meta.xml",
    "report-meta.xml",
    "dashboard-meta.xml",
    "pathAssistant-meta.xml",
    "listView-meta.xml",
    "recordType-meta.xml",
    "compactLayout-meta.xml",
    "webLink-meta.xml",
    "sharingRules-meta.xml",
    "assignmentRules-meta.xml",
    "autoResponseRules-meta.xml",
    "escalationRules-meta.xml",
    "matchingRule-meta.xml",
    "duplicateRule-meta.xml",
];

pub struct ConvertStats {
    pub files_processed: usize,
    pub original_bytes: u64,
    pub compact_bytes: u64,
    pub by_type: IndexMap<String, TypeStats>,
}

pub struct TypeStats {
    pub count: usize,
    pub original_bytes: u64,
    pub compact_bytes: u64,
}

impl TypeStats {
    pub fn reduction_percent(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        (1.0 - self.compact_bytes as f64 / self.original_bytes as f64) * 100.0
    }
}

impl ConvertStats {
    fn new() -> Self {
        Self {
            files_processed: 0,
            original_bytes: 0,
            compact_bytes: 0,
            by_type: IndexMap::new(),
        }
    }

    pub fn reduction_percent(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        (1.0 - self.compact_bytes as f64 / self.original_bytes as f64) * 100.0
    }

    pub fn tokens_saved(&self) -> u64 {
        // ~4 chars per token approximation
        (self.original_bytes - self.compact_bytes) / 4
    }
}

fn is_sf_metadata(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    SF_META_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

fn metadata_type(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // Extract the type from e.g. "Admin.profile-meta.xml" → "profile"
    if let Some(pos) = name.rfind("-meta.xml") {
        let before = &name[..pos];
        if let Some(dot) = before.rfind('.') {
            return before[dot + 1..].to_string();
        }
    }
    "unknown".to_string()
}

fn find_sf_xml_files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_sf_metadata(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn yaml_path_for_xml(xml_path: &Path, source_root: &Path, output_root: &Path) -> PathBuf {
    let relative = xml_path.strip_prefix(source_root).unwrap_or(xml_path);
    let mut out = output_root.join(relative);
    // Replace .xml extension with .yaml
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace("-meta.xml", "-meta.yaml");
    out.set_file_name(name);
    out
}

fn xml_path_for_yaml(yaml_path: &Path, source_root: &Path, output_root: &Path) -> PathBuf {
    let relative = yaml_path.strip_prefix(source_root).unwrap_or(yaml_path);
    let mut out = output_root.join(relative);
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace("-meta.yaml", "-meta.xml");
    out.set_file_name(name);
    out
}

/// Pack: XML → YAML
pub fn pack(source: &Path, output: &Path) -> Result<ConvertStats> {
    let files = find_sf_xml_files(source);
    let mut stats = ConvertStats::new();

    for xml_path in &files {
        let xml_content =
            fs::read_to_string(xml_path).with_context(|| format!("Reading {}", xml_path.display()))?;
        let xml_bytes = xml_content.len() as u64;

        let node = xml_parser::parse_xml(&xml_content)
            .with_context(|| format!("Parsing {}", xml_path.display()))?;

        let yaml = yaml_writer::xml_to_yaml(&node)
            .with_context(|| format!("Converting {}", xml_path.display()))?;
        let yaml_bytes = yaml.len() as u64;

        let yaml_path = yaml_path_for_xml(xml_path, source, output);
        if let Some(parent) = yaml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&yaml_path, &yaml)?;

        let meta_type = metadata_type(xml_path);
        let type_stats = stats.by_type.entry(meta_type).or_insert(TypeStats {
            count: 0,
            original_bytes: 0,
            compact_bytes: 0,
        });
        type_stats.count += 1;
        type_stats.original_bytes += xml_bytes;
        type_stats.compact_bytes += yaml_bytes;

        stats.files_processed += 1;
        stats.original_bytes += xml_bytes;
        stats.compact_bytes += yaml_bytes;
    }

    Ok(stats)
}

/// Unpack: YAML → XML
pub fn unpack(source: &Path, output: &Path) -> Result<ConvertStats> {
    let yaml_files: Vec<PathBuf> = WalkDir::new(source)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext == "yaml" || ext == "yml")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let mut stats = ConvertStats::new();

    for yaml_path in &yaml_files {
        let yaml_content =
            fs::read_to_string(yaml_path).with_context(|| format!("Reading {}", yaml_path.display()))?;

        let node = yaml_writer::yaml_to_xml_node(&yaml_content)
            .with_context(|| format!("Converting {}", yaml_path.display()))?;

        let xml = xml_parser::to_xml(&node);

        let xml_path = xml_path_for_yaml(yaml_path, source, output);
        if let Some(parent) = xml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&xml_path, &xml)?;

        stats.files_processed += 1;
        stats.compact_bytes += yaml_content.len() as u64;
        stats.original_bytes += xml.len() as u64;
    }

    Ok(stats)
}

/// Stats: analyze without writing
pub fn stats(source: &Path) -> Result<ConvertStats> {
    let files = find_sf_xml_files(source);
    let mut stats = ConvertStats::new();

    for xml_path in &files {
        let xml_content =
            fs::read_to_string(xml_path).with_context(|| format!("Reading {}", xml_path.display()))?;
        let xml_bytes = xml_content.len() as u64;

        let node = xml_parser::parse_xml(&xml_content)
            .with_context(|| format!("Parsing {}", xml_path.display()))?;

        let yaml = yaml_writer::xml_to_yaml(&node)
            .with_context(|| format!("Converting {}", xml_path.display()))?;
        let yaml_bytes = yaml.len() as u64;

        let meta_type = metadata_type(xml_path);
        let type_stats = stats.by_type.entry(meta_type).or_insert(TypeStats {
            count: 0,
            original_bytes: 0,
            compact_bytes: 0,
        });
        type_stats.count += 1;
        type_stats.original_bytes += xml_bytes;
        type_stats.compact_bytes += yaml_bytes;

        stats.files_processed += 1;
        stats.original_bytes += xml_bytes;
        stats.compact_bytes += yaml_bytes;
    }

    Ok(stats)
}
