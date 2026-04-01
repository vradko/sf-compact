# sf-compact

Cut Salesforce metadata tokens in half for AI coding agents. Transparent, lossless, zero-config.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000-50,000+ lines of XML with 70-85% structural overhead. Every time an AI agent reads your metadata, you're paying for tokens that carry no useful information.

**sf-compact** solves this at the infrastructure level: a Claude Code hook transparently intercepts metadata reads and serves compact versions instead. The AI never sees the XML. No prompts, no instructions, no behavior changes needed.

## Quick Start

```bash
npm install -g sf-compact-cli
sf-compact pack
sf-compact init hook
```

That's it. From now on, when Claude Code reads any `*-meta.xml` file from `force-app/`, the hook silently redirects to the compact version in `.sf-compact/`. The AI doesn't know or care — it just reads fewer tokens.

## How the Hook Works

The hook is a Claude Code `PreToolUse` interceptor on the `Read` tool:

1. Checks if the requested file is a Salesforce `*-meta.xml` under `force-app/`
2. Looks for a compact equivalent in `.sf-compact/` (tries `.json` first, then `.yaml`)
3. If found, rewrites the file path — the AI reads the compact version transparently
4. If no compact version exists, the original read proceeds unchanged

```bash
sf-compact init hook                                      # install
sf-compact init hook --source src/metadata --output .compact  # custom paths
sf-compact init hook --remove                             # uninstall
```

## Output Formats

| Format | Preserves order | Human-readable | Token savings | Default |
|--------|:-:|:-:|:-:|:-:|
| `json` | Yes | Less | ~54% | **Default** |
| `yaml` | No | Yes | ~49% | Order-insensitive types |
| `yaml-ordered` | Yes | Yes | ~42% | -- |

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

### Pack (XML -> compact format)
```bash
sf-compact pack [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern] [--incremental]
```

```bash
sf-compact pack                                   # pack force-app -> .sf-compact (default)
sf-compact pack force-app --format json            # JSON for max token savings
sf-compact pack --incremental                      # only repack modified files
sf-compact pack force-app --include "*.profile-meta.xml"  # filter by pattern
```

### Unpack (compact format -> XML)
```bash
sf-compact unpack .sf-compact -o force-app
```

### Watch (auto-pack on changes)
```bash
sf-compact watch
```

### Stats (preview savings)
```bash
sf-compact stats force-app
sf-compact stats force-app --files    # per-file breakdown
```

### Diff / Lint / Changes
```bash
sf-compact diff                                     # detect unpacked changes
sf-compact lint                                     # CI validation (exit 1 if stale)
sf-compact changes                                  # track modified compact files
sf-compact changes --since-deploy                   # delta since last deploy reset
sf-compact changes reset --since-deploy             # reset deployment tracking
```

### Configuration
```bash
sf-compact config init                              # create .sfcompact.yaml with smart defaults
sf-compact config set flow json profile yaml        # set format per type
sf-compact config set default json                  # change default format
sf-compact config skip customMetadata               # exclude a type
sf-compact config show                              # view current config
```

Default config: `json` for all types, with `yaml` overrides for order-insensitive types (Profile, PermissionSet, etc.) for better readability.

### AI Tool Integration
```bash
sf-compact init hook                                # Claude Code hook (recommended)
sf-compact init instructions                        # inject directives into AI instruction files
sf-compact init mcp                                 # MCP server integration
```

## Workflow

1. **Configure** (once): `sf-compact config init`
2. **Pull metadata**: `sf project retrieve start`
3. **Pack**: `sf-compact pack`
4. **Install hook**: `sf-compact init hook` (once)
5. **Work** -- AI reads compact files transparently, edits them directly
6. **Unpack**: `sf-compact unpack` -- restores XML
7. **Deploy**: `sf project deploy start`

Full documentation: [github.com/vradko/sf-compact](https://github.com/vradko/sf-compact)

## License

MIT
