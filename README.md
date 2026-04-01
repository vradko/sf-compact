# sf-compact

Cut Salesforce metadata tokens in half for AI coding agents. Transparent, lossless, zero-config.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000-50,000+ lines of XML with 70-85% structural overhead. Every time an AI agent reads your metadata, you're paying for tokens that carry no useful information.

**sf-compact** solves this at the infrastructure level: a Claude Code hook transparently intercepts metadata reads and serves compact versions instead. The AI never sees the XML. No prompts, no instructions, no behavior changes needed.

## Quick Start

```bash
# Install
npm install -g sf-compact-cli

# Pack your metadata into compact format
sf-compact pack

# Install the Claude Code hook
sf-compact init hook
```

That's it. From now on, when Claude Code reads any `*-meta.xml` file from `force-app/`, the hook silently redirects to the compact version in `.sf-compact/`. The AI doesn't know or care — it just reads fewer tokens.

## How the Hook Works

```
AI: "Read force-app/main/default/profiles/Admin.profile-meta.xml"
              │
              ▼
    ┌─────────────────────┐
    │  PreToolUse hook     │
    │  sf-compact-read.sh  │
    └─────────┬───────────┘
              │  .sf-compact/main/default/profiles/Admin.profile-meta.json exists?
              │  Yes → redirect
              ▼
AI receives: .sf-compact/.../Admin.profile-meta.json (50% fewer tokens)
```

The hook is a Claude Code `PreToolUse` interceptor on the `Read` tool. It:

1. Checks if the requested file is a Salesforce `*-meta.xml` under `force-app/`
2. Looks for a compact equivalent in `.sf-compact/` (tries `.json` first, then `.yaml`)
3. If found, rewrites the file path — the AI reads the compact version transparently
4. If no compact version exists, the original read proceeds unchanged

### Hook Commands

```bash
# Install hook (creates .claude/hooks/sf-compact-read.sh + updates .claude/settings.json)
sf-compact init hook

# Install with custom paths
sf-compact init hook --source src/metadata --output .compact

# Remove hook
sf-compact init hook --remove
```

## Output Formats

| Format | Preserves order | Human-readable | Token savings | Default |
|--------|:-:|:-:|:-:|:-:|
| `json` | Yes | Less | ~54% | **Default** |
| `yaml` | No | Yes | ~49% | Order-insensitive types |
| `yaml-ordered` | Yes | Yes | ~42% | -- |

- **json** (default) — compact single-line JSON. Preserves element order, fewest tokens.
- **yaml** — groups repeated elements into arrays. More readable, but sibling order may change. Used for order-insensitive types (Profile, PermissionSet).
- **yaml-ordered** — uses `_children` sequences to preserve exact element order in YAML.

## Before / After

**XML (848 tokens):**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<Profile xmlns="http://soap.sforce.com/2006/04/metadata">
    <custom>false</custom>
    <userLicense>Salesforce</userLicense>
    <fieldPermissions>
        <editable>true</editable>
        <field>Account.AnnualRevenue</field>
        <readable>true</readable>
    </fieldPermissions>
    <fieldPermissions>
        <editable>false</editable>
        <field>Account.BillingCity</field>
        <readable>true</readable>
    </fieldPermissions>
    ...
</Profile>
```

**YAML (432 tokens -- 49% reduction):**
```yaml
_tag: Profile
_ns: http://soap.sforce.com/2006/04/metadata
custom: false
userLicense: Salesforce
fieldPermissions:
- editable: true
  field: Account.AnnualRevenue
  readable: true
- editable: false
  field: Account.BillingCity
  readable: true
...
```

**JSON (389 tokens -- 54% reduction):**
```json
{"_tag":"Profile","_ns":"http://soap.sforce.com/2006/04/metadata","custom":"false","userLicense":"Salesforce","fieldPermissions":[{"editable":"true","field":"Account.AnnualRevenue","readable":"true"},{"editable":"false","field":"Account.BillingCity","readable":"true"}]}
```

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

### From source
```bash
cargo install --path .
```

## Commands

### Pack (XML -> compact format)
```bash
sf-compact pack [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern] [--incremental]
```

```bash
# Pack entire project (default: force-app -> .sf-compact)
sf-compact pack

# Pack as JSON for maximum token savings
sf-compact pack force-app --format json

# Incremental: only repack files modified since last pack
sf-compact pack --incremental

# Pack only profiles
sf-compact pack force-app --include "*.profile-meta.xml"
```

### Unpack (compact format -> XML)
```bash
sf-compact unpack [source...] [-o output] [--include pattern]
```

Auto-detects format by file extension (`.yaml` or `.json`).

```bash
sf-compact unpack .sf-compact -o force-app
```

### Watch (auto-pack on changes)
```bash
sf-compact watch [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern]
```

Watches source directories for XML changes and automatically repacks.

```bash
sf-compact watch
```

### Stats (preview savings)
```bash
sf-compact stats [source...] [--include pattern] [--files]
```

```bash
$ sf-compact stats force-app

Preview: what sf-compact pack would produce
Tokenizer: cl100k_base (GPT-4 / Claude)

                                           XML (now)    YAML (after)     savings
  --------------------------------------------------------------------------------
                                 Bytes          7313          3418       53.3%
                                Tokens          1719           925       46.2%

  Would save 794 tokens across 5 files
```

### Diff (detect unpacked changes)
```bash
sf-compact diff [source...] [-o packed-dir] [--include pattern]
```

Compare current XML metadata against the last packed output. Shows new, modified, and deleted files.

### Lint (CI validation)
```bash
sf-compact lint [source...] [-o packed-dir] [--include pattern]
```

Check that compact files are up-to-date. Exits with code 1 if stale. Use in CI pipelines.

### Changes (track modified compact files)
```bash
sf-compact changes [-o compact-dir]             # all modified files (global)
sf-compact changes --since-deploy               # changes since last deploy reset
sf-compact changes --json                       # machine-readable JSON output
sf-compact changes reset --global               # clear all tracking
sf-compact changes reset --since-deploy         # clear deployment tracking only
```

Tracks which compact files were modified since last `pack`. Per-branch tracking with two scopes:
- **Global** -- all files changed since tracking started. For final retrieve before commit.
- **Deployment** -- delta since last deploy reset. For deploying only what changed.

### Configuration
```bash
sf-compact config init                                    # create .sfcompact.yaml with smart defaults
sf-compact config set flow json profile yaml              # set format per type
sf-compact config set default json                        # change default format
sf-compact config skip customMetadata                     # exclude a type
sf-compact config show                                    # view current config
```

### AI Instructions

Inject sf-compact directives into AI tool instruction files. Auto-detects which tools are configured.

```bash
sf-compact init instructions                              # auto-detect and inject into all
sf-compact init instructions --target claude              # CLAUDE.md only
sf-compact init instructions --target cursor              # .cursorrules only
sf-compact init instructions --remove                     # remove from all files
```

Supported targets: `claude`, `cursor`, `copilot`, `codex`, `windsurf`, `cline`, `aider`, `stdout`.

### MCP Server

Built-in [MCP](https://modelcontextprotocol.io/) server for direct AI tool integration.

```bash
sf-compact init mcp        # add to .mcp.json
sf-compact mcp-serve       # start manually
```

Exposes `sf_compact_pack`, `sf_compact_unpack`, `sf_compact_stats`, `sf_compact_lint`, and `sf_compact_changes` as MCP tools.

### Manifest

Output supported metadata types in JSON:

```bash
sf-compact manifest
```

## Workflow

1. **Configure** (once): `sf-compact config init`
2. **Pull metadata**: `sf project retrieve start`
3. **Pack**: `sf-compact pack`
4. **Install hook**: `sf-compact init hook` (once)
5. **Work** -- AI reads compact files transparently, edits them directly
6. **Unpack**: `sf-compact unpack` -- restores XML
7. **Deploy**: `sf project deploy start`

Use `sf-compact watch` during development to auto-pack on changes, and `sf-compact lint` in CI to ensure compact files stay in sync.

> Tip: Add `.sf-compact/` to `.gitignore` if you treat it as a build artifact, or commit it for AI-friendly diffs.

## What "Semantically Lossless" Means

The roundtrip preserves all data that Salesforce cares about:

- **Whitespace** -- leading/trailing whitespace in text nodes is trimmed
- **Comments** -- stripped (Salesforce metadata doesn't use comments; use `--preserve-comments` to keep them)
- **CDATA** -- unwrapped to escaped text (`&lt;`, `&amp;`)
- **Empty elements** -- `<tag></tag>` may become `<tag/>`
- **Element order** -- may change with `yaml` format; use `yaml-ordered` or `json` to preserve

## Supported Metadata Types

76 file extensions mapping to Salesforce metadata types across 10 categories:

| Category | Types |
|----------|-------|
| **Security** | Profile, PermissionSet, PermissionSetGroup, RemoteSiteSetting, CspTrustedSite, ConnectedApp, SharingRules, CustomPermission, Role, Group, AuthProvider, SamlSsoConfig, Certificate |
| **Schema** | CustomObject, CustomField, ValidationRule, CustomMetadata, GlobalValueSet, StandardValueSet, RecordType, MatchingRule, DuplicateRule, CustomIndex, FieldSet |
| **Code** | ApexClass, ApexTrigger, ApexComponent, ApexPage, LightningComponentBundle (js/css/html/xml), AuraDefinitionBundle (cmp/evt), StaticResource |
| **Automation** | Flow*, Workflow, WorkflowRule, AssignmentRules, AutoResponseRules, EscalationRules |
| **UI** | Layout*, CustomLabels, CustomApplication, CustomTab, FlexiPage*, CustomSite, QuickAction, PathAssistant, ListView, CompactLayout, WebLink, HomePageLayout, AppMenu, Community, Letterhead |
| **Analytics** | ReportType, Report, Dashboard |
| **Integration** | ExternalServiceRegistration, NamedCredential, ExternalCredential |
| **Config** | Settings, InstalledPackage, TopicsForObjects, CustomNotificationType, CleanDataService, NotificationTypeConfig, PlatformEventChannelMember |
| **Translation** | CustomObjectTranslation, CustomFieldTranslation |
| **Content** | EmailTemplate, ManagedContentType, IframeWhiteListUrlSettings, LightningMessageChannel |

\* Order-sensitive types -- `config init` defaults these to `yaml-ordered` to preserve element order.

## How It Works

- Parses Salesforce metadata XML into a tree structure
- Groups repeated elements (e.g., `<fieldPermissions>`) into arrays
- Coerces booleans: `"true"` -> `true`, `"false"` -> `false`. All other values preserved as-is
- Flattens simple key-value containers into inline mappings
- Preserves namespaces, attributes, and all structural information
- Order-sensitive types default to `yaml-ordered` format with `_children` sequences

Token counting uses the `cl100k_base` tokenizer (same family used by GPT-4 and Claude).

## License

MIT
