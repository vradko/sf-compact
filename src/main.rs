mod config;
mod constants;
mod convert;
mod diff;
mod json_writer;
mod manifest;
mod mcp;
mod metadata_types;
mod tracking;
mod watch;
mod xml_parser;
mod yaml_writer;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sf-compact")]
#[command(
    about = "Convert Salesforce metadata XML to AI-friendly YAML and back. Semantically lossless roundtrip."
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Number of parallel threads (default: number of CPU cores)
    #[arg(long, short = 'j', global = true)]
    jobs: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert Salesforce XML metadata to compact YAML or JSON
    Pack {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Output directory for compact files
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,

        /// Only include files matching this glob pattern (e.g. "profiles/**", "*.profile-meta.xml")
        #[arg(long)]
        include: Option<String>,

        /// Output format override (yaml, yaml-ordered, or json). Takes precedence over config.
        #[arg(long)]
        format: Option<String>,

        /// Only repack files modified since last pack (compare mtimes)
        #[arg(long)]
        incremental: bool,

        /// Preserve XML comments through roundtrip (overrides config)
        #[arg(long)]
        preserve_comments: bool,
    },
    /// Convert compact YAML back to Salesforce XML metadata (semantically lossless)
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

        /// XML output indentation spaces (overrides config, default: 4)
        #[arg(long)]
        indent: Option<u8>,
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
    /// Watch source directories and auto-pack on XML changes
    Watch {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Output directory for compact files
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,

        /// Only include files matching this glob pattern
        #[arg(long)]
        include: Option<String>,

        /// Output format override (yaml, yaml-ordered, or json)
        #[arg(long)]
        format: Option<String>,
    },
    /// Show which XML files changed since last pack
    Diff {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Packed directory to compare against
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,

        /// Only include files matching this glob pattern
        #[arg(long)]
        include: Option<String>,
    },
    /// Check that compact files are up-to-date (exit 1 if stale). Use in CI pipelines.
    Lint {
        /// Source path: directory or specific file(s)
        #[arg(default_value = "force-app")]
        source: Vec<PathBuf>,

        /// Packed directory to compare against
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,

        /// Only include files matching this glob pattern
        #[arg(long)]
        include: Option<String>,
    },
    /// Manage .sfcompact.yaml configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Show tracked file changes (compact files modified since last pack)
    Changes {
        #[command(subcommand)]
        action: Option<ChangesAction>,

        /// Show only changes since last deploy reset
        #[arg(long)]
        since_deploy: bool,

        /// Output as JSON for machine consumption
        #[arg(long)]
        json: bool,

        /// Compact directory to check
        #[arg(short, long, default_value = ".sf-compact")]
        output: PathBuf,
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
    /// Generate .cursorrules / .windsurfrules for AI editors
    Cursorrules,
}

#[derive(Subcommand)]
enum ChangesAction {
    /// Reset tracking state
    Reset {
        /// Clear all global tracking
        #[arg(long)]
        global: bool,

        /// Clear deployment tracking only
        #[arg(long, alias = "since-deploy")]
        since_deploy: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Create .sfcompact.yaml with smart defaults
    Init,
    /// Set format for metadata types (e.g. `config set flow json profile yaml`) or set default format (`config set default json`)
    Set {
        /// Key-value pairs: type1 format1 type2 format2 ... or "default" format
        #[arg(required = true, num_args = 2..)]
        pairs: Vec<String>,
    },
    /// Add a metadata type to the skip list
    Skip {
        /// Metadata type name to skip
        #[arg(required = true)]
        type_name: String,
    },
    /// Display current configuration
    Show,
}

fn validate_format(format: &Option<String>) -> Result<()> {
    if let Some(fmt) = format {
        if !constants::VALID_FORMATS.contains(&fmt.as_str()) {
            anyhow::bail!(
                "Invalid format '{}'. Valid formats: {}",
                fmt,
                constants::VALID_FORMATS.join(", ")
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(jobs) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
            .ok(); // ignore if already set
    }

    match cli.command {
        Commands::Pack {
            source,
            output,
            include,
            format,
            incremental,
            preserve_comments,
        } => {
            validate_format(&format)?;
            let mut opts = convert::ConvertOpts {
                paths: source,
                include,
                format_override: format,
                incremental,
                preserve_comments: None,
                indent: None,
            };
            if preserve_comments {
                opts.preserve_comments = Some(true);
            }
            let stats = convert::pack(&opts, &output)?;
            tracking::record_pack_result(&output, &stats);
            println!(
                "Packed {} files: {} → {} bytes ({:.1}% reduction)",
                stats.files_processed,
                stats.original_bytes,
                stats.compact_bytes,
                stats.reduction_percent(),
            );
            println!("Run `sf-compact stats` for detailed token savings.");
        }
        Commands::Unpack {
            source,
            output,
            include,
            indent,
        } => {
            let opts = convert::ConvertOpts {
                paths: source,
                include,
                format_override: None,
                incremental: false,
                preserve_comments: None,
                indent,
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
                format_override: None,
                incremental: false,
                preserve_comments: None,
                indent: None,
            };
            let stats = convert::stats(&opts)?;

            println!("Preview: what sf-compact pack would produce");
            println!("Tokenizer: cl100k_base (GPT-4 / Claude)");
            println!();
            println!(
                "  {:>40}    {:>10}    {:>10}    {:>8}",
                "", "XML (now)", "compact (after)", "savings"
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
        Commands::Watch {
            source,
            output,
            include,
            format,
        } => {
            validate_format(&format)?;
            watch::watch(&source, &output, include, format)?;
        }
        Commands::Diff {
            source,
            output,
            include,
        } => {
            let result = diff::diff(&source, &output, include.as_deref())?;

            let total_changes =
                result.new_files.len() + result.modified_files.len() + result.deleted_files.len();

            if total_changes == 0 {
                println!(
                    "No changes detected ({} files up to date)",
                    result.unchanged_files
                );
            } else {
                for f in &result.new_files {
                    println!("  + {f}  (new — not yet packed)");
                }
                for f in &result.modified_files {
                    println!("  ~ {f}  (modified since last pack)");
                }
                for f in &result.deleted_files {
                    println!("  - {f}  (packed file has no source XML)");
                }
                println!();
                println!(
                    "{} new, {} modified, {} deleted, {} unchanged",
                    result.new_files.len(),
                    result.modified_files.len(),
                    result.deleted_files.len(),
                    result.unchanged_files,
                );
                println!("Run `sf-compact pack` to update.");
            }
        }
        Commands::Lint {
            source,
            output,
            include,
        } => {
            let result = diff::diff(&source, &output, include.as_deref())?;
            let total_changes =
                result.new_files.len() + result.modified_files.len() + result.deleted_files.len();

            if total_changes == 0 {
                println!("OK: {} files up to date", result.unchanged_files);
            } else {
                for f in &result.new_files {
                    eprintln!("  + {f}  (not packed)");
                }
                for f in &result.modified_files {
                    eprintln!("  ~ {f}  (stale)");
                }
                for f in &result.deleted_files {
                    eprintln!("  - {f}  (orphaned)");
                }
                eprintln!(
                    "\n{} stale ({} new, {} modified, {} orphaned)",
                    total_changes,
                    result.new_files.len(),
                    result.modified_files.len(),
                    result.deleted_files.len(),
                );
                std::process::exit(1);
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
            InitMode::Cursorrules => {
                init_cursorrules()?;
            }
        },
        Commands::Changes {
            action,
            since_deploy,
            json,
            output,
        } => {
            if let Some(ChangesAction::Reset {
                global,
                since_deploy,
            }) = action
            {
                if !global && !since_deploy {
                    anyhow::bail!("Specify --global or --since-deploy to reset");
                }
                let scope = if global {
                    tracking::ResetScope::Global
                } else {
                    tracking::ResetScope::SinceDeploy
                };
                tracking::reset_tracking(&output, scope)?;
                let label = if global { "global" } else { "deployment" };
                println!(
                    "Reset {label} tracking for branch '{}'",
                    tracking::get_current_branch()
                );
            } else {
                let changes = tracking::detect_changes(&output, since_deploy)?;

                if json {
                    let items: Vec<serde_json::Value> = changes
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "xml_path": c.xml_relative_path,
                                "compact_path": c.compact_path,
                            })
                        })
                        .collect();

                    let out = if since_deploy {
                        serde_json::json!({ "deployment": items })
                    } else {
                        serde_json::json!({ "global": items })
                    };
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else if changes.is_empty() {
                    let scope = if since_deploy {
                        "since last deploy"
                    } else {
                        "globally"
                    };
                    println!("No compact files modified {scope}.");
                } else {
                    let scope = if since_deploy {
                        "since last deploy"
                    } else {
                        "globally"
                    };
                    println!("{} file(s) modified {scope}:\n", changes.len());
                    for c in &changes {
                        println!("  M {}", c.xml_relative_path);
                    }

                    // Suggest deploy/retrieve commands
                    let paths: Vec<&str> = changes
                        .iter()
                        .map(|c| c.xml_relative_path.as_str())
                        .collect();
                    println!("\nTo deploy changes:");
                    println!("  sf project deploy start -d {}", paths.join(" -d "));
                    println!("\nTo retrieve canonical XML before commit:");
                    println!("  sf project retrieve start -d {}", paths.join(" -d "));
                }
            }
        }
        Commands::Config { action } => match action {
            ConfigAction::Init => {
                config_init()?;
            }
            ConfigAction::Set { pairs } => {
                config_set(&pairs)?;
            }
            ConfigAction::Skip { type_name } => {
                config_skip(&type_name)?;
            }
            ConfigAction::Show => {
                config_show()?;
            }
        },
    }

    Ok(())
}

fn config_init() -> Result<()> {
    let entries = metadata_types::metadata_info_for_config();
    let cfg = config::SfCompactConfig::with_smart_defaults(&entries);
    let path = config::save_config_to_dir(&cfg, &std::env::current_dir()?)?;
    println!("Created {}", path.display());
    Ok(())
}

fn config_set(pairs: &[String]) -> Result<()> {
    if !pairs.len().is_multiple_of(2) {
        anyhow::bail!(
            "Expected key-value pairs (even number of arguments), got {}",
            pairs.len()
        );
    }

    let cwd = std::env::current_dir()?;
    let config_path = config::find_config_file(&cwd).unwrap_or_else(|| cwd.join(".sfcompact.yaml"));

    let mut cfg = if config_path.exists() {
        config::load_config_from(&config_path)?
    } else {
        config::SfCompactConfig::default()
    };

    for chunk in pairs.chunks(2) {
        let key = &chunk[0];
        let value = &chunk[1];

        // Validate key is not empty
        if key.is_empty() {
            anyhow::bail!("Type name cannot be empty");
        }

        // Special keys bypass format validation
        if key == "preserve_comments" || key == "indent" {
            // handled below
        } else if !constants::VALID_FORMATS.contains(&value.as_str()) {
            anyhow::bail!(
                "Invalid format '{}' for '{}'. Valid formats: {}",
                value,
                key,
                constants::VALID_FORMATS.join(", ")
            );
        }

        if key == "default" {
            cfg.default_format = value.clone();
        } else if key == "preserve_comments" {
            cfg.preserve_comments = value == "true";
        } else if key == "indent" {
            cfg.indent = value.parse::<u8>().map_err(|_| {
                anyhow::anyhow!("Invalid indent value '{}', expected a number", value)
            })?;
        } else {
            cfg.formats.insert(key.clone(), value.clone());
        }
    }

    config::save_config(&cfg, &config_path)?;
    println!("Updated {}", config_path.display());
    Ok(())
}

fn config_skip(type_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let config_path = config::find_config_file(&cwd).unwrap_or_else(|| cwd.join(".sfcompact.yaml"));

    let mut cfg = if config_path.exists() {
        config::load_config_from(&config_path)?
    } else {
        config::SfCompactConfig::default()
    };

    if type_name.is_empty() {
        anyhow::bail!("Type name cannot be empty");
    }

    if !cfg.skip.contains(&type_name.to_string()) {
        cfg.skip.push(type_name.to_string());
    }

    config::save_config(&cfg, &config_path)?;
    println!("Updated {}", config_path.display());
    Ok(())
}

fn config_show() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load_config(&cwd)?;
    let yaml = serde_yaml::to_string(&cfg).context("Failed to serialize config")?;
    print!("{yaml}");
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

fn init_cursorrules() -> Result<()> {
    let content = generate_cursorrules();
    let files = [".cursorrules", ".windsurfrules"];
    for name in &files {
        let path = PathBuf::from(name);
        fs::write(&path, &content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        println!("Created {}", path.display());
    }
    Ok(())
}

fn generate_cursorrules() -> String {
    r#"# Salesforce Metadata — sf-compact rules

## Compact files

This project uses sf-compact to convert Salesforce metadata XML into compact YAML/JSON.
The compact files live in `.sf-compact/` and are 42-54% smaller in tokens.

**When reading Salesforce metadata, prefer `.sf-compact/` over `force-app/` XML files.**
The compact format is semantically identical but uses far fewer tokens.

## File mapping

- `.sf-compact/**/*-meta.yaml` → compact YAML (grouped arrays)
- `.sf-compact/**/*-meta.json` → compact JSON (preserves order)
- `force-app/**/*-meta.xml` → original Salesforce XML

## Editing workflow

1. **Read** from `.sf-compact/` (saves tokens)
2. **Edit** the compact YAML/JSON directly
3. **Run `sf-compact unpack`** to restore XML before deployment
4. Or: edit XML in `force-app/`, then run `sf-compact pack` to update compact files

## Format reference

Compact files use these conventions:
- `_tag` — root element name (e.g., "Profile", "Flow")
- `_ns` — XML namespace
- `_attrs` — XML attributes (rare in SF metadata)
- `_children` — ordered child elements (yaml-ordered/json format)
- Repeated elements become arrays (yaml format)
- Booleans: `true`/`false` (not strings)

## Commands

```bash
sf-compact pack [source] [-o output] [--format yaml|yaml-ordered|json]
sf-compact unpack [source] [-o output]
sf-compact stats [source]    # preview token savings
sf-compact diff              # check what changed since last pack
sf-compact lint              # CI check — exits 1 if stale
sf-compact changes           # show compact files modified since pack
sf-compact changes --json    # machine-readable for AI
sf-compact changes reset --since-deploy  # clear deploy tracking
```

## Important

- Always run `sf-compact unpack` before `sf project deploy`
- The conversion is semantically lossless for Salesforce metadata
- When in doubt, check the XML in `force-app/` — it's the source of truth
"#
    .to_string()
}

fn init_instructions(name: &str) -> Result<()> {
    let content = mcp::generate_instructions();
    let path = PathBuf::from(name);

    fs::write(&path, &content).with_context(|| format!("Failed to write {}", path.display()))?;

    println!("Created {} with AI instructions", path.display());
    Ok(())
}
