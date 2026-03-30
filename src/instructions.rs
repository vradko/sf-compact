use crate::config;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Marker constants
// ---------------------------------------------------------------------------

const MARKER_START: &str = "<!-- sf-compact:start -->";
const MARKER_END: &str = "<!-- sf-compact:end -->";

// ---------------------------------------------------------------------------
// Target definitions
// ---------------------------------------------------------------------------

/// Known AI tool instruction file targets.
pub struct Target {
    pub name: &'static str,
    pub file_path: &'static str,
}

const TARGETS: &[Target] = &[
    Target {
        name: "claude",
        file_path: "CLAUDE.md",
    },
    Target {
        name: "cursor",
        file_path: ".cursorrules",
    },
    Target {
        name: "copilot",
        file_path: ".github/copilot-instructions.md",
    },
    Target {
        name: "codex",
        file_path: "AGENTS.md",
    },
    Target {
        name: "windsurf",
        file_path: ".windsurfrules",
    },
    Target {
        name: "cline",
        file_path: ".clinerules",
    },
    Target {
        name: "aider",
        file_path: ".aiderules",
    },
];

/// Resolve a target name to its file path.
fn target_file(name: &str) -> Option<&'static str> {
    TARGETS.iter().find(|t| t.name == name).map(|t| t.file_path)
}

// ---------------------------------------------------------------------------
// Directive template
// ---------------------------------------------------------------------------

fn generate_directive(output_dir: &str, source_dir: &str) -> String {
    format!(
        r#"{MARKER_START}
## Salesforce Metadata — sf-compact

This project uses `sf-compact` to compress Salesforce metadata XML into compact YAML/JSON.
Compact files are in `{output_dir}/` and mirror the `{source_dir}/` structure exactly (same
paths, only the file extension changes: `-meta.xml` → `-meta.yaml` or `-meta.json`).

**When reading Salesforce metadata** (objects, fields, profiles, permission sets, flows,
layouts, etc.), prefer `{output_dir}/` over `{source_dir}/` — the files are smaller and
contain identical information.

**Path mapping**: replace `{source_dir}/` with `{output_dir}/` and `.xml` with `.yaml`/`.json`.
Example: `{source_dir}/main/default/objects/Account/Account.object-meta.xml`
→ `{output_dir}/main/default/objects/Account/Account.object-meta.json`

**If `{output_dir}/` is missing or outdated**, run:
```
sf-compact pack
```

**If you need to modify metadata**, edit the compact file in `{output_dir}/`, then run:
```
sf-compact unpack
```

**Non-metadata files** (Apex classes `.cls`, triggers `.trigger`, LWC `.js/.html/.css`)
should still be read directly from `{source_dir}/`.
{MARKER_END}"#
    )
}

// ---------------------------------------------------------------------------
// Config-aware parameter resolution
// ---------------------------------------------------------------------------

struct ResolvedConfig {
    output_dir: String,
    source_dir: String,
}

fn resolve_config(project_root: &Path) -> ResolvedConfig {
    // Try to read .sfcompact.yaml for output/source overrides.
    // The config struct doesn't have output/source fields yet,
    // so we fall back to defaults from constants.
    let _ = config::load_config(project_root);
    ResolvedConfig {
        output_dir: crate::constants::DEFAULT_OUTPUT.to_string(),
        source_dir: crate::constants::DEFAULT_SOURCE.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Marker-based injection / replacement / removal
// ---------------------------------------------------------------------------

/// Inject or replace the directive block in existing file content.
/// Returns the new file content.
fn inject_block(existing_content: &str, block: &str) -> String {
    if let Some(replaced) = replace_between_markers(existing_content, block) {
        replaced
    } else {
        // No markers found — append
        let mut result = existing_content.to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(block);
        result.push('\n');
        result
    }
}

/// Replace everything between (and including) markers. Returns None if no markers found.
fn replace_between_markers(content: &str, replacement: &str) -> Option<String> {
    let start_idx = content.find(MARKER_START)?;
    let end_idx = content.find(MARKER_END)?;
    if end_idx < start_idx {
        return None;
    }
    let end_idx = end_idx + MARKER_END.len();
    // Consume trailing newline after end marker if present
    let end_idx = if content[end_idx..].starts_with('\n') {
        end_idx + 1
    } else {
        end_idx
    };

    let mut result = String::new();
    result.push_str(&content[..start_idx]);
    result.push_str(replacement);
    result.push('\n');
    result.push_str(&content[end_idx..]);
    Some(result)
}

/// Remove the directive block (between markers, inclusive) from content.
/// Returns None if no markers found.
fn remove_block(content: &str) -> Option<String> {
    let start_idx = content.find(MARKER_START)?;
    let end_idx = content.find(MARKER_END)?;
    if end_idx < start_idx {
        return None;
    }
    let end_idx = end_idx + MARKER_END.len();
    // Consume trailing newline
    let end_idx = if content[end_idx..].starts_with('\n') {
        end_idx + 1
    } else {
        end_idx
    };
    // Also consume one blank line before the block if present
    let start_idx = if start_idx > 0 && content[..start_idx].ends_with('\n') {
        start_idx - 1
    } else {
        start_idx
    };
    let mut result = String::new();
    result.push_str(&content[..start_idx]);
    result.push_str(&content[end_idx..]);
    Some(result)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Auto-detect which AI instruction files exist in the project root.
/// Returns file paths relative to project_root.
fn auto_detect_targets(project_root: &Path) -> Vec<&'static str> {
    let mut found = Vec::new();
    for target in TARGETS {
        let path = project_root.join(target.file_path);
        if path.exists() {
            found.push(target.file_path);
        }
    }
    found
}

/// Main entry point: inject directive into target file(s).
pub fn inject(project_root: &Path, target: &str) -> Result<Vec<String>> {
    let cfg = resolve_config(project_root);
    let block = generate_directive(&cfg.output_dir, &cfg.source_dir);
    let mut results = Vec::new();

    if target == "stdout" {
        println!("{block}");
        results.push("stdout".to_string());
        return Ok(results);
    }

    let file_paths: Vec<&str> = if target == "auto" {
        let detected = auto_detect_targets(project_root);
        if detected.is_empty() {
            // Default to CLAUDE.md
            vec!["CLAUDE.md"]
        } else {
            detected
        }
    } else {
        match target_file(target) {
            Some(path) => vec![path],
            None => anyhow::bail!(
                "Unknown target '{}'. Valid targets: auto, stdout, {}",
                target,
                TARGETS
                    .iter()
                    .map(|t| t.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    };

    for rel_path in file_paths {
        let full_path = project_root.join(rel_path);

        // Create parent dirs if needed (e.g., .github/)
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory {}", parent.display()))?;
            }
        }

        let existing = if full_path.exists() {
            fs::read_to_string(&full_path)
                .with_context(|| format!("Failed to read {}", full_path.display()))?
        } else {
            String::new()
        };

        let new_content = inject_block(&existing, &block);
        fs::write(&full_path, &new_content)
            .with_context(|| format!("Failed to write {}", full_path.display()))?;

        results.push(rel_path.to_string());
    }

    Ok(results)
}

/// Remove sf-compact directive blocks from all known AI instruction files.
pub fn remove(project_root: &Path) -> Result<Vec<String>> {
    let mut results = Vec::new();

    for target in TARGETS {
        let full_path = project_root.join(target.file_path);
        if !full_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read {}", full_path.display()))?;

        if let Some(cleaned) = remove_block(&content) {
            let trimmed = cleaned.trim();
            if trimmed.is_empty() {
                fs::remove_file(&full_path)
                    .with_context(|| format!("Failed to remove {}", full_path.display()))?;
                results.push(format!("{} (deleted — was empty)", target.file_path));
            } else {
                fs::write(&full_path, format!("{trimmed}\n"))
                    .with_context(|| format!("Failed to write {}", full_path.display()))?;
                results.push(target.file_path.to_string());
            }
        }
    }

    Ok(results)
}

/// Generate the legacy standalone instructions file (full reference manual).
/// This preserves the old `--name` behavior.
pub fn generate_legacy_instructions() -> String {
    crate::mcp::generate_instructions()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_contains_markers() {
        let d = generate_directive(".sf-compact", "force-app");
        assert!(d.starts_with(MARKER_START));
        assert!(d.ends_with(MARKER_END));
    }

    #[test]
    fn directive_substitutes_paths() {
        let d = generate_directive("output-dir", "src-dir");
        assert!(d.contains("`output-dir/`"));
        assert!(d.contains("`src-dir/`"));
        assert!(!d.contains("{output_dir}"));
        assert!(!d.contains("{source_dir}"));
    }

    #[test]
    fn inject_into_empty() {
        let block = "<!-- sf-compact:start -->\ntest\n<!-- sf-compact:end -->";
        let result = inject_block("", block);
        assert_eq!(result, format!("{block}\n"));
    }

    #[test]
    fn inject_appends_to_existing() {
        let existing = "# My Project\n\nSome content.\n";
        let block = "<!-- sf-compact:start -->\ntest\n<!-- sf-compact:end -->";
        let result = inject_block(existing, block);
        assert!(result.starts_with("# My Project"));
        assert!(result.contains(block));
        // Two newlines between existing and block
        assert!(result.contains("Some content.\n\n<!-- sf-compact:start -->"));
    }

    #[test]
    fn inject_replaces_existing_block() {
        let existing = "# Header\n\n<!-- sf-compact:start -->\nold content\n<!-- sf-compact:end -->\n\n# Footer\n";
        let block = "<!-- sf-compact:start -->\nnew content\n<!-- sf-compact:end -->";
        let result = inject_block(existing, block);
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
    }

    #[test]
    fn remove_block_from_content() {
        let content = "# Header\n\n<!-- sf-compact:start -->\nstuff\n<!-- sf-compact:end -->\n\n# Footer\n";
        let result = remove_block(content).unwrap();
        assert!(!result.contains("sf-compact"));
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
    }

    #[test]
    fn remove_block_returns_none_if_no_markers() {
        assert!(remove_block("no markers here").is_none());
    }

    #[test]
    fn auto_detect_returns_empty_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let detected = auto_detect_targets(dir.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn auto_detect_finds_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# test").unwrap();
        let detected = auto_detect_targets(dir.path());
        assert_eq!(detected, vec!["CLAUDE.md"]);
    }

    #[test]
    fn auto_detect_finds_multiple() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "").unwrap();
        fs::write(dir.path().join(".cursorrules"), "").unwrap();
        fs::write(dir.path().join("AGENTS.md"), "").unwrap();
        let detected = auto_detect_targets(dir.path());
        assert_eq!(detected.len(), 3);
    }

    #[test]
    fn inject_and_remove_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("CLAUDE.md");
        fs::write(&file, "# My Project\n").unwrap();

        // Inject
        let results = inject(dir.path(), "claude").unwrap();
        assert_eq!(results, vec!["CLAUDE.md"]);

        let content = fs::read_to_string(&file).unwrap();
        assert!(content.contains(MARKER_START));
        assert!(content.contains("# My Project"));

        // Remove
        let results = remove(dir.path()).unwrap();
        assert_eq!(results.len(), 1);

        let content = fs::read_to_string(&file).unwrap();
        assert!(!content.contains(MARKER_START));
        assert!(content.contains("# My Project"));
    }

    #[test]
    fn inject_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("CLAUDE.md");
        fs::write(&file, "# Header\n").unwrap();

        inject(dir.path(), "claude").unwrap();
        let first = fs::read_to_string(&file).unwrap();

        inject(dir.path(), "claude").unwrap();
        let second = fs::read_to_string(&file).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn inject_stdout_does_not_create_file() {
        let dir = tempfile::tempdir().unwrap();
        let results = inject(dir.path(), "stdout").unwrap();
        assert_eq!(results, vec!["stdout"]);
        // No files created
        assert!(!dir.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn inject_auto_defaults_to_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let results = inject(dir.path(), "auto").unwrap();
        assert_eq!(results, vec!["CLAUDE.md"]);
        assert!(dir.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn inject_copilot_creates_github_dir() {
        let dir = tempfile::tempdir().unwrap();
        let results = inject(dir.path(), "copilot").unwrap();
        assert_eq!(results, vec![".github/copilot-instructions.md"]);
        assert!(dir.path().join(".github/copilot-instructions.md").exists());
    }

    #[test]
    fn remove_deletes_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("CLAUDE.md");

        // Create file with only the directive block
        inject(dir.path(), "claude").unwrap();
        assert!(file.exists());

        // Remove should delete the file since it becomes empty
        let results = remove(dir.path()).unwrap();
        assert!(results[0].contains("deleted"));
        assert!(!file.exists());
    }
}
