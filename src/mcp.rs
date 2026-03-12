use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::convert;
use crate::manifest;

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

        let method = msg
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

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
                "description": "Convert Salesforce metadata XML files to compact YAML format, reducing token consumption for AI tools.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing Salesforce metadata XML files (default: force-app)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output directory for compact YAML files (default: .sf-compact)"
                        }
                    }
                }
            },
            {
                "name": "sf_compact_unpack",
                "description": "Convert compact YAML files back to Salesforce metadata XML (lossless roundtrip).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "Source directory containing compact YAML files (default: .sf-compact)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output directory for restored XML files (default: force-app)"
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
        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    }
}

fn call_pack(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or("force-app");
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or(".sf-compact");

    let stats = convert::pack_path(Path::new(source), Path::new(output))?;

    let text = format!(
        "Packed {} files: {} -> {} bytes ({:.1}% reduction, ~{} tokens saved)",
        stats.files_processed,
        stats.original_bytes,
        stats.compact_bytes,
        stats.reduction_percent(),
        stats.tokens_saved(),
    );

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn call_unpack(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or(".sf-compact");
    let output = args
        .get("output")
        .and_then(|s| s.as_str())
        .unwrap_or("force-app");

    let stats = convert::unpack_path(Path::new(source), Path::new(output))?;

    let text = format!("Unpacked {} files", stats.files_processed);

    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
}

fn call_stats(args: &Value) -> Result<Value> {
    let source = args
        .get("source")
        .and_then(|s| s.as_str())
        .unwrap_or("force-app");

    let stats = convert::stats_path(Path::new(source))?;

    let mut text = format!(
        "Files: {}\nXML bytes: {}\nYAML bytes: {}\nByte reduction: {:.1}%\nXML tokens: {}\nYAML tokens: {}\nToken reduction: {:.1}%\nTokens saved: {}",
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

sf-compact converts Salesforce metadata XML files into a compact YAML format that is semantically equivalent but uses significantly fewer tokens. This reduces cost and improves performance when AI tools read or analyze Salesforce metadata.

The conversion is **lossless** — you can always convert back to the original XML format.

## Available Commands

### Pack (XML to YAML)
```bash
sf-compact pack [source] [-o output]
```
Convert Salesforce metadata XML to compact YAML.
- `source` — directory with XML files (default: `force-app`)
- `-o output` — output directory (default: `.sf-compact`)

**Example:**
```bash
sf-compact pack force-app -o .sf-compact
```

### Unpack (YAML to XML)
```bash
sf-compact unpack [source] [-o output]
```
Convert compact YAML back to Salesforce metadata XML.
- `source` — directory with YAML files (default: `.sf-compact`)
- `-o output` — output directory (default: `force-app`)

**Example:**
```bash
sf-compact unpack .sf-compact -o force-app
```

### Stats
```bash
sf-compact stats [source]
```
Analyze metadata and show token/byte savings.

**Example:**
```bash
sf-compact stats force-app
```

### Manifest
```bash
sf-compact manifest
```
Output supported metadata types in JSON format.

### MCP Server
```bash
sf-compact mcp-serve
```
Start the MCP (Model Context Protocol) server over stdio for tool integration.

## Workflow Instructions

1. **Always work with YAML files** in `.sf-compact/` for reading and editing metadata.
2. **Run `sf-compact pack`** after pulling metadata from Salesforce to create compact versions.
3. **Edit the YAML files** — they are the working copies.
4. **Run `sf-compact unpack` before deploy** to convert YAML back to XML that Salesforce CLI expects.
5. **Add `.sf-compact/` to `.gitignore`** if you prefer to treat it as a build artifact, or commit it for AI-friendly diffs.

## Supported Metadata Types

{types_list}"#
    )
}
