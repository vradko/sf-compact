# sf-compact

Convert Salesforce metadata XML to AI-friendly YAML and back. Lossless roundtrip.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000–50,000+ lines of XML with 70–85% structural overhead. This burns tokens and money when AI tools (Claude Code, Codex, Cursor, etc.) read or edit your metadata.

**sf-compact** converts it to a compact YAML format that preserves all information but uses ~50% fewer tokens.

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

**YAML (432 tokens) — 49% reduction:**
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

### Pack (XML → YAML)
```bash
sf-compact pack [source...] [-o output]
```

Convert Salesforce metadata XML to compact YAML.

```bash
# Pack entire project
sf-compact pack force-app -o .sf-compact

# Pack specific directories
sf-compact pack force-app/main/default/profiles force-app/main/default/classes

# Pack only profiles
sf-compact pack force-app --include "*.profile-meta.xml"
```

### Unpack (YAML → XML)
```bash
sf-compact unpack [source...] [-o output]
```

Convert compact YAML back to Salesforce metadata XML (lossless).

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

Output supported metadata types in JSON:

```bash
sf-compact manifest
```

## Supported Metadata Types

43 Salesforce metadata types across 7 categories:

| Category | Types |
|----------|-------|
| **Security** | Profile, PermissionSet, PermissionSetGroup, RemoteSiteSetting, CspTrustedSite, ConnectedApp, SharingRules |
| **Schema** | CustomObject, CustomField, ValidationRule, CustomMetadata, GlobalValueSet, StandardValueSet, RecordType, MatchingRule, DuplicateRule |
| **Code** | ApexClass, ApexTrigger, ApexComponent, ApexPage, LightningComponentBundle (js/css/html/xml) |
| **Automation** | Flow, Workflow, AssignmentRules, AutoResponseRules, EscalationRules |
| **UI** | Layout, CustomLabels, CustomApplication, CustomTab, FlexiPage, CustomSite, QuickAction, PathAssistant, ListView, CompactLayout, WebLink |
| **Analytics** | ReportType, Report, Dashboard |
| **Content** | EmailTemplate |

## Workflow

1. **Pull metadata** from Salesforce (`sf project retrieve`)
2. **Pack**: `sf-compact pack` — creates `.sf-compact/` with YAML versions
3. **Work with YAML** — let AI tools read/edit the compact format
4. **Unpack**: `sf-compact unpack` — restores XML for deployment
5. **Deploy** to Salesforce (`sf project deploy`)

> Tip: Add `.sf-compact/` to `.gitignore` if you treat it as a build artifact, or commit it for AI-friendly diffs.

## How it works

- Parses Salesforce metadata XML into a tree structure
- Groups repeated elements (e.g., `<fieldPermissions>`) into YAML arrays
- Coerces types: `"true"` → `true`, `"59.0"` → `59.0`
- Flattens simple key-value containers into inline YAML mappings
- Preserves namespaces, attributes, and all structural information for lossless roundtrip

Token counting uses the `cl100k_base` tokenizer (same family used by GPT-4 and Claude).

## License

MIT
