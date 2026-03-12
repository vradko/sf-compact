mod convert;
mod xml_parser;
mod yaml_writer;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
            println!("Salesforce metadata analysis:");
            println!("  Files:           {}", stats.files_processed);
            println!("  XML bytes:       {}", stats.original_bytes);
            println!("  Compact bytes:   {}", stats.compact_bytes);
            println!(
                "  Reduction:       {:.1}%",
                stats.reduction_percent()
            );
            println!("  Tokens saved:    ~{}", stats.tokens_saved());
            println!();
            for (meta_type, type_stats) in &stats.by_type {
                println!(
                    "  {:<25} {:>6} files  {:>10} → {:>10} bytes  ({:.1}%)",
                    meta_type,
                    type_stats.count,
                    type_stats.original_bytes,
                    type_stats.compact_bytes,
                    type_stats.reduction_percent(),
                );
            }
        }
    }

    Ok(())
}
