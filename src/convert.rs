use crate::xml_parser;
use crate::yaml_writer;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const SF_META_EXTENSIONS: &[&str] = &[
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

/// Options for selecting which files to process.
pub struct ConvertOpts {
    /// One or more paths (files or directories).
    pub paths: Vec<PathBuf>,
    /// Optional glob filter (e.g. "profiles/**", "*.flow-meta.xml").
    pub include: Option<String>,
}

pub struct FileStats {
    pub relative_path: String,
    pub original_tokens: usize,
    pub compact_tokens: usize,
}

pub struct ConvertStats {
    pub files_processed: usize,
    pub original_bytes: u64,
    pub compact_bytes: u64,
    pub original_tokens: usize,
    pub compact_tokens: usize,
    pub by_type: IndexMap<String, TypeStats>,
    pub per_file: Vec<FileStats>,
}

pub struct TypeStats {
    pub count: usize,
    pub original_bytes: u64,
    pub compact_bytes: u64,
    pub original_tokens: usize,
    pub compact_tokens: usize,
}

impl TypeStats {
    #[allow(dead_code)]
    pub fn reduction_percent(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        (1.0 - self.compact_bytes as f64 / self.original_bytes as f64) * 100.0
    }

    pub fn token_reduction_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        (1.0 - self.compact_tokens as f64 / self.original_tokens as f64) * 100.0
    }
}

impl ConvertStats {
    fn new() -> Self {
        Self {
            files_processed: 0,
            original_bytes: 0,
            compact_bytes: 0,
            original_tokens: 0,
            compact_tokens: 0,
            by_type: IndexMap::new(),
            per_file: Vec::new(),
        }
    }

    pub fn reduction_percent(&self) -> f64 {
        if self.original_bytes == 0 {
            return 0.0;
        }
        (1.0 - self.compact_bytes as f64 / self.original_bytes as f64) * 100.0
    }

    pub fn token_reduction_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        (1.0 - self.compact_tokens as f64 / self.original_tokens as f64) * 100.0
    }

    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.compact_tokens)
    }
}

/// Count tokens using tiktoken (cl100k_base, used by GPT-4/4o/Claude).
fn count_tokens(text: &str) -> usize {
    use tiktoken_rs::cl100k_base;
    let bpe = cl100k_base().expect("Failed to load cl100k tokenizer");
    bpe.encode_with_special_tokens(text).len()
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

fn find_sf_xml_files_from_opts(opts: &ConvertOpts) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in &opts.paths {
        if path.is_file() {
            if is_sf_metadata(path) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && is_sf_metadata(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    // Apply glob filter if provided
    if let Some(ref pattern) = opts.include {
        files.retain(|f| {
            let name = f.to_string_lossy();
            glob_match(pattern, &name)
        });
    }

    files
}

fn is_sf_compact_yaml(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("-meta.yaml")
}

fn find_yaml_files_from_opts(opts: &ConvertOpts) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in &opts.paths {
        if path.is_file() {
            if is_sf_compact_yaml(path) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && is_sf_compact_yaml(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    if let Some(ref pattern) = opts.include {
        files.retain(|f| {
            let name = f.to_string_lossy();
            glob_match(pattern, &name)
        });
    }

    files
}

/// Glob matching using the `glob` crate's Pattern.
fn glob_match(pattern: &str, path: &str) -> bool {
    let options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    match glob::Pattern::new(pattern) {
        Ok(p) => {
            // Try matching against the full path and also just the filename
            p.matches_with(path, options)
                || Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|name| p.matches_with(name, options))
        }
        Err(_) => path.contains(pattern),
    }
}

/// Determine the common root directory from opts for relative path display.
fn common_root(opts: &ConvertOpts) -> PathBuf {
    if opts.paths.is_empty() {
        return PathBuf::from(".");
    }
    if opts.paths.len() == 1 {
        let p = &opts.paths[0];
        if p.is_dir() {
            return p.clone();
        }
        return p.parent().unwrap_or(p).to_path_buf();
    }
    // Find longest common ancestor of all paths
    let dirs: Vec<PathBuf> = opts
        .paths
        .iter()
        .map(|p| {
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            };
            if abs.is_dir() {
                abs
            } else {
                abs.parent().unwrap_or(&abs).to_path_buf()
            }
        })
        .collect();

    let first: Vec<_> = dirs[0].components().collect();
    let mut prefix_len = first.len();
    for path in &dirs[1..] {
        let comps: Vec<_> = path.components().collect();
        prefix_len = prefix_len.min(
            first
                .iter()
                .zip(comps.iter())
                .take_while(|(a, b)| a == b)
                .count(),
        );
    }
    if prefix_len == 0 {
        return PathBuf::from("/");
    }
    first[..prefix_len].iter().collect()
}

/// Compute relative path safely, falling back to filename if strip_prefix fails.
fn safe_relative(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| {
            // Try with canonicalized paths
            let canon_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            let canon_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
            canon_path
                .strip_prefix(&canon_root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| {
                    // Last resort: use just the filename
                    path.file_name()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| path.to_path_buf())
                })
        })
}

fn yaml_path_for_xml(xml_path: &Path, source_root: &Path, output_root: &Path) -> PathBuf {
    let relative = safe_relative(xml_path, source_root);
    let mut out = output_root.join(relative);
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace("-meta.xml", "-meta.yaml");
    out.set_file_name(name);
    out
}

fn xml_path_for_yaml(yaml_path: &Path, source_root: &Path, output_root: &Path) -> PathBuf {
    let relative = safe_relative(yaml_path, source_root);
    let mut out = output_root.join(relative);
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace("-meta.yaml", "-meta.xml");
    out.set_file_name(name);
    out
}

fn validate_paths(paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        if !p.exists() {
            anyhow::bail!("Path not found: {}", p.display());
        }
    }
    Ok(())
}

/// Pack: XML → YAML
pub fn pack(opts: &ConvertOpts, output: &Path) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_sf_xml_files_from_opts(opts);
    let root = common_root(opts);
    let mut stats = ConvertStats::new();

    for xml_path in &files {
        let xml_content = fs::read_to_string(xml_path)
            .with_context(|| format!("Reading {}", xml_path.display()))?;
        let xml_bytes = xml_content.len() as u64;

        let node = xml_parser::parse_xml(&xml_content)
            .with_context(|| format!("Parsing {}", xml_path.display()))?;

        let yaml = yaml_writer::xml_to_yaml(&node)
            .with_context(|| format!("Converting {}", xml_path.display()))?;
        let yaml_bytes = yaml.len() as u64;

        let yaml_path = yaml_path_for_xml(xml_path, &root, output);
        if let Some(parent) = yaml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&yaml_path, &yaml)?;

        accumulate_stats(&mut stats, xml_path, &root, xml_bytes, yaml_bytes, 0, 0);
    }

    Ok(stats)
}

/// Unpack: YAML → XML
pub fn unpack(opts: &ConvertOpts, output: &Path) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_yaml_files_from_opts(opts);
    let root = common_root(opts);
    let mut stats = ConvertStats::new();

    for yaml_path in &files {
        let yaml_content = fs::read_to_string(yaml_path)
            .with_context(|| format!("Reading {}", yaml_path.display()))?;

        let node = yaml_writer::yaml_to_xml_node(&yaml_content)
            .with_context(|| format!("Converting {}", yaml_path.display()))?;

        let xml = xml_parser::to_xml(&node);

        let xml_path = xml_path_for_yaml(yaml_path, &root, output);
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

/// Stats: preview without writing, with real token counting.
pub fn stats(opts: &ConvertOpts) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_sf_xml_files_from_opts(opts);
    let root = common_root(opts);
    let mut stats = ConvertStats::new();

    for xml_path in &files {
        let xml_content = fs::read_to_string(xml_path)
            .with_context(|| format!("Reading {}", xml_path.display()))?;
        let xml_bytes = xml_content.len() as u64;
        let xml_tokens = count_tokens(&xml_content);

        let node = xml_parser::parse_xml(&xml_content)
            .with_context(|| format!("Parsing {}", xml_path.display()))?;

        let yaml = yaml_writer::xml_to_yaml(&node)
            .with_context(|| format!("Converting {}", xml_path.display()))?;
        let yaml_bytes = yaml.len() as u64;
        let yaml_tokens = count_tokens(&yaml);

        accumulate_stats(
            &mut stats,
            xml_path,
            &root,
            xml_bytes,
            yaml_bytes,
            xml_tokens,
            yaml_tokens,
        );
    }

    Ok(stats)
}

/// Single-path stats for MCP and backward compatibility.
pub fn stats_path(source: &Path) -> Result<ConvertStats> {
    stats(&ConvertOpts {
        paths: vec![source.to_path_buf()],
        include: None,
    })
}

/// Single-path pack for MCP.
pub fn pack_path(source: &Path, output: &Path) -> Result<ConvertStats> {
    pack(
        &ConvertOpts {
            paths: vec![source.to_path_buf()],
            include: None,
        },
        output,
    )
}

/// Single-path unpack for MCP.
pub fn unpack_path(source: &Path, output: &Path) -> Result<ConvertStats> {
    unpack(
        &ConvertOpts {
            paths: vec![source.to_path_buf()],
            include: None,
        },
        output,
    )
}

fn accumulate_stats(
    stats: &mut ConvertStats,
    xml_path: &Path,
    root: &Path,
    xml_bytes: u64,
    yaml_bytes: u64,
    xml_tokens: usize,
    yaml_tokens: usize,
) {
    let relative = xml_path
        .strip_prefix(root)
        .unwrap_or(xml_path)
        .to_string_lossy()
        .to_string();

    let meta_type = metadata_type(xml_path);
    let type_stats = stats.by_type.entry(meta_type).or_insert(TypeStats {
        count: 0,
        original_bytes: 0,
        compact_bytes: 0,
        original_tokens: 0,
        compact_tokens: 0,
    });
    type_stats.count += 1;
    type_stats.original_bytes += xml_bytes;
    type_stats.compact_bytes += yaml_bytes;
    type_stats.original_tokens += xml_tokens;
    type_stats.compact_tokens += yaml_tokens;

    stats.per_file.push(FileStats {
        relative_path: relative,
        original_tokens: xml_tokens,
        compact_tokens: yaml_tokens,
    });

    stats.files_processed += 1;
    stats.original_bytes += xml_bytes;
    stats.compact_bytes += yaml_bytes;
    stats.original_tokens += xml_tokens;
    stats.compact_tokens += yaml_tokens;
}
