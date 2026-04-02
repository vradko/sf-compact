use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const AGENTS_DIR: &str = ".claude/agents";
const AGENT_FILENAME: &str = "sf-explorer.md";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create the sf-explorer agent file for Claude Code.
pub fn install() -> Result<()> {
    let agents_dir = PathBuf::from(AGENTS_DIR);
    let agent_path = agents_dir.join(AGENT_FILENAME);

    fs::create_dir_all(&agents_dir)?;
    fs::write(&agent_path, generate_agent())?;

    println!("Created {}", agent_path.display());
    println!(
        "\nTo use: add a CLAUDE.md directive to delegate metadata exploration to sf-explorer,\n\
         or run Claude Code with: claude --agent sf-explorer"
    );

    Ok(())
}

/// Remove the sf-explorer agent file.
pub fn remove() -> Result<()> {
    let agent_path = Path::new(AGENTS_DIR).join(AGENT_FILENAME);

    if agent_path.exists() {
        fs::remove_file(&agent_path)?;
        println!("Removed {}", agent_path.display());
    } else {
        println!("No sf-explorer agent found.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Agent content
// ---------------------------------------------------------------------------

fn generate_agent() -> String {
    format!(
        "\
---
name: sf-explorer
model: haiku
description: Salesforce-aware file explorer that reads compact metadata from .sf-compact/
tools: Read, Glob, Grep, Bash
---
You are a fast codebase exploration agent for Salesforce projects.

IMPORTANT: When reading Salesforce metadata (objects, fields, profiles, permission sets, \
flows, layouts, etc.), ALWAYS read from .sf-compact/ directory instead of force-app/. \
The .sf-compact/ files are compact equivalents with identical information. \
Path mapping: replace force-app/ with .sf-compact/ and -meta.xml with -meta.yaml \
(or -meta.json if .yaml does not exist).

For Apex code (.cls, .trigger) and LWC/Aura (.js, .html, .css), read directly from force-app/.
"
    )
}
