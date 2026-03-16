use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::constants;
use crate::convert;
use crate::diff;
use crate::manifest;
use crate::tracking;

/// Run the MCP server over stdio (JSON-RPC 2.0, line-delimited).
pub fn serve() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.context("Failed to read from stdin")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                });
                write_response(&stdout, &err_resp)?;
                continue;
            }
        };

        // Notifications have no "id" field — ignore them
        let id = match msg.get("id") {
            Some(id) => id.clone(),
            None => continue,
        };

        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let result = handle_method(method, &params);

        let response = match result {
            Ok(res) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": res
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("{e}")
                }
            }),
        };

        write_response(&stdout, &response)?;
    }

    Ok(())
}

fn write_response(stdout: &io::Stdout, response: &Value) -> Result<()> {
    let mut out = stdout.lock();
    serde_json::to_writer(&mut out, response)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

fn handle_method(method: &str, params: &Value) -> Result<Value> {
    match method {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params),
        _ => Err(anyhow::anyhow!("Method not found: {method}")),
    }
}

fn handle_initialize() -> Result<Value> {
    Ok(json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "sf-compact",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

fn handle_tools_list() -> Result<Value> {
    Ok(json!({
        "tools": [
            {
                "name": "sf_compact_pack",
                "description": "Convert Salesforce metadata XML files to compact YAML or JSON format, reducing token consumption by 42-54% for AI tools.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing Salesforce metadata XML files (default: force-app)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output directory for compact files (default: .sf-compact)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["yaml", "yaml-ordered", "json"],
                            "description": "Output format: json (~54% savings, default, preserves order), yaml (~49%, human-readable), yaml-ordered (~42%, preserves order in YAML). Default: json or per-type config."
                        },
                        "include": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g. '*.profile-meta.xml')"
                        }
                    }
                }
            },
            {
                "name": "sf_compact_unpack",
                "description": "Convert compact YAML/JSON files back to Salesforce metadata XML (semantically lossless roundtrip).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing compact YAML/JSON files (default: .sf-compact)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output directory for restored XML files (default: force-app)"
                        },
                        "include": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g. '*.profile-meta.yaml')"
                        }
                    }
                }
            },
            {
                "name": "sf_compact_stats",
                "description": "Analyze Salesforce metadata and show token/byte savings from compaction.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing Salesforce metadata XML files (default: force-app)"
                        },
                        "include": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g. '*.profile-meta.xml')"
                        }
                    }
                }
            },
            {
                "name": "sf_compact_lint",
                "description": "Check that compact files are up-to-date with XML sources. Returns stale/new/orphaned files. Use for CI validation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing Salesforce metadata XML files (default: force-app)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Packed directory to compare against (default: .sf-compact)"
                        },
                        "include": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g. '*.profile-meta.xml')"
                        }
                    }
                }
            },
            {
                "name": "sf_compact_changes",
                "description": "Show which compact files have been modified since last pack. Tracks changes per git branch with global and deployment scopes. Use to know what to deploy or retrieve before commit.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "output": {
                            "type": "string",
                            "description": "Compact directory to check (default: .sf-compact)"
                        },
                        "since_deploy": {
                            "type": "boolean",
                            "description": "If true, show only changes since last deployment reset (default: false, shows all global changes)"
                        },
                        "reset": {
                            "type": "string",
                            "enum": ["global", "since-deploy"],
                            "description": "Reset tracking state: 'global' clears all tracking, 'since-deploy' clears only deployment tracking"
                        }
                    }
                }
            }
        ]
    }))
}

fn handle_tools_call(params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name in tools/call"))?;

    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "sf_compact_pack" => call_pack(&args),
        "sf_compact_unpack" => call_unpack(&args),
        "sf_compact_stats" => call_stats(&args),
        "sf_compact_lint" => call_lint(&args),
        "sf_compact_changes" => call_changes(&args),
        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    }
}

fn call_pack(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_SOURCE);
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_OUTPUT);
    let format = args
        .get("format")
        .and_then(|s| s.as_str())
        .map(String::from);
    let include = args
        .get("include")
        .and_then(|s| s.as_str())
        .map(String::from);

    if let Some(ref fmt) = format {
        if !constants::VALID_FORMATS.contains(&fmt.as_str()) {
            anyhow::bail!(
                "Invalid format '{}'. Valid formats: {}",
                fmt,
                constants::VALID_FORMATS.join(", ")
            );
        }
    }

    let output_path = Path::new(output);
    let opts = convert::ConvertOpts {
        paths: vec![Path::new(source).to_path_buf()],
        include,
        format_override: format,
        incremental: false,
        preserve_comments: None,
        indent: None,
    };
    let stats = convert::pack(&opts, output_path)?;
    tracking::record_pack_result(output_path, &stats);

    let text = format!(
        "Packed {} files: {} -> {} bytes ({:.1}% reduction)",
        stats.files_processed,
        stats.original_bytes,
        stats.compact_bytes,
        stats.reduction_percent(),
    );

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn call_unpack(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_OUTPUT);
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_SOURCE);
    let include = args
        .get("include")
        .and_then(|s| s.as_str())
        .map(String::from);

    let opts = convert::ConvertOpts {
        paths: vec![Path::new(source).to_path_buf()],
        include,
        format_override: None,
        incremental: false,
        preserve_comments: None,
        indent: None,
    };
    let stats = convert::unpack(&opts, Path::new(output))?;

    let text = format!("Unpacked {} files", stats.files_processed);

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn call_stats(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_SOURCE);
    let include = args
        .get("include")
        .and_then(|s| s.as_str())
        .map(String::from);

    let opts = convert::ConvertOpts {
        paths: vec![Path::new(source).to_path_buf()],
        include,
        format_override: None,
        incremental: false,
        preserve_comments: None,
        indent: None,
    };
    let stats = convert::stats(&opts)?;

    let mut text = format!(
        "Files: {}\nXML bytes: {}\nCompact bytes: {}\nByte reduction: {:.1}%\nXML tokens: {}\nCompact tokens: {}\nToken reduction: {:.1}%\nTokens saved: {}",
        stats.files_processed,
        stats.original_bytes,
        stats.compact_bytes,
        stats.reduction_percent(),
        stats.original_tokens,
        stats.compact_tokens,
        stats.token_reduction_percent(),
        stats.tokens_saved(),
    );

    for (meta_type, ts) in &stats.by_type {
        text.push_str(&format!(
            "\n  {}: {} files, {} -> {} tokens ({:.1}%)",
            meta_type,
            ts.count,
            ts.original_tokens,
            ts.compact_tokens,
            ts.token_reduction_percent(),
        ));
    }

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn call_lint(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_SOURCE);
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_OUTPUT);
    let include = args.get("include").and_then(|s| s.as_str());

    let sources = vec![Path::new(source).to_path_buf()];
    let result = diff::diff(&sources, Path::new(output), include)?;

    let total = result.new_files.len() + result.modified_files.len() + result.deleted_files.len();

    let mut text = String::new();
    if total == 0 {
        text.push_str(&format!("OK: {} files up to date", result.unchanged_files));
    } else {
        for f in &result.new_files {
            text.push_str(&format!("+ {f} (not packed)\n"));
        }
        for f in &result.modified_files {
            text.push_str(&format!("~ {f} (stale)\n"));
        }
        for f in &result.deleted_files {
            text.push_str(&format!("- {f} (orphaned)\n"));
        }
        text.push_str(&format!(
            "\n{} stale ({} new, {} modified, {} orphaned)",
            total,
            result.new_files.len(),
            result.modified_files.len(),
            result.deleted_files.len(),
        ));
    }

    Ok(json!({
        "content": [{ "type": "text", "text": text }],
        "isError": total > 0
    }))
}

fn call_changes(args: &Value) -> Result<Value> {
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or(constants::DEFAULT_OUTPUT);
    let output_path = Path::new(output);

    // Handle reset
    if let Some(reset) = args.get("reset").and_then(|s| s.as_str()) {
        let scope = match reset {
            "global" => tracking::ResetScope::Global,
            "since-deploy" => tracking::ResetScope::SinceDeploy,
            _ => anyhow::bail!("Invalid reset scope: {reset}. Use 'global' or 'since-deploy'"),
        };
        tracking::reset_tracking(output_path, scope)?;
        let label = if reset == "global" {
            "global"
        } else {
            "deployment"
        };
        let text = format!(
            "Reset {label} tracking for branch '{}'",
            tracking::get_current_branch()
        );
        return Ok(json!({
            "content": [{ "type": "text", "text": text }]
        }));
    }

    let since_deploy = args
        .get("since_deploy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let changes = tracking::detect_changes(output_path, since_deploy)?;

    if changes.is_empty() {
        let scope = if since_deploy {
            "since last deploy"
        } else {
            "globally"
        };
        let text = format!("No compact files modified {scope}.");
        return Ok(json!({
            "content": [{ "type": "text", "text": text }]
        }));
    }

    let scope_key = if since_deploy { "deployment" } else { "global" };
    let items: Vec<Value> = changes
        .iter()
        .map(|c| {
            json!({
                "xml_path": c.xml_relative_path,
                "compact_path": c.compact_path,
            })
        })
        .collect();

    let paths: Vec<&str> = changes
        .iter()
        .map(|c| c.xml_relative_path.as_str())
        .collect();

    let result = json!({
        scope_key: items,
        "deploy_command": format!("sf project deploy start -d {}", paths.join(" -d ")),
        "retrieve_command": format!("sf project retrieve start -d {}", paths.join(" -d ")),
    });

    let text = serde_json::to_string_pretty(&result)?;

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

/// Generate the AI instructions markdown content.
pub fn generate_instructions() -> String {
    let manifest = manifest::build_manifest();

    let mut types_list = String::new();
    for entry in &manifest.supported_metadata {
        types_list.push_str(&format!(
            "- **{}** (`{}`) — {}\n",
            entry.meta_type, entry.extension, entry.category
        ));
    }

    format!(
        r#"# sf-compact: Salesforce Metadata Compaction for AI

## What is sf-compact?

sf-compact converts Salesforce metadata XML files into compact YAML or JSON formats that are semantically equivalent but use significantly fewer tokens (42–54% reduction). This reduces cost and improves performance when AI tools read or analyze Salesforce metadata.

The conversion is **semantically lossless for Salesforce metadata** — you can always convert back to XML. Normalizations: element order may change (use `yaml-ordered` or `json` to preserve), leading/trailing whitespace in text nodes is trimmed, XML comments and CDATA sections are normalized, empty elements may change form (`<tag></tag>` ↔ `<tag/>`). All safe for Salesforce metadata.

## Install

```bash
npm install -g sf-compact-cli   # npm (downloads pre-built binary)
brew install vradko/tap/sf-compact  # Homebrew (macOS/Linux)
cargo install sf-compact            # From source (Rust required)
```

## Output Formats

- **json** (default) — compact single-line, ~54% savings, preserves element order
- **yaml** — grouped arrays, ~49% savings, best for order-insensitive types (Profile, PermissionSet)
- **yaml-ordered** — `_children` sequences, ~42% savings, preserves order in YAML format

## Available Commands

### Pack (XML to compact format)
```bash
sf-compact pack [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern] [--incremental]
```
- `source` — one or more directories or files (default: `force-app`)
- `-o output` — output directory (default: `.sf-compact`)
- `--format` — output format override (default: from config or `json`)
- `--include` — glob filter (e.g. `"*.profile-meta.xml"`)
- `--incremental` — only repack files modified since last pack (by mtime)

### Unpack (compact format to XML)
```bash
sf-compact unpack [source...] [-o output] [--include pattern]
```
Auto-detects format by file extension (`.yaml` or `.json`).

### Stats
```bash
sf-compact stats [source...] [--include pattern] [--files]
```
Preview token/byte savings without writing files.

### Configuration
```bash
sf-compact config init                              # create .sfcompact.yaml with smart defaults
sf-compact config set flow json profile yaml        # batch set formats per type
sf-compact config set default json                  # change default format
sf-compact config skip customMetadata               # exclude a type
sf-compact config show                              # display config
```

### Watch (auto-pack on changes)
```bash
sf-compact watch [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern]
```
Watches source directories and auto-repacks when XML files change.

### Diff (detect unpacked changes)
```bash
sf-compact diff [source...] [-o packed-dir] [--include pattern]
```
Shows which XML files have changed since last pack (new, modified, deleted).

### Lint (CI validation)
```bash
sf-compact lint [source...] [-o packed-dir] [--include pattern]
```
Checks that compact files are up-to-date. Exits with code 1 if any files are stale, new, or orphaned. Use in CI pipelines to enforce that compact files stay in sync.

### Changes (track modified compact files)
```bash
sf-compact changes [-o compact-dir]                    # show all modified files (global)
sf-compact changes --since-deploy                      # show changes since last deploy reset
sf-compact changes --json                              # machine-readable JSON output
sf-compact changes reset --global                      # clear all tracking
sf-compact changes reset --since-deploy                # clear deployment tracking only
```
Tracks which compact files were modified (by AI or human) since last `pack`. Per-branch tracking with two scopes: global (all changes) and deployment (delta since last deploy reset). Use to know what to deploy or retrieve before commit.

### Other Commands
- `sf-compact manifest` — output supported metadata types in JSON
- `sf-compact mcp-serve` — start MCP server over stdio (exposes pack, unpack, stats, lint, changes as MCP tools)
- `sf-compact init mcp` — create/update .mcp.json
- `sf-compact init instructions` — generate AI instructions markdown

## Workflow

1. **Configure** (once): `sf-compact config init`
2. **Run `sf-compact pack`** after pulling metadata from Salesforce.
3. **Work with compact files** in `.sf-compact/` — let AI tools read/edit them.
4. **Run `sf-compact unpack` before deploy** to restore XML.
5. **Add `.sf-compact/` to `.gitignore`** if you prefer to treat it as a build artifact.

## Supported Metadata Types

{types_list}"#
    )
}
