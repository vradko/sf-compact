mod convert;
mod manifest;
mod mcp;
mod xml_parser;
mod yaml_writer;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sf-compact")]
#[command(
    about = "Convert Salesforce metadata XML to AI-friendly YAML and back. Lossless roundtrip."
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert Salesforce XML metadata to compact YAML
    Pack {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Output directory for compact YAML files
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,

        /// Only include files matching this glob pattern (e.g. "profiles/**", "*.profile-meta.xml")
        #[arg(long)]
        include: Option<String>,
    },
    /// Convert compact YAML back to Salesforce XML metadata (lossless)
    Unpack {
        /// Source path: directory or specific file(s)
        #[arg(default_value = ".sf-compact")]
        source: Vec<PathBuf>,

        /// Output directory for restored XML files
        #[arg(short, long, default_value = "force-app")]
        output: PathBuf,

        /// Only include files matching this glob pattern
        #[arg(long)]
        include: Option<String>,
    },
    /// Preview token/byte savings without writing files
    Stats {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Only include files matching this glob pattern
        #[arg(long)]
        include: Option<String>,

        /// Show per-file breakdown
        #[arg(long)]
        files: bool,
    },
    /// Start MCP (Model Context Protocol) server over stdio
    McpServe,
    /// Output supported Salesforce metadata types in JSON format
    Manifest,
    /// Initialize sf-compact configuration
    Init {
        #[command(subcommand)]
        mode: InitMode,
    },
}

#[derive(Subcommand)]
enum InitMode {
    /// Create/update .mcp.json for MCP tool integration
    Mcp,
    /// Create AI instructions markdown file
    Instructions {
        /// Output filename for the instructions
        #[arg(long, default_value = "SF_COMPACT.md")]
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            source,
            output,
            include,
        } => {
            let opts = convert::ConvertOpts {
                paths: source,
                include,
            };
            let stats = convert::pack(&opts, &output)?;
            println!(
                "Packed {} files: {} → {} bytes ({:.1}% reduction, ~{} tokens saved)",
                stats.files_processed,
                stats.original_bytes,
                stats.compact_bytes,
                stats.reduction_percent(),
                stats.tokens_saved(),
            );
        }
        Commands::Unpack {
            source,
            output,
            include,
        } => {
            let opts = convert::ConvertOpts {
                paths: source,
                include,
            };
            let stats = convert::unpack(&opts, &output)?;
            println!("Unpacked {} files", stats.files_processed);
        }
        Commands::Stats {
            source,
            include,
            files: show_files,
        } => {
            let opts = convert::ConvertOpts {
                paths: source,
                include,
            };
            let stats = convert::stats(&opts)?;

            println!("Preview: what sf-compact pack would produce");
            println!("Tokenizer: cl100k_base (GPT-4 / Claude)");
            println!();
            println!(
                "  {:>40}    {:>10}    {:>10}    {:>8}",
                "", "XML (now)", "YAML (after)", "savings"
            );
            println!("  {}", "-".repeat(80));
            println!(
                "  {:>40}    {:>10}    {:>10}    {:>7.1}%",
                "Bytes",
                stats.original_bytes,
                stats.compact_bytes,
                stats.reduction_percent()
            );
            println!(
                "  {:>40}    {:>10}    {:>10}    {:>7.1}%",
                "Tokens",
                stats.original_tokens,
                stats.compact_tokens,
                stats.token_reduction_percent()
            );
            println!();
            println!(
                "  Would save {} tokens across {} files",
                stats.tokens_saved(),
                stats.files_processed
            );
            println!();

            if !stats.by_type.is_empty() {
                println!("  By metadata type:");
                println!(
                    "  {:<20} {:>5}    {:>8} → {:>8} tokens    {:>6}",
                    "type", "files", "now", "after", "saved"
                );
                println!("  {}", "-".repeat(70));
                for (meta_type, ts) in &stats.by_type {
                    println!(
                        "  {:<20} {:>5}    {:>8} → {:>8} tokens    {:>5.1}%",
                        meta_type,
                        ts.count,
                        ts.original_tokens,
                        ts.compact_tokens,
                        ts.token_reduction_percent(),
                    );
                }
                println!();
            }

            if show_files {
                println!("  Per file:");
                println!("  {:<60} {:>8} → {:>8} tokens", "file", "now", "after");
                println!("  {}", "-".repeat(90));
                for fi in &stats.per_file {
                    println!(
                        "  {:<60} {:>8} → {:>8} tokens",
                        fi.relative_path, fi.original_tokens, fi.compact_tokens
                    );
                }
            }
        }
        Commands::McpServe => {
            mcp::serve()?;
        }
        Commands::Manifest => {
            let m = manifest::build_manifest();
            let json = serde_json::to_string_pretty(&m).context("Failed to serialize manifest")?;
            println!("{json}");
        }
        Commands::Init { mode } => match mode {
            InitMode::Mcp => {
                init_mcp()?;
            }
            InitMode::Instructions { name } => {
                init_instructions(&name)?;
            }
        },
    }

    Ok(())
}

fn init_mcp() -> Result<()> {
    let path = PathBuf::from(".mcp.json");

    let mut root: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(&path).context("Failed to read existing .mcp.json")?;
        serde_json::from_str(&content).context("Failed to parse existing .mcp.json")?
    } else {
        serde_json::json!({})
    };

    let servers = root
        .as_object_mut()
        .context("Expected .mcp.json to be a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let servers_map = servers
        .as_object_mut()
        .context("Expected mcpServers to be a JSON object")?;

    servers_map.insert(
        "sf-compact".to_string(),
        serde_json::json!({
            "type": "stdio",
            "command": "sf-compact",
            "args": ["mcp-serve"]
        }),
    );

    let json = serde_json::to_string_pretty(&root).context("Failed to serialize .mcp.json")?;
    fs::write(&path, format!("{json}\n")).context("Failed to write .mcp.json")?;

    println!("Created/updated .mcp.json with sf-compact MCP server entry");
    Ok(())
}

fn init_instructions(name: &str) -> Result<()> {
    let content = mcp::generate_instructions();
    let path = PathBuf::from(name);

    fs::write(&path, &content).with_context(|| format!("Failed to write {}", path.display()))?;

    println!("Created {} with AI instructions", path.display());
    Ok(())
}
