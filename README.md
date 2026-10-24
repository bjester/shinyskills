# ShinySkills

A validation tool for Agent Skills (Rust port of the reference library [agentskills/skills-ref](https://github.com/agentskills/agentskills/tree/main/skills-ref)).

This CLI utility is designed to validate LLM agent skills (their directories), checking for proper structure, required fields, and file references. Additionally, it provides a `validate-staged` subcommand intended for pre-commit hooks.

## What's different

The `skills-ref` tool is described as only a demonstration and reference library. They aren't accepting contributions there at this time. 

So this project has some additional features:
- It validates the skill's file references ([proposal](https://github.com/agentskills/agentskills/discussions/282))
- It prototypes allowing agent-specific customization of skills:
  - For example, `shinyskills validate --vibe <skill_dir>` allows `user-invocable` in the frontmatter
- It adds an additional `validate-staged` subcommand for pre-commit hook integration

## Building

```bash
cargo build --release
```

## Usage

### Validate a skill directory

```bash
cargo run -- validate /path/to/skill-directory
```

Exit codes:
- 0: Valid skill
- 1: Validation errors found

Options:
- `--vibe`: Enable Vibe-specific validation rules (allows `user-invocable` in frontmatter)
- `--config <PATH>`: Read configuration from this file instead of the repository root

### Validate based on changed files (for pre-commit)

```bash
cargo run -- validate-staged /path/to/changed/file1.py /path/to/changed/SKILL.md
```

This command is designed for pre-commit hooks. It:
- Accepts a list of changed files
- Finds which skill directories contain those files
- Validates each unique skill directory only once
- Reports errors prefixed with skill names

Options:
- `--vibe`: Enable Vibe-specific validation rules
- `--config <PATH>`: Read configuration from this file instead of the repository root
- `--context-dir <DIR>`: Limit upward search to within this directory
- `--max-depth <N>`: Maximum levels to walk up when searching for skill directory (default: 3)

### Read skill properties as JSON

```bash
cargo run -- read-properties /path/to/skill-directory
```

### Generate XML prompt block

```bash
cargo run -- to-prompt /path/to/skill1 /path/to/skill2
```

### Use as a library

The same checks are open to Rust callers. `validate` returns a `ValidationReport`, where errors decide validity and warnings do not.

```rust
use shinyskills::{parser, validator, ValidationMode};

let report = validator::validate(&skill_dir, ValidationMode::Default);
for warning in &report.warnings {
    println!("warning: {warning}");
}
if !report.is_valid() {
    for error in &report.errors {
        println!("error: {error}");
    }
}

match parser::read_properties(&skill_dir) {
    Ok(properties) => println!("{:?}", properties.to_map()),
    Err(e) => eprintln!("could not read properties: {e}"),
}
```

Callers that resolve repository configuration call `validator::validate_with_options` with `ValidationOptions` from `config::resolve_validation_options`.

## Repository Configuration

ShinySkills reads one configuration file: `.shinyskills.toml` in the repository root. It finds that root by walking up from the skill directory until it sees `.git`. A `.git` file, used by worktrees, also counts. Files in skill directories or other subfolders are ignored on purpose. When no file exists, ShinySkills uses built-in defaults and validation behaves as before.

To select the file yourself, pass `--config <PATH>`. An explicit path skips the search entirely. Both `validate` and `validate-staged` accept the option.

### Settings

All keys sit under `[validation]` and all are optional.

| Key | Type | Default | Effect |
| --- | --- | --- | --- |
| `allowed_fields` | list of strings | `name`, `description`, `license`, `allowed-tools`, `metadata`, `compatibility` | Frontmatter keys the validator accepts |
| `ignore_files` | list of glob patterns | empty, so nothing is ignored | Files the reference scanner skips |
| `scan_extensions` | list of strings | `.md` | File types whose body is read for reference syntax |

```toml
[validation]
allowed_fields = ["name", "description", "user-invocable"]
ignore_files = ["CHANGELOG.md", "docs/**"]
scan_extensions = [".md", ".markdown"]
```

`allowed_fields` adds to the default list, it does not replace it. A configured list can never remove a built-in key, so an empty list keeps the defaults.

Globs match paths relative to the skill directory. `docs/**` ignores `docs/nested/note.txt` but not `references/docs/note.txt`. A skipped file raises neither an orphan warning nor a broken-path error.

ShinySkills reads a file body only when its extension is listed. This stops shell scripts and source files from producing false broken-path errors. Extensions are matched case-insensitively, so `MD` and `.md` behave the same.

Unknown keys fail the run with an error that names the key, which catches typos such as `[validaton]`.

## Pre-Commit Hook

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/yourusername/shinyskills
    rev: v0.1.0  # Use the git tag you want
    hooks:
      - id: shinyskills-validate
        # Optional: limit search to your project directory
        args: ["--context-dir", "."]
        # Optional: adjust max depth (default is 3)
        # args: ["--max-depth", "2"]
```

The hook will:
- Run on any file change in your repository
- Automatically detect which skill directories need validation
- Only validate skills that contain changed files
- Report errors with skill names prefixed for clarity

## Development

Rust code has been aligned to [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.html).

Run what CI runs:

```bash
# Format
cargo fmt --all
cargo fmt --all -- --check

# Lint, where any warning fails the run
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all-features

# Documentation, where a broken doc fails the run
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

The rules live in the repository, not in a recipe, so local runs report what CI will report:

- `rustfmt.toml` sets the line width to 100 columns.
- `Cargo.toml` forbids `unsafe` code and warns on missing documentation for public items.
- `Cargo.toml` also warns on `unwrap()` outside tests, so run `cargo clippy` to see it.

## Structure

- `src/main.rs` - CLI entry point
- `src/lib.rs` - Library root and public re-exports
- `src/config.rs` - `.shinyskills.toml` discovery and resolution
- `src/errors.rs` - Custom error types
- `src/models.rs` - Data models (`ValidationReport`, `ParsedFrontmatter`, `SkillProperties`, `ValidationOptions`, `ValidationMode`)
- `src/parser.rs` - YAML frontmatter parsing and file scanning
- `src/validator.rs` - Validation logic
- `src/prompt.rs` - XML prompt generation

## AI Disclosure

I'm not a Rust developer, but I was keen to port the tool to Rust because of its speed benefits, particularly for pre-commit integration. In the initial port, I used Qwen3.5-122B to complete most of it, then Kimi-k2.5 to do review and set up pre-commit integration with new `validate-staged` subcommand. Since then, I've used other frontier models to create specifications and implementation plans, and have implemented those with local models.

Happy to have actual Rust engineers involved!


## License & Attribution

This project is a Rust port of [agentskills/skills-ref](https://github.com/agentskills/agentskills/tree/main/skills-ref), originally licensed under Apache 2.0, copyright (c) 2025 Anthropic, PBC.

Key changes and additions in this port:

- Language: Complete rewrite from Python to Rust for performance and type safety.

- Validation: Added skill file reference validation ([link to proposal](https://github.com/agentskills/agentskills/discussions/282)).

- Customization: Implemented agent-specific skill customization (e.g., --vibe flag).

- CLI Tools: Added validate-staged for pre-commit integration.

[MIT](LICENSE) :: Copyright 2026 Blaine Jester
