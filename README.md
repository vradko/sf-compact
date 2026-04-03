# sf-compact

Cut Salesforce metadata tokens in half for AI coding agents.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000-50,000+ lines of XML with 70-85% structural overhead. Every time an AI agent reads your metadata, you're paying for tokens that carry no useful information.

**sf-compact** converts metadata XML to compact YAML/JSON. In controlled benchmarks (3 runs each, same task), compact files reduced Claude Code costs by **11.5%** — and by **33%** when combined with a custom exploration agent.

## Quick Start

```bash
npm install -g sf-compact-cli
sf-compact pack
sf-compact init instructions
```

That's it. `init instructions` injects a directive into your CLAUDE.md (or equivalent AI tool file) telling the AI to read from `.sf-compact/` instead of `force-app/`. In our benchmarks, this was the simplest approach that reliably worked.

## Why This Approach (Benchmarks)

We tested 17 different approaches over 6 weeks on a production Salesforce org (~10,000 files, 42MB). Most failed or made things worse. Here's what we learned:

**What works:**
- **CLAUDE.md directive** (`init instructions`): -11.5% cost, statistically verified across 3 runs. Sub-agents DO follow CLAUDE.md — an earlier assumption that they don't was wrong.
- **Custom sf-explorer agent** (`init agent`): -32.9% cost in a single-run test. Routes metadata reads through a Haiku-based agent optimized for `.sf-compact/`.

**What doesn't work:**
- **Mentioning sf-compact in the prompt** without setup: +22.9% cost. The AI wasted turns discovering the tool.
- **Hooks** (5 variants tested): -1% to +29% cost. Technically functional but unnecessary — CLAUDE.md already redirects reads, so hooks have nothing to intercept.
- **MCP tools alone**: -2.6% cost. The AI never voluntarily called the tools.

**Why YAML is the default format:**

Token savings vary dramatically by metadata type. JSON saves 31-34% on large types (profiles, objects, record types) but is actually *worse* for small types — fields (+0.4%), list views (+1.1%), compact layouts (+5.5%). The JSON `_children`/`_tag`/`_text` overhead exceeds the XML tag overhead on small files. YAML avoids this and works well across all types.

Full benchmark details: [Medium article](https://medium.com/@radko.volodymyr/cutting-salesforce-metadata-tokens-by-50-for-ai-and-why-that-wasnt-enough-ede7fbe18626)

## Output Formats

| Format | Preserves order | Human-readable | Best for |
|--------|:-:|:-:|----------|
| `yaml` | No | Yes | Most types (default) |
| `yaml-ordered` | Yes | Yes | Order-sensitive types (Flow, FlexiPage, Layout) |
| `json` | Yes | Less | Maximum savings on large types (profiles, objects) |

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
...
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
sf-compact pack                                   # pack force-app -> .sf-compact (default)
sf-compact pack force-app --format json            # JSON for max savings on large types
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
sf-compact config set default yaml                  # change default format
sf-compact config skip customMetadata               # exclude a type
sf-compact config show                              # view current config
```

Default config: `yaml` for all types, with `yaml-ordered` overrides for order-sensitive types (Flow, FlexiPage, Layout) to preserve element order.

### AI Tool Integration

```bash
sf-compact init instructions                        # inject directive into CLAUDE.md / .cursorrules / etc. (recommended)
sf-compact init agent                               # create sf-explorer agent for Claude Code (advanced, -33% cost)
sf-compact init hook                                # Claude Code PreToolUse hook (optional, see benchmarks)
sf-compact init mcp                                 # MCP server integration
```

**`init instructions`** (recommended) — injects a directive block into your AI tool's instruction file. Auto-detects which tools are configured. This is the simplest approach and sufficient for the -11.5% cost saving measured in benchmarks.

Supported targets: `claude`, `cursor`, `copilot`, `codex`, `windsurf`, `cline`, `aider`, `stdout`.

**`init agent`** — creates a `.claude/agents/sf-explorer.md` custom agent that reads metadata from `.sf-compact/`. In benchmarks, Opus + sf-explorer delegation showed -32.9% cost reduction. Add a CLAUDE.md directive to delegate metadata exploration to sf-explorer for best results.

**`init hook`** — installs a Claude Code PreToolUse hook. In our benchmarks, hooks were unnecessary because CLAUDE.md already redirected reads. Included for environments where CLAUDE.md is not available.

**`init mcp`** — exposes sf-compact tools via MCP. In benchmarks, the AI did not voluntarily use MCP tools without explicit instructions.

### Manifest

Output supported metadata types in JSON:

```bash
sf-compact manifest
```

## Workflow

1. **Configure** (once): `sf-compact config init`
2. **Pull metadata**: `sf project retrieve start`
3. **Pack**: `sf-compact pack`
4. **Setup AI**: `sf-compact init instructions` (once per project)
5. **Work** -- AI reads compact files via CLAUDE.md directive
6. **Unpack**: `sf-compact unpack` -- restores XML
7. **Deploy**: `sf project deploy start`

Use `sf-compact watch` during development to auto-pack on changes, and `sf-compact lint` in CI to ensure compact files stay in sync.

> Tip: Add `.sf-compact/` to `.gitignore` if you treat it as a build artifact, or commit it for AI-friendly diffs.

## What "Semantically Lossless" Means

The roundtrip preserves all data that Salesforce cares about:

- **Whitespace** -- leading/trailing whitespace in text nodes is trimmed
- **Comments** -- stripped (use `--preserve-comments` to keep them)
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
