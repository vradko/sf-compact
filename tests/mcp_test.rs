use std::io::Write;
use std::process::{Command, Stdio};

fn sf_compact() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sf-compact"))
}

/// Send JSON-RPC requests to mcp-serve and collect responses.
fn mcp_call(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut child = sf_compact()
        .arg("mcp-serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start mcp-serve");

    let stdin = child.stdin.as_mut().unwrap();
    for req in requests {
        let line = serde_json::to_string(req).unwrap();
        writeln!(stdin, "{line}").unwrap();
    }
    drop(child.stdin.take()); // close stdin to signal EOF

    let output = child
        .wait_with_output()
        .expect("failed to wait on mcp-serve");
    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{l}")))
        .collect()
}

fn jsonrpc(id: u64, method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

// ── initialize ─────────────────────────────────────────────────────

#[test]
fn mcp_initialize() {
    let responses = mcp_call(&[jsonrpc(1, "initialize", serde_json::json!({}))]);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    assert_eq!(r["id"], 1);
    assert!(r["result"]["protocolVersion"].is_string());
    assert!(r["result"]["serverInfo"]["name"].as_str().unwrap() == "sf-compact");
    assert!(r["result"]["serverInfo"]["version"].is_string());
    assert!(r["result"]["capabilities"]["tools"].is_object());
}

// ── tools/list ─────────────────────────────────────────────────────

#[test]
fn mcp_tools_list_has_all_tools() {
    let responses = mcp_call(&[jsonrpc(1, "tools/list", serde_json::json!({}))]);
    assert_eq!(responses.len(), 1);

    let tools = responses[0]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(
        names.contains(&"sf_compact_pack"),
        "missing sf_compact_pack: {names:?}"
    );
    assert!(
        names.contains(&"sf_compact_unpack"),
        "missing sf_compact_unpack: {names:?}"
    );
    assert!(
        names.contains(&"sf_compact_stats"),
        "missing sf_compact_stats: {names:?}"
    );
    assert!(
        names.contains(&"sf_compact_lint"),
        "missing sf_compact_lint: {names:?}"
    );
    assert!(
        names.contains(&"sf_compact_changes"),
        "missing sf_compact_changes: {names:?}"
    );

    // Each tool should have description and inputSchema
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(
            tool["description"].is_string(),
            "{name} missing description"
        );
        assert!(
            tool["inputSchema"]["type"].as_str() == Some("object"),
            "{name} missing inputSchema"
        );
    }
}

// ── tools/call: sf_compact_pack ────────────────────────────────────

#[test]
fn mcp_pack_tool() {
    let packed = tempfile::tempdir().unwrap();
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    assert_eq!(responses.len(), 1);
    let content = &responses[0]["result"]["content"][0]["text"];
    let text = content.as_str().unwrap();
    assert!(
        text.contains("Packed"),
        "pack output should mention Packed: {text}"
    );
    assert!(text.contains("5 files"), "should pack 5 files: {text}");
}

// ── tools/call: sf_compact_unpack ──────────────────────────────────

#[test]
fn mcp_unpack_tool() {
    let packed = tempfile::tempdir().unwrap();
    let unpacked = tempfile::tempdir().unwrap();

    // Pack first
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Unpack
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_unpack",
            "arguments": {
                "source": packed.path().to_str().unwrap(),
                "output": unpacked.path().to_str().unwrap()
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("Unpacked"), "unpack output: {text}");
    assert!(text.contains("5 files"), "should unpack 5 files: {text}");
}

// ── tools/call: sf_compact_stats ───────────────────────────────────

#[test]
fn mcp_stats_tool() {
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_stats",
            "arguments": {
                "source": "tests/fixtures"
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("Files: 5"), "stats output: {text}");
    assert!(
        text.contains("Token reduction"),
        "should have token stats: {text}"
    );
    assert!(
        text.contains("Tokens saved"),
        "should have tokens saved: {text}"
    );
}

// ── tools/call: sf_compact_lint ────────────────────────────────────

#[test]
fn mcp_lint_tool_up_to_date() {
    let packed = tempfile::tempdir().unwrap();

    // Pack first
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Lint — should be OK
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_lint",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("OK"),
        "lint should pass after fresh pack: {text}"
    );
    // isError should be false
    let is_error = responses[0]["result"]["isError"].as_bool().unwrap_or(false);
    assert!(!is_error, "lint should not be error when up-to-date");
}

// ── tools/call: sf_compact_changes ─────────────────────────────────

#[test]
fn mcp_changes_tool_empty_after_pack() {
    let packed = tempfile::tempdir().unwrap();

    // Pack
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Changes — should be empty
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_changes",
            "arguments": {
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("No compact files modified"),
        "should be empty after fresh pack: {text}"
    );
}

#[test]
fn mcp_changes_tool_detects_modified() {
    let packed = tempfile::tempdir().unwrap();

    // Pack
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Wait and modify a compact file
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let compact_file = find_first_compact_file(packed.path()).unwrap();
    let content = std::fs::read_to_string(&compact_file).unwrap();
    std::fs::write(&compact_file, format!("{content}\n")).unwrap();

    // Changes — should detect modification
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_changes",
            "arguments": {
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    // Should be JSON with global array and deploy commands
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|_| panic!("changes output should be JSON: {text}"));
    assert!(
        parsed["global"].is_array(),
        "should have global array: {text}"
    );
    assert!(
        !parsed["global"].as_array().unwrap().is_empty(),
        "should have changes: {text}"
    );
    assert!(
        parsed["deploy_command"].is_string(),
        "should have deploy_command: {text}"
    );
    assert!(
        parsed["retrieve_command"].is_string(),
        "should have retrieve_command: {text}"
    );
}

#[test]
fn mcp_changes_tool_reset() {
    let packed = tempfile::tempdir().unwrap();

    // Pack
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Reset global
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_changes",
            "arguments": {
                "output": packed.path().to_str().unwrap(),
                "reset": "global"
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("Reset global tracking"),
        "reset output: {text}"
    );
}

#[test]
fn mcp_changes_tool_since_deploy() {
    let packed = tempfile::tempdir().unwrap();

    // Pack
    mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap()
            }
        }),
    )]);

    // Changes with since_deploy
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_changes",
            "arguments": {
                "output": packed.path().to_str().unwrap(),
                "since_deploy": true
            }
        }),
    )]);

    let text = responses[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("No compact files modified since last deploy"),
        "should be empty since-deploy: {text}"
    );
}

// ── error handling ─────────────────────────────────────────────────

#[test]
fn mcp_unknown_tool_returns_error() {
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_nonexistent",
            "arguments": {}
        }),
    )]);

    assert!(
        responses[0]["error"].is_object(),
        "should return error for unknown tool"
    );
    let msg = responses[0]["error"]["message"].as_str().unwrap();
    assert!(msg.contains("Unknown tool"), "error message: {msg}");
}

#[test]
fn mcp_unknown_method_returns_error() {
    let responses = mcp_call(&[jsonrpc(1, "nonexistent/method", serde_json::json!({}))]);
    assert!(
        responses[0]["error"].is_object(),
        "should return error for unknown method"
    );
}

#[test]
fn mcp_invalid_json_returns_parse_error() {
    let mut child = sf_compact()
        .arg("mcp-serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start mcp-serve");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, "{{not valid json").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let resp: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(resp["error"]["code"], -32700, "should be parse error code");
}

#[test]
fn mcp_pack_invalid_format_returns_error() {
    let packed = tempfile::tempdir().unwrap();
    let responses = mcp_call(&[jsonrpc(
        1,
        "tools/call",
        serde_json::json!({
            "name": "sf_compact_pack",
            "arguments": {
                "source": "tests/fixtures",
                "output": packed.path().to_str().unwrap(),
                "format": "invalid-format"
            }
        }),
    )]);

    assert!(
        responses[0]["error"].is_object(),
        "should return error for invalid format"
    );
    let msg = responses[0]["error"]["message"].as_str().unwrap();
    assert!(msg.contains("Invalid format"), "error: {msg}");
}

// ── multiple requests in one session ───────────────────────────────

#[test]
fn mcp_multiple_requests() {
    let responses = mcp_call(&[
        jsonrpc(1, "initialize", serde_json::json!({})),
        jsonrpc(2, "tools/list", serde_json::json!({})),
    ]);

    assert_eq!(responses.len(), 2, "should get 2 responses");
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[1]["id"], 2);
    assert!(responses[0]["result"]["serverInfo"].is_object());
    assert!(responses[1]["result"]["tools"].is_array());
}

// ── generate_instructions includes changes ─────────────────────────

#[test]
fn init_instructions_includes_changes_command() {
    let dir = tempfile::tempdir().unwrap();
    let output_file = dir.path().join("SF_COMPACT.md");

    let output = sf_compact()
        .args([
            "init",
            "instructions",
            "--name",
            output_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init instructions failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("changes"),
        "instructions should mention changes command"
    );
    assert!(
        content.contains("--since-deploy"),
        "instructions should mention --since-deploy"
    );
    assert!(
        content.contains("reset"),
        "instructions should mention reset"
    );
}

// ── cursorrules includes changes ───────────────────────────────────

#[test]
fn init_cursorrules_includes_changes_command() {
    let dir = tempfile::tempdir().unwrap();

    let output = sf_compact()
        .current_dir(dir.path())
        .args(["init", "cursorrules"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let content = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert!(
        content.contains("changes"),
        "cursorrules should mention changes command"
    );
}

// ── helpers ────────────────────────────────────────────────────────

fn find_first_compact_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && {
                let name = e.file_name().to_string_lossy();
                (name.ends_with("-meta.json") || name.ends_with("-meta.yaml"))
                    && !e.path().components().any(|c| c.as_os_str() == ".tracking")
            }
        })
        .map(|e| e.path().to_path_buf())
        .next()
}
