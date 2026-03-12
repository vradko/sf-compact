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
#[command(about = "Convert Salesforce metadata XML to AI-friendly YAML and back. Lossless roundtrip.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert Salesforce XML metadata to compact YAML
    Pack {
        /// Source directory containing Salesforce metadata XML files
        #[arg(default_value = "force-app")]
        source: PathBuf,

        /// Output directory for compact YAML files
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,
    },
    /// Convert compact YAML back to Salesforce XML metadata (lossless)
    Unpack {
        /// Source directory containing compact YAML files
        #[arg(default_value = ".sf-compact")]
        source: PathBuf,

        /// Output directory for restored XML files
        #[arg(short, long, default_value = "force-app")]
        output: PathBuf,
    },
    /// Show stats: how many tokens/bytes saved
    Stats {
        /// Source directory containing Salesforce metadata XML files
        #[arg(default_value = "force-app")]
        source: PathBuf,
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
        Commands::Pack { source, output } => {
            let stats = convert::pack(&source, &output)?;
            println!(
                "Packed {} files: {} → {} bytes ({:.1}% reduction, ~{} tokens saved)",
                stats.files_processed,
                stats.original_bytes,
                stats.compact_bytes,
                stats.reduction_percent(),
                stats.tokens_saved(),
            );
        }
        Commands::Unpack { source, output } => {
            let stats = convert::unpack(&source, &output)?;
            println!("Unpacked {} files", stats.files_processed);
        }
        Commands::Stats { source } => {
            let stats = convert::stats(&source)?;
            println!("Salesforce metadata analysis (tokenizer: cl100k_base):");
            println!();
            println!("  Files:             {}", stats.files_processed);
            println!("  XML bytes:         {}", stats.original_bytes);
            println!("  YAML bytes:        {}", stats.compact_bytes);
            println!("  Byte reduction:    {:.1}%", stats.reduction_percent());
            println!();
            println!("  XML tokens:        {}", stats.original_tokens);
            println!("  YAML tokens:       {}", stats.compact_tokens);
            println!("  Token reduction:   {:.1}%", stats.token_reduction_percent());
            println!("  Tokens saved:      {}", stats.tokens_saved());
            println!();
            println!("  {:<20} {:>5} files  {:>8} → {:>8} tokens  ({:>5.1}% tokens)  {:>8} → {:>8} bytes",
                "Type", "", "XML", "YAML", "", "XML", "YAML");
            println!("  {}", "-".repeat(100));
            for (meta_type, ts) in &stats.by_type {
                println!(
                    "  {:<20} {:>5} files  {:>8} → {:>8} tokens  ({:>5.1}%)        {:>8} → {:>8} bytes",
                    meta_type,
                    ts.count,
                    ts.original_tokens,
                    ts.compact_tokens,
                    ts.token_reduction_percent(),
                    ts.original_bytes,
                    ts.compact_bytes,
                );
            }
        }
        Commands::McpServe => {
            mcp::serve()?;
        }
        Commands::Manifest => {
            let m = manifest::build_manifest();
            let json = serde_json::to_string_pretty(&m)
                .context("Failed to serialize manifest")?;
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
        let content = fs::read_to_string(&path)
            .context("Failed to read existing .mcp.json")?;
        serde_json::from_str(&content)
            .context("Failed to parse existing .mcp.json")?
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

    let json = serde_json::to_string_pretty(&root)
        .context("Failed to serialize .mcp.json")?;
    fs::write(&path, format!("{json}\n"))
        .context("Failed to write .mcp.json")?;

    println!("Created/updated .mcp.json with sf-compact MCP server entry");
    Ok(())
}

fn init_instructions(name: &str) -> Result<()> {
    let content = mcp::generate_instructions();
    let path = PathBuf::from(name);

    fs::write(&path, &content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!("Created {} with AI instructions", path.display());
    Ok(())
}
