# sf-compact

Convert Salesforce metadata XML to AI-friendly compact formats. Semantically lossless roundtrip.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000–50,000+ lines of XML with 70–85% structural overhead. This burns tokens and money when AI tools (Claude Code, Codex, Cursor, etc.) read or edit your metadata.

**sf-compact** converts it to compact YAML or JSON, saving 42–54% of tokens depending on format.

## Output Formats

| Format | Preserves order | Human-readable | Token savings |
|--------|:-:|:-:|:-:|
| `yaml` | No | Yes | ~49% |
| `yaml-ordered` | Yes | Yes | ~42% |
| `json` | Yes | Less | ~54% |

- **yaml** — groups repeated elements into arrays. Most compact YAML, but sibling order may change. Best for order-insensitive types (Profile, PermissionSet).
- **yaml-ordered** — uses `_children` sequences to preserve exact element order. Best for order-sensitive types (Flow, FlexiPage, Layout).
- **json** — compact single-line JSON with arrays. Preserves order, fewest tokens, less human-readable.

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

**YAML (432 tokens — 49% reduction):**
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

**JSON (389 tokens — 54% reduction):**
```json
{"_tag":"Profile","_ns":"http://soap.sforce.com/2006/04/metadata","custom":"false","userLicense":"Salesforce","fieldPermissions":[{"editable":"true","field":"Account.AnnualRevenue","readable":"true"},{"editable":"false","field":"Account.BillingCity","readable":"true"}]}
```

## Install

### From source (Rust required)
```bash
cargo install --path .
```

### From crates.io (coming soon)
```bash
cargo install sf-compact
```

## Usage

### Pack (XML → compact format)
```bash
sf-compact pack [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern]
```

```bash
# Pack entire project (default: YAML format)
sf-compact pack force-app -o .sf-compact

# Pack as JSON for maximum token savings
sf-compact pack force-app --format json

# Pack specific directories
sf-compact pack force-app/main/default/profiles force-app/main/default/classes

# Pack only profiles
sf-compact pack force-app --include "*.profile-meta.xml"
```

### Unpack (compact format → XML)
```bash
sf-compact unpack [source...] [-o output] [--include pattern]
```

Auto-detects format by file extension (`.yaml` or `.json`).

```bash
sf-compact unpack .sf-compact -o force-app
```

### Stats (preview savings)
```bash
sf-compact stats [source...] [--include pattern] [--files]
```

Analyze metadata and preview token/byte savings without writing files.

```bash
$ sf-compact stats force-app

Preview: what sf-compact pack would produce
Tokenizer: cl100k_base (GPT-4 / Claude)

                                               XML (now)    YAML (after)     savings
  --------------------------------------------------------------------------------
                                     Bytes          7313          3418       53.3%
                                    Tokens          1719           925       46.2%

  Would save 794 tokens across 5 files

  By metadata type:
  type                 files         now →    after tokens     saved
  ----------------------------------------------------------------------
  profile                  1         848 →      432 tokens     49.1%
  flow                     1         464 →      268 tokens     42.2%
  field                    1         232 →      126 tokens     45.7%
  js                       1         116 →       66 tokens     43.1%
  cls                      1          59 →       33 tokens     44.1%
```

Use `--files` for per-file breakdown, `--include` to filter by glob pattern.

### Configuration

sf-compact uses a `.sfcompact.yaml` config file for per-type format control.

```bash
# Create config with smart defaults (yaml-ordered for order-sensitive types)
sf-compact config init

# Set format for specific types (batch — multiple types in one call)
sf-compact config set flow json profile yaml flexipage yaml-ordered

# Change default format for all types
sf-compact config set default json

# Skip a metadata type from conversion
sf-compact config skip customMetadata

# View current configuration
sf-compact config show
```

Default config after `config init`:

```yaml
default_format: yaml
formats:
  Flow: yaml-ordered
  FlexiPage: yaml-ordered
  Layout: yaml-ordered
skip: []
```

When `pack` runs, it reads `.sfcompact.yaml` and applies the format per metadata type. The `--format` CLI flag overrides the config for a single run.

### MCP Server

sf-compact includes a built-in [MCP](https://modelcontextprotocol.io/) server for direct AI tool integration.

```bash
# Add to your project's .mcp.json
sf-compact init mcp

# Or start manually
sf-compact mcp-serve
```

This exposes `sf_compact_pack`, `sf_compact_unpack`, and `sf_compact_stats` as MCP tools that Claude Code, Cursor, and other MCP-compatible tools can discover and use automatically.

### AI Instructions

Generate a provider-agnostic markdown file with usage instructions for any AI tool:

```bash
sf-compact init instructions
sf-compact init instructions --name SALESFORCE.md
```

### Manifest

Output supported metadata types in JSON (includes format support and order-sensitivity flags):

```bash
sf-compact manifest
```

## Supported Metadata Types

44 file extensions mapping to 41 unique Salesforce metadata types across 7 categories:

| Category | Types |
|----------|-------|
| **Security** | Profile, PermissionSet, PermissionSetGroup, RemoteSiteSetting, CspTrustedSite, ConnectedApp, SharingRules |
| **Schema** | CustomObject, CustomField, ValidationRule, CustomMetadata, GlobalValueSet, StandardValueSet, RecordType, MatchingRule, DuplicateRule |
| **Code** | ApexClass, ApexTrigger, ApexComponent, ApexPage, LightningComponentBundle (js/css/html/xml) |
| **Automation** | Flow*, Workflow, AssignmentRules, AutoResponseRules, EscalationRules |
| **UI** | Layout*, CustomLabels, CustomApplication, CustomTab, FlexiPage*, CustomSite, QuickAction, PathAssistant, ListView, CompactLayout, WebLink |
| **Analytics** | ReportType, Report, Dashboard |
| **Content** | EmailTemplate |

\* Order-sensitive types — `config init` defaults these to `yaml-ordered` to preserve element order.

## Workflow

1. **Configure** (once): `sf-compact config init` — creates `.sfcompact.yaml` with smart defaults
2. **Pull metadata** from Salesforce (`sf project retrieve`)
3. **Pack**: `sf-compact pack` — creates `.sf-compact/` with compact files
4. **Work with compact files** — let AI tools read/edit the YAML/JSON format
5. **Unpack**: `sf-compact unpack` — restores XML for deployment
6. **Deploy** to Salesforce (`sf project deploy`)

> Tip: Add `.sf-compact/` to `.gitignore` if you treat it as a build artifact, or commit it for AI-friendly diffs.

## How it works

- Parses Salesforce metadata XML into a tree structure
- Groups repeated elements (e.g., `<fieldPermissions>`) into arrays (YAML) or `_children` sequences (yaml-ordered, JSON)
- Coerces booleans: `"true"` → `true`, `"false"` → `false`. All other values (including numeric strings like `"59.0"`, `"0012"`) are preserved as-is
- Flattens simple key-value containers into inline mappings
- Preserves namespaces, attributes, and all structural information for semantically lossless roundtrip
- Order-sensitive types (Flow, FlexiPage, Layout) default to `yaml-ordered` format, which preserves exact element order via `_children` sequences

Token counting uses the `cl100k_base` tokenizer (same family used by GPT-4 and Claude).

## License

MIT
