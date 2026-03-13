use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::convert;

/// Watch source directories for XML changes and auto-pack.
pub fn watch(
    sources: &[PathBuf],
    output: &Path,
    include: Option<String>,
    format_override: Option<String>,
) -> Result<()> {
    // Validate paths exist
    for p in sources {
        if !p.exists() {
            anyhow::bail!("Path not found: {}", p.display());
        }
    }

    // Initial pack
    let opts = convert::ConvertOpts {
        paths: sources.to_vec(),
        include: include.clone(),
        format_override: format_override.clone(),
        incremental: false,
    };
    let stats = convert::pack(&opts, output)?;
    eprintln!(
        "Initial pack: {} files, {} → {} bytes ({:.1}% reduction)",
        stats.files_processed,
        stats.original_bytes,
        stats.compact_bytes,
        stats.reduction_percent(),
    );

    let (tx, rx) = mpsc::channel();

    let mut debouncer =
        new_debouncer(Duration::from_millis(500), tx).context("Failed to create file watcher")?;

    for source in sources {
        debouncer
            .watcher()
            .watch(source, notify::RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch {}", source.display()))?;
    }

    eprintln!(
        "Watching {} for changes... (Ctrl+C to stop)",
        sources
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Check if any event is relevant (SF metadata XML files)
                let has_xml_change = events.iter().any(|e| {
                    e.kind == DebouncedEventKind::Any
                        && e.path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.ends_with("-meta.xml") || n.ends_with(".xml"))
                });

                if has_xml_change {
                    let opts = convert::ConvertOpts {
                        paths: sources.to_vec(),
                        include: include.clone(),
                        format_override: format_override.clone(),
                        incremental: false,
                    };
                    match convert::pack(&opts, output) {
                        Ok(stats) => {
                            eprintln!(
                                "Repacked {} files ({:.1}% reduction)",
                                stats.files_processed,
                                stats.reduction_percent(),
                            );
                        }
                        Err(e) => {
                            eprintln!("Pack error: {e}");
                        }
                    }
                }
            }
            Ok(Err(err)) => {
                eprintln!("Watch error: {err}");
            }
            Err(e) => {
                anyhow::bail!("Watch channel closed: {e}");
            }
        }
    }
}
