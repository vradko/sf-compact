# sf-compact

Convert Salesforce metadata XML to AI-friendly compact formats. Semantically lossless roundtrip for Salesforce metadata.

Salesforce metadata XML is extremely verbose — profiles, permission sets, flows, and objects can be 20,000–50,000+ lines of XML with 70–85% structural overhead. This burns tokens and money when AI tools (Claude Code, Codex, Cursor, etc.) read or edit your metadata.

**sf-compact** converts it to compact YAML or JSON, saving 42–54% of tokens depending on format.

## Output Formats

| Format | Preserves order | Human-readable | Token savings | Default |
|--------|:-:|:-:|:-:|:-:|
| `json` | Yes | Less | ~54% | **Default** |
| `yaml` | No | Yes | ~49% | Order-insensitive types |
| `yaml-ordered` | Yes | Yes | ~42% | — |

- **json** (default) — compact single-line JSON. Preserves element order, fewest tokens. Recommended for most metadata types.
- **yaml** — groups repeated elements into arrays. More human-readable, but sibling order may change. Use for order-insensitive types (Profile, PermissionSet) where readability matters.
- **yaml-ordered** — uses `_children` sequences to preserve exact element order in YAML. Use when you need both YAML readability and order preservation.

## What "semantically lossless" means

The roundtrip preserves all data that Salesforce cares about. These XML features are normalized:

- **Whitespace trimming** — leading/trailing whitespace in text nodes is trimmed (safe for SF metadata)
- **Comments stripped** — `<!-- ... -->` are removed (Salesforce metadata doesn't use comments)
- **CDATA unwrapped** — `<![CDATA[...]]>` becomes escaped text (`&lt;`, `&amp;`) — semantically identical
- **Empty elements** — `<tag></tag>` may become `<tag/>` — semantically identical
- **Element order** — may change with `yaml` format; use `yaml-ordered` or `json` to preserve order

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

### npm (recommended — no build required)
```bash
npm install -g sf-compact-cli
```

### Homebrew (macOS / Linux)
```bash
brew install vradko/tap/sf-compact
```

### From crates.io (Rust required)
```bash
cargo install sf-compact
```

### From source
```bash
cargo install --path .
```

## Usage

### Pack (XML → compact format)
```bash
sf-compact pack [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern] [--incremental]
```

```bash
# Pack entire project (default: YAML format)
sf-compact pack force-app -o .sf-compact

# Pack as JSON for maximum token savings
sf-compact pack force-app --format json

# Incremental: only repack files modified since last pack
sf-compact pack --incremental

# Pack specific directories
sf-compact pack force-app/main/default/profiles force-app/main/default/classes

# Pack only profiles
sf-compact pack force-app --include "*.profile-meta.xml"

# Limit parallel threads (default: all CPU cores)
sf-compact -j 4 pack force-app
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
default_format: json
formats:
  Profile: yaml
  PermissionSet: yaml
  PermissionSetGroup: yaml
  # ... other order-insensitive types get yaml for readability
skip: []
```

When `pack` runs, it reads `.sfcompact.yaml` and applies the format per metadata type. The `--format` CLI flag overrides the config for a single run.

### Watch (auto-pack on changes)
```bash
sf-compact watch [source...] [-o output] [--format yaml|yaml-ordered|json] [--include pattern]
```

Watches source directories for XML changes and automatically repacks. Runs an initial pack, then monitors for file changes.

```bash
# Watch default force-app directory
sf-compact watch

# Watch with JSON format
sf-compact watch force-app --format json
```

### Diff (detect unpacked changes)
```bash
sf-compact diff [source...] [-o packed-dir] [--include pattern]
```

Compare current XML metadata against the last packed output. Shows new, modified, and deleted files.

```bash
$ sf-compact diff

  + force-app/main/default/profiles/NewProfile.profile-meta.xml  (new — not yet packed)
  ~ force-app/main/default/flows/Case_Assignment.flow-meta.xml  (modified since last pack)

1 new, 1 modified, 0 deleted, 3 unchanged
Run `sf-compact pack` to update.
```

### Lint (CI validation)
```bash
sf-compact lint [source...] [-o packed-dir] [--include pattern]
```

Check that compact files are up-to-date. Exits with code 1 if any files are stale, new, or orphaned. Use in CI pipelines.

### Changes (track modified compact files)
```bash
sf-compact changes [-o compact-dir]                    # show all modified files (global)
sf-compact changes --since-deploy                      # show changes since last deploy reset
sf-compact changes --json                              # machine-readable JSON output
sf-compact changes reset --global                      # clear all tracking
sf-compact changes reset --since-deploy                # clear deployment tracking only
```

Tracks which compact files were modified (by AI or human) since last `pack`. Per-branch tracking with two scopes:
- **Global** — all files changed since tracking started. For final retrieve before commit.
- **Deployment** — delta since last deploy reset. For deploying only what changed.

```bash
$ sf-compact changes

3 file(s) modified globally:

  M main/default/objects/Account/Account.object-meta.xml
  M main/default/profiles/Admin.profile-meta.xml
  M main/default/flows/Case_Assignment.flow-meta.xml

To deploy changes:
  sf project deploy start -d main/default/objects/Account/Account.object-meta.xml -d ...

To retrieve canonical XML before commit:
  sf project retrieve start -d main/default/objects/Account/Account.object-meta.xml -d ...
```

### MCP Server

sf-compact includes a built-in [MCP](https://modelcontextprotocol.io/) server for direct AI tool integration.

```bash
# Add to your project's .mcp.json
sf-compact init mcp

# Or start manually
sf-compact mcp-serve
```

This exposes `sf_compact_pack`, `sf_compact_unpack`, `sf_compact_stats`, `sf_compact_lint`, and `sf_compact_changes` as MCP tools that Claude Code, Cursor, and other MCP-compatible tools can discover and use automatically.

### AI Instructions

Generate a provider-agnostic markdown file with usage instructions for any AI tool:

```bash
sf-compact init instructions
sf-compact init instructions --name SALESFORCE.md
```

### Cursor / Windsurf Rules

Generate `.cursorrules` and `.windsurfrules` files that teach AI editors to prefer compact files:

```bash
sf-compact init cursorrules
```

This creates rules that instruct Cursor, Windsurf, and similar AI editors to read from `.sf-compact/` instead of raw XML, saving tokens on every interaction.

### Manifest

Output supported metadata types in JSON (includes format support and order-sensitivity flags):

```bash
sf-compact manifest
```

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

\* Order-sensitive types — `config init` defaults these to `yaml-ordered` to preserve element order.

## Workflow

1. **Configure** (once): `sf-compact config init` — creates `.sfcompact.yaml` with smart defaults
2. **Pull metadata** from Salesforce (`sf project retrieve`)
3. **Pack**: `sf-compact pack` — creates `.sf-compact/` with compact files
4. **Work with compact files** — let AI tools read/edit the YAML/JSON format
5. **Unpack**: `sf-compact unpack` — restores XML for deployment
6. **Deploy** to Salesforce (`sf project deploy`)

> Use `sf-compact watch` during development to auto-pack on changes, and `sf-compact diff` to check if a repack is needed.

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
