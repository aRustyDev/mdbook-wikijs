# mdbook-wikijs

MDBook backend that outputs Wiki.js-compatible markdown with schema validation.

## Installation

```bash
cargo install mdbook-wikijs
```

Or via Homebrew:

```bash
brew tap arustydev/tap
brew install mdbook-wikijs
```

## Usage

Add to your `book.toml`:

```toml
[output.wikijs]
output-dir = "wikijs-output"
validate = true
path-prefix = "/docs"

[output.wikijs.frontmatter]
published = true
editor = "markdown"
tags = ["documentation"]
```

Then build:

```bash
mdbook build
```

## Features

### Admonition Conversion

Converts MDBook/GitHub-style admonitions to Wiki.js callout syntax:

| MDBook Syntax | Wiki.js Output |
|---------------|----------------|
| `> [!NOTE]` | `{.is-info}` |
| `> [!WARNING]` | `{.is-warning}` |
| `> [!DANGER]` | `{.is-danger}` |
| `> [!TIP]` | `{.is-success}` |

### Link Rewriting

Relative markdown links are converted to Wiki.js absolute paths:

```markdown
<!-- Input -->
[guide](./getting-started.md)

<!-- Output (with path-prefix = "/docs") -->
[guide](/docs/getting-started)
```

### Schema Validation

Output is validated against a JSON Schema to ensure Wiki.js compatibility:

- Frontmatter must include `title`
- Only valid Wiki.js classes allowed: `{.is-info}`, `{.is-warning}`, etc.
- Path format validation

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `output-dir` | path | `wikijs/` | Output directory |
| `validate` | bool | `false` | Enable schema validation |
| `schema` | path | (builtin) | Custom schema path |
| `path-prefix` | string | `""` | Wiki.js path prefix |
| `frontmatter.published` | bool | `true` | Default published state |
| `frontmatter.editor` | string | `"markdown"` | Default editor type |
| `frontmatter.tags` | array | `[]` | Default tags |

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run with sample book
cd /path/to/mdbook-project
mdbook build
```

## License

MIT
