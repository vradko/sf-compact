use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const HOOKS_DIR: &str = ".claude/hooks";
const SCRIPT_NAME: &str = "sf-compact-read.sh";
const SETTINGS_PATH: &str = ".claude/settings.json";

/// Identifier used to detect sf-compact entries in settings.json.
const HOOK_IDENTIFIER: &str = "sf-compact-read";

/// Hook matcher for the PreToolUse event.
const HOOK_MATCHER: &str = "Read";

/// Timeout in seconds for the hook execution.
const HOOK_TIMEOUT: u64 = 5;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install the Claude Code PreToolUse hook that redirects XML reads to compact files.
pub fn install(source: &str, output: &str) -> Result<()> {
    let script_path = write_script(source, output)?;
    update_settings_add(&script_path)?;

    println!(
        "\nWhen AI reads a file from {source}/, the hook will redirect to {output}/ \
         if a compact version exists."
    );

    Ok(())
}

/// Remove the Claude Code hook script and its settings.json entry.
pub fn remove() -> Result<()> {
    let script_path = PathBuf::from(HOOKS_DIR).join(SCRIPT_NAME);
    let settings_path = PathBuf::from(SETTINGS_PATH);

    if script_path.exists() {
        fs::remove_file(&script_path)?;
        println!("Removed {}", script_path.display());
    }

    if settings_path.exists() {
        remove_hook_from_settings(&settings_path)?;
        println!("Removed sf-compact hook from {}", settings_path.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Script generation
// ---------------------------------------------------------------------------

fn write_script(source: &str, output: &str) -> Result<PathBuf> {
    let hooks_dir = PathBuf::from(HOOKS_DIR);
    let script_path = hooks_dir.join(SCRIPT_NAME);

    fs::create_dir_all(&hooks_dir)?;
    fs::write(&script_path, generate_script(source, output))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("Created {}", script_path.display());
    Ok(script_path)
}

fn generate_script(source: &str, output: &str) -> String {
    format!(
        r##"#!/bin/sh
# sf-compact Read hook: redirects Salesforce XML reads to compact equivalents.
# Installed by: sf-compact init hook
# No external dependencies — pure POSIX shell.

set -eu

INPUT=$(cat)

# Extract file_path from JSON input (lightweight — no jq dependency)
FILE_PATH=$(printf '%s' "$INPUT" | sed -n 's/.*"file_path" *: *"\([^"]*\)".*/\1/p')

# Exit early if no file path
[ -z "$FILE_PATH" ] && exit 0

# Only intercept *-meta.xml files under the source directory
case "$FILE_PATH" in
  */{source}/*-meta.xml | {source}/*-meta.xml) ;;
  *) exit 0 ;;
esac

# Compute compact path: replace source prefix with output prefix, swap extension
RELATIVE="${{FILE_PATH#*{source}/}}"

# Resolve prefix: if input is absolute, make compact path absolute too
PREFIX="{output}"
case "$FILE_PATH" in
  /*) PREFIX="${{FILE_PATH%%{source}/*}}{output}" ;;
esac

# Try YAML first (default format), then JSON
for EXT in yaml json; do
  COMPACT="$PREFIX/${{RELATIVE%-meta.xml}}-meta.$EXT"
  if [ -f "$COMPACT" ]; then
    printf '{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","updatedInput":{{"file_path":"%s"}},"additionalContext":"Reading compact version instead of XML. Original: %s"}}}}\n' "$COMPACT" "$FILE_PATH"
    exit 0
  fi
done

# No compact file found — let the original read proceed
exit 0
"##
    )
}

// ---------------------------------------------------------------------------
// Settings.json manipulation
// ---------------------------------------------------------------------------

fn update_settings_add(script_path: &Path) -> Result<()> {
    let settings_path = PathBuf::from(SETTINGS_PATH);
    let mut settings = load_settings(&settings_path)?;

    let arr = ensure_pre_tool_use_array(&mut settings)?;

    // Remove existing sf-compact entry (idempotent reinstall)
    retain_non_sfcompact(arr);

    // Add new entry
    let command = script_path.to_string_lossy().to_string();
    arr.push(serde_json::json!({
        "matcher": HOOK_MATCHER,
        "hooks": [{
            "type": "command",
            "command": command,
            "timeout": HOOK_TIMEOUT
        }]
    }));

    save_settings(&settings_path, &settings)?;
    println!(
        "Updated {} with PreToolUse Read hook",
        settings_path.display()
    );

    Ok(())
}

fn remove_hook_from_settings(settings_path: &Path) -> Result<()> {
    let content = fs::read_to_string(settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(hooks) = settings.get_mut("hooks") {
        if let Some(pre) = hooks.get_mut("PreToolUse") {
            if let Some(arr) = pre.as_array_mut() {
                retain_non_sfcompact(arr);
            }
        }
    }

    save_settings(settings_path, &settings)
}

fn load_settings(path: &Path) -> Result<serde_json::Value> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).context("Failed to parse .claude/settings.json")
    } else {
        Ok(serde_json::json!({}))
    }
}

fn save_settings(path: &Path, settings: &serde_json::Value) -> Result<()> {
    let json = serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn ensure_pre_tool_use_array(
    settings: &mut serde_json::Value,
) -> Result<&mut Vec<serde_json::Value>> {
    let hooks = settings
        .as_object_mut()
        .context("Expected settings to be a JSON object")?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let pre_tool_use = hooks
        .as_object_mut()
        .context("Expected hooks to be a JSON object")?
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]));

    pre_tool_use
        .as_array_mut()
        .context("Expected PreToolUse to be an array")
}

/// Retain only entries that are NOT sf-compact hooks.
fn retain_non_sfcompact(arr: &mut Vec<serde_json::Value>) {
    arr.retain(|entry| {
        entry
            .pointer("/hooks/0/command")
            .and_then(|c| c.as_str())
            .is_none_or(|c| !c.contains(HOOK_IDENTIFIER))
    });
}
