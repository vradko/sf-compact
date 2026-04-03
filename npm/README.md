# sf-compact

Cut Salesforce metadata tokens in half for AI coding agents.

Salesforce metadata XML is extremely verbose — 70-85% structural overhead. **sf-compact** converts it to compact YAML/JSON. In controlled benchmarks, this reduced Claude Code costs by **11.5%** — and by **33%** with a custom exploration agent.

## Quick Start

```bash
npm install -g sf-compact-cli
sf-compact pack
sf-compact init instructions
```

`init instructions` injects a directive into your CLAUDE.md telling the AI to read from `.sf-compact/` instead of `force-app/`. In 17 benchmark variants, this was the simplest approach that reliably worked.

## Why YAML is the Default

Token savings vary by metadata type. JSON saves 31-34% on large types (profiles, objects) but is *worse* for small types — fields (+0.4%), list views (+1.1%), compact layouts (+5.5%). YAML avoids the JSON `_children` overhead and works well across all sizes.

## Install

### npm (recommended)
```bash
npm install -g sf-compact-cli
```

### Homebrew (macOS / Linux)
```bash
brew install vradko/tap/sf-compact
```

### From crates.io
```bash
cargo install sf-compact
```

## Commands

```bash
sf-compact pack                                     # XML -> compact (force-app -> .sf-compact)
sf-compact unpack .sf-compact -o force-app          # compact -> XML
sf-compact watch                                    # auto-pack on changes
sf-compact stats force-app                          # preview savings
sf-compact diff                                     # detect unpacked changes
sf-compact lint                                     # CI validation (exit 1 if stale)
sf-compact changes --since-deploy                   # track modified files
```

### Configuration
```bash
sf-compact config init                              # create .sfcompact.yaml with smart defaults
sf-compact config set flow json profile yaml        # set format per type
sf-compact config show                              # view current config
```

### AI Tool Integration
```bash
sf-compact init instructions                        # CLAUDE.md / .cursorrules / etc. (recommended)
sf-compact init agent                               # sf-explorer agent for Claude Code (-33% cost)
sf-compact init hook                                # PreToolUse hook (optional)
sf-compact init mcp                                 # MCP server
```

## Workflow

1. `sf-compact config init` (once)
2. `sf project retrieve start`
3. `sf-compact pack`
4. `sf-compact init instructions` (once)
5. AI reads compact files via CLAUDE.md directive
6. `sf-compact unpack` -> `sf project deploy start`

Full documentation: [github.com/vradko/sf-compact](https://github.com/vradko/sf-compact)

## License

MIT
