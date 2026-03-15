use crate::config;
use crate::constants::{self, SUFFIX_META_JSON, SUFFIX_META_XML, SUFFIX_META_YAML};
use crate::json_writer;
use crate::metadata_types;
use crate::xml_parser;
use crate::yaml_writer;
use anyhow::Result;
use indexmap::IndexMap;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Options for selecting which files to process.
pub struct ConvertOpts {
    /// One or more paths (files or directories).
    pub paths: Vec<PathBuf>,
    /// Optional glob filter (e.g. "profiles/**", "*.flow-meta.xml").
    pub include: Option<String>,
    /// CLI format override (takes precedence over config).
    pub format_override: Option<String>,
    /// Only process files modified since last pack (by mtime comparison).
    pub incremental: bool,
    /// CLI override for preserve_comments (None = use config).
    pub preserve_comments: Option<bool>,
    /// CLI override for indent size (None = use config).
    pub indent: Option<u8>,
}

pub struct FileStats {
    pub relative_path: String,
    pub original_tokens: usize,
    pub compact_tokens: usize,
}

pub struct PackedFileInfo {
    pub xml_relative_path: String,
    pub compact_absolute_path: PathBuf,
}

pub struct ConvertStats {
    pub files_processed: usize,
    pub original_bytes: u64,
    pub compact_bytes: u64,
    pub original_tokens: usize,
    pub compact_tokens: usize,
    pub by_type: IndexMap<String, TypeStats>,
    pub per_file: Vec<FileStats>,
    pub packed_files: Vec<PackedFileInfo>,
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
            packed_files: Vec::new(),
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
    metadata_types::is_sf_metadata_filename(name)
}

fn metadata_type(path: &Path) -> String {
    metadata_type_for_ext(path, SUFFIX_META_XML)
}

fn metadata_type_for_ext(path: &Path, suffix: &str) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if let Some(pos) = name.rfind(suffix) {
        let before = &name[..pos];
        if let Some(dot) = before.rfind('.') {
            return before[dot + 1..].to_string();
        }
    }
    "unknown".to_string()
}

/// Map a short metadata_type (like "flow") to the manifest type name (like "Flow").
fn manifest_type_name(short_type: &str) -> String {
    metadata_types::manifest_type_for_short(short_type)
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

fn is_sf_compact_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(SUFFIX_META_YAML) || name.ends_with(SUFFIX_META_JSON)
}

fn find_compact_files_from_opts(opts: &ConvertOpts) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in &opts.paths {
        if path.is_file() {
            if is_sf_compact_file(path) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && is_sf_compact_file(entry.path()) {
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

/// Compute compact output path for an XML file, using the given format extension.
fn compact_path_for_xml(
    xml_path: &Path,
    source_root: &Path,
    output_root: &Path,
    format: &str,
) -> PathBuf {
    let relative = safe_relative(xml_path, source_root);
    let mut out = output_root.join(relative);
    let ext = if format == constants::FORMAT_JSON {
        SUFFIX_META_JSON
    } else {
        SUFFIX_META_YAML
    };
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace(SUFFIX_META_XML, ext);
    out.set_file_name(name);
    out
}

/// Compute XML output path for a compact file (YAML or JSON).
fn xml_path_for_compact(compact_path: &Path, source_root: &Path, output_root: &Path) -> PathBuf {
    let relative = safe_relative(compact_path, source_root);
    let mut out = output_root.join(relative);
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .replace(SUFFIX_META_YAML, SUFFIX_META_XML)
        .replace(SUFFIX_META_JSON, SUFFIX_META_XML);
    out.set_file_name(name);
    out
}

/// Check if source file is newer than target file (by mtime).
fn is_newer_than(source: &Path, target: &Path) -> bool {
    if !target.exists() {
        return true;
    }
    let src_mtime = fs::metadata(source).and_then(|m| m.modified()).ok();
    let tgt_mtime = fs::metadata(target).and_then(|m| m.modified()).ok();
    match (src_mtime, tgt_mtime) {
        (Some(s), Some(t)) => s > t,
        _ => true, // if we can't read mtime, reprocess
    }
}

fn validate_paths(paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        if !p.exists() {
            anyhow::bail!("Path not found: {}", p.display());
        }
    }
    Ok(())
}

/// Determine effective format for a file, considering CLI override and config.
fn effective_format(
    short_type: &str,
    type_name: &str,
    cfg: &config::SfCompactConfig,
    cli_override: &Option<String>,
) -> String {
    if let Some(fmt) = cli_override {
        return fmt.clone();
    }
    let format = cfg.format_for_type(type_name);
    let format_alt = cfg.format_for_type(short_type);
    if format != constants::FORMAT_YAML {
        format.to_string()
    } else {
        format_alt.to_string()
    }
}

/// Load config searching from source paths first, then cwd as fallback.
/// Validates that all format values in the config are valid.
pub fn load_config_from_sources(opts: &ConvertOpts) -> Result<config::SfCompactConfig> {
    let root = common_root(opts);
    // Try loading from source directory first
    if let Ok(cfg) = config::load_config(&root) {
        if config::find_config_file(&root).is_some() {
            cfg.validate()?;
            return Ok(cfg);
        }
    }
    // Fallback to cwd
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cfg = config::load_config(&cwd)?;
    cfg.validate()?;
    Ok(cfg)
}

/// Result of processing a single file (used for parallel collection).
struct PackedFile {
    xml_path: PathBuf,
    compact_content: String,
    out_path: PathBuf,
    stale_path: PathBuf,
    xml_bytes: u64,
    compact_bytes: u64,
}

/// Pack: XML → compact format (YAML or JSON)
pub fn pack(opts: &ConvertOpts, output: &Path) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_sf_xml_files_from_opts(opts);
    let root = common_root(opts);

    let cfg = load_config_from_sources(opts)?;

    // Process files in parallel: parse XML → convert to compact format
    let results: Vec<Option<PackedFile>> = files
        .par_iter()
        .map(|xml_path| {
            let short_type = metadata_type(xml_path);
            let type_name = manifest_type_name(&short_type);
            if cfg.should_skip(&type_name) || cfg.should_skip(&short_type) {
                return None;
            }

            let format = effective_format(&short_type, &type_name, &cfg, &opts.format_override);

            // Incremental: skip files that haven't changed since last pack
            if opts.incremental {
                let ext = if format == constants::FORMAT_JSON {
                    constants::FORMAT_JSON
                } else {
                    "yaml"
                };
                let out_path = compact_path_for_xml(xml_path, &root, output, ext);
                if !is_newer_than(xml_path, &out_path) {
                    return None;
                }
            }

            let xml_content = match fs::read_to_string(xml_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: skipping {} (read error: {e})", xml_path.display());
                    return None;
                }
            };
            let xml_bytes = xml_content.len() as u64;

            let preserve_comments = opts.preserve_comments.unwrap_or(cfg.preserve_comments);
            let parse_opts = xml_parser::ParseOptions { preserve_comments };
            let node = match xml_parser::parse_xml_with_options(&xml_content, &parse_opts) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!(
                        "Warning: skipping {} (invalid XML: {e})",
                        xml_path.display()
                    );
                    return None;
                }
            };

            let (compact_content, ext) = if format == constants::FORMAT_JSON {
                match json_writer::xml_to_json(&node) {
                    Ok(json) => (json, constants::FORMAT_JSON),
                    Err(e) => {
                        eprintln!("Warning: skipping {} ({e})", xml_path.display());
                        return None;
                    }
                }
            } else if format == constants::FORMAT_YAML_ORDERED {
                match yaml_writer::xml_to_yaml_ordered(&node) {
                    Ok(yaml) => (yaml, "yaml"),
                    Err(e) => {
                        eprintln!("Warning: skipping {} ({e})", xml_path.display());
                        return None;
                    }
                }
            } else {
                match yaml_writer::xml_to_yaml(&node) {
                    Ok(yaml) => (yaml, "yaml"),
                    Err(e) => {
                        eprintln!("Warning: skipping {} ({e})", xml_path.display());
                        return None;
                    }
                }
            };
            let compact_bytes = compact_content.len() as u64;

            let out_path = compact_path_for_xml(xml_path, &root, output, ext);
            let stale_ext = if ext == constants::FORMAT_JSON {
                constants::FORMAT_YAML
            } else {
                constants::FORMAT_JSON
            };
            let stale_path = compact_path_for_xml(xml_path, &root, output, stale_ext);

            Some(PackedFile {
                xml_path: xml_path.clone(),
                compact_content,
                out_path,
                stale_path,
                xml_bytes,
                compact_bytes,
            })
        })
        .collect();

    // Write files and accumulate stats (sequential for filesystem safety)
    let mut stats = ConvertStats::new();
    for item in results.into_iter().flatten() {
        if let Some(parent) = item.out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&item.out_path, &item.compact_content)?;

        if item.stale_path.exists() {
            let _ = fs::remove_file(&item.stale_path);
        }

        let xml_relative = item
            .xml_path
            .strip_prefix(&root)
            .unwrap_or(&item.xml_path)
            .to_string_lossy()
            .to_string();
        let compact_absolute = if item.out_path.is_absolute() {
            item.out_path.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(&item.out_path)
        };
        stats.packed_files.push(PackedFileInfo {
            xml_relative_path: xml_relative,
            compact_absolute_path: compact_absolute,
        });

        accumulate_stats(
            &mut stats,
            &item.xml_path,
            &root,
            item.xml_bytes,
            item.compact_bytes,
            0,
            0,
        );
    }

    Ok(stats)
}

/// Result of unpacking a single file.
struct UnpackedFile {
    xml_content: String,
    xml_path: PathBuf,
    compact_bytes: u64,
    xml_bytes: u64,
}

/// Unpack: compact format (YAML or JSON) → XML
pub fn unpack(opts: &ConvertOpts, output: &Path) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_compact_files_from_opts(opts);
    let root = common_root(opts);

    let cfg = load_config_from_sources(opts).unwrap_or_default();
    let indent_size = opts.indent.unwrap_or(cfg.indent) as usize;

    // Process files in parallel: parse compact → convert to XML
    let results: Vec<Option<UnpackedFile>> = files
        .par_iter()
        .map(|compact_path| {
            let content = match fs::read_to_string(compact_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "Warning: skipping {} (read error: {e})",
                        compact_path.display()
                    );
                    return None;
                }
            };

            let is_json = compact_path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(SUFFIX_META_JSON));

            let node = if is_json {
                match json_writer::json_to_xml_node(&content) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!(
                            "Warning: skipping {} (not valid sf-compact JSON: {e})",
                            compact_path.display()
                        );
                        return None;
                    }
                }
            } else {
                match yaml_writer::yaml_to_xml_node(&content) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!(
                            "Warning: skipping {} (not valid sf-compact YAML: {e})",
                            compact_path.display()
                        );
                        return None;
                    }
                }
            };

            let xml = xml_parser::to_xml_with_indent(&node, indent_size);
            let xml_path = xml_path_for_compact(compact_path, &root, output);

            Some(UnpackedFile {
                xml_bytes: xml.len() as u64,
                xml_content: xml,
                xml_path,
                compact_bytes: content.len() as u64,
            })
        })
        .collect();

    // Write files and accumulate stats
    let mut stats = ConvertStats::new();
    for item in results.into_iter().flatten() {
        if let Some(parent) = item.xml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&item.xml_path, &item.xml_content)?;

        stats.files_processed += 1;
        stats.compact_bytes += item.compact_bytes;
        stats.original_bytes += item.xml_bytes;
    }

    Ok(stats)
}

/// Per-file stats result for parallel collection.
struct FileStatsResult {
    xml_path: PathBuf,
    xml_bytes: u64,
    compact_bytes: u64,
    xml_tokens: usize,
    compact_tokens: usize,
}

/// Stats: preview without writing, with real token counting.
/// Respects config and format_override to match what pack would produce.
pub fn stats(opts: &ConvertOpts) -> Result<ConvertStats> {
    validate_paths(&opts.paths)?;
    let files = find_sf_xml_files_from_opts(opts);
    let root = common_root(opts);

    let cfg = load_config_from_sources(opts)?;

    // Process files in parallel (token counting is CPU-heavy)
    let results: Vec<Option<FileStatsResult>> = files
        .par_iter()
        .map(|xml_path| {
            let short_type = metadata_type(xml_path);
            let type_name = manifest_type_name(&short_type);
            if cfg.should_skip(&type_name) || cfg.should_skip(&short_type) {
                return None;
            }

            let format = effective_format(&short_type, &type_name, &cfg, &opts.format_override);

            let xml_content = match fs::read_to_string(xml_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: skipping {} (read error: {e})", xml_path.display());
                    return None;
                }
            };
            let xml_bytes = xml_content.len() as u64;
            let xml_tokens = count_tokens(&xml_content);

            let preserve_comments = opts.preserve_comments.unwrap_or(cfg.preserve_comments);
            let parse_opts = xml_parser::ParseOptions { preserve_comments };
            let node = match xml_parser::parse_xml_with_options(&xml_content, &parse_opts) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!(
                        "Warning: skipping {} (invalid XML: {e})",
                        xml_path.display()
                    );
                    return None;
                }
            };

            let compact = if format == constants::FORMAT_JSON {
                json_writer::xml_to_json(&node).ok()?
            } else if format == constants::FORMAT_YAML_ORDERED {
                yaml_writer::xml_to_yaml_ordered(&node).ok()?
            } else {
                yaml_writer::xml_to_yaml(&node).ok()?
            };
            let compact_bytes = compact.len() as u64;
            let compact_tokens = count_tokens(&compact);

            Some(FileStatsResult {
                xml_path: xml_path.clone(),
                xml_bytes,
                compact_bytes,
                xml_tokens,
                compact_tokens,
            })
        })
        .collect();

    // Merge results
    let mut stats = ConvertStats::new();
    for item in results.into_iter().flatten() {
        accumulate_stats(
            &mut stats,
            &item.xml_path,
            &root,
            item.xml_bytes,
            item.compact_bytes,
            item.xml_tokens,
            item.compact_tokens,
        );
    }

    Ok(stats)
}

// ─── Public accessors for diff module ────────────────────────────

/// Find SF metadata XML files (public for diff).
pub fn find_sf_xml_files(opts: &ConvertOpts) -> Vec<PathBuf> {
    find_sf_xml_files_from_opts(opts)
}

/// Find compact files (public for diff).
pub fn find_compact_files(opts: &ConvertOpts) -> Vec<PathBuf> {
    find_compact_files_from_opts(opts)
}

/// Common root (public for diff).
pub fn common_root_from_opts(opts: &ConvertOpts) -> PathBuf {
    common_root(opts)
}

/// Metadata type for a path (public for diff).
pub fn metadata_type_for(path: &Path) -> String {
    metadata_type(path)
}

/// Effective format (public for diff).
pub fn effective_format_for(
    short_type: &str,
    type_name: &str,
    cfg: &config::SfCompactConfig,
    cli_override: &Option<String>,
) -> String {
    effective_format(short_type, type_name, cfg, cli_override)
}

/// Compact path for XML (public for diff).
pub fn compact_path_for(
    xml_path: &Path,
    source_root: &Path,
    output_root: &Path,
    format: &str,
) -> PathBuf {
    compact_path_for_xml(xml_path, source_root, output_root, format)
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
