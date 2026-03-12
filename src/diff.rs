use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::convert;
use crate::json_writer;
use crate::metadata_types;
use crate::xml_parser;
use crate::yaml_writer;

pub struct DiffResult {
    pub new_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub unchanged_files: usize,
}

/// Compare current XML metadata against packed compact files.
/// Shows which files are new, modified, or deleted since last pack.
pub fn diff(sources: &[PathBuf], packed_dir: &Path, include: Option<&str>) -> Result<DiffResult> {
    // Validate
    for p in sources {
        if !p.exists() {
            anyhow::bail!("Path not found: {}", p.display());
        }
    }

    let opts = convert::ConvertOpts {
        paths: sources.to_vec(),
        include: include.map(|s| s.to_string()),
        format_override: None,
    };

    let xml_files = convert::find_sf_xml_files(&opts);
    let root = convert::common_root_from_opts(&opts);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cfg = config::load_config(&cwd).unwrap_or_default();

    let mut result = DiffResult {
        new_files: Vec::new(),
        modified_files: Vec::new(),
        deleted_files: Vec::new(),
        unchanged_files: 0,
    };

    // Check each XML file against its packed counterpart
    for xml_path in &xml_files {
        let short_type = convert::metadata_type_for(xml_path);
        let type_name = metadata_types::manifest_type_for_short(&short_type);
        if cfg.should_skip(&type_name) || cfg.should_skip(&short_type) {
            continue;
        }

        let format = convert::effective_format_for(&short_type, &type_name, &cfg, &None);
        let ext = if format == "json" { "json" } else { "yaml" };
        let compact_path = convert::compact_path_for(xml_path, &root, packed_dir, ext);

        let relative = xml_path
            .strip_prefix(&root)
            .unwrap_or(xml_path)
            .display()
            .to_string();

        if !compact_path.exists() {
            result.new_files.push(relative);
            continue;
        }

        // Generate fresh compact content and compare
        let xml_content = fs::read_to_string(xml_path)
            .with_context(|| format!("Reading {}", xml_path.display()))?;
        let node = xml_parser::parse_xml(&xml_content)
            .with_context(|| format!("Parsing {}", xml_path.display()))?;

        let fresh = if format == "json" {
            json_writer::xml_to_json(&node)?
        } else if format == "yaml-ordered" {
            yaml_writer::xml_to_yaml_ordered(&node)?
        } else {
            yaml_writer::xml_to_yaml(&node)?
        };

        let existing = fs::read_to_string(&compact_path)
            .with_context(|| format!("Reading {}", compact_path.display()))?;

        if fresh.trim() != existing.trim() {
            result.modified_files.push(relative);
        } else {
            result.unchanged_files += 1;
        }
    }

    // Check for deleted files (packed files with no corresponding XML)
    if packed_dir.exists() {
        let packed_opts = convert::ConvertOpts {
            paths: vec![packed_dir.to_path_buf()],
            include: include.map(|s| s.to_string()),
            format_override: None,
        };
        let compact_files = convert::find_compact_files(&packed_opts);
        let packed_root = packed_dir.to_path_buf();

        for compact_path in &compact_files {
            let relative = compact_path
                .strip_prefix(&packed_root)
                .unwrap_or(compact_path);
            let xml_name = relative
                .to_string_lossy()
                .replace("-meta.yaml", "-meta.xml")
                .replace("-meta.json", "-meta.xml");

            // Check if any source contains this XML
            let found = sources.iter().any(|src| src.join(&xml_name).exists());
            if !found {
                result
                    .deleted_files
                    .push(compact_path.display().to_string());
            }
        }
    }

    Ok(result)
}
