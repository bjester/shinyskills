# ShinySkills

A validation tool for Agent Skills (Rust port of the reference library [agentskills/skills-ref](https://github.com/agentskills/agentskills/tree/main/skills-ref)).

This CLI utility is designed to validate LLM agent skills (their directories), checking for proper structure, required fields, and file references. Additionally, it provides a `validate-staged` subcommand intended for pre-commit hooks.

> [!NOTE]
> This was vibe coded. Happy to have Rust engineers get involved!

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

```bash
# Build
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Structure

- `src/main.rs` - CLI entry point
- `src/lib.rs` - Library root
- `src/errors.rs` - Custom error types
- `src/models.rs` - Data models (SkillProperties)
- `src/parser.rs` - YAML frontmatter parsing and file scanning
- `src/validator.rs` - Validation logic
- `src/prompt.rs` - XML prompt generation

## AI Disclosure

I'm not a Rust developer, but I was keen to port the tool to Rust because of its speed benefits, particularly for pre-commit integration. I used Qwen3.5-122B to complete most of the port, then Kimi-k2.5 to do review and set up pre-commit integration with new `validate-staged` subcommand.


## License & Attribution

📜 License & Attribution
This project is a Rust port of [agentskills/skills-ref](https://github.com/agentskills/agentskills/tree/main/skills-ref), originally licensed under Apache 2.0, copyright (c) 2025 Anthropic, PBC.

Key changes and additions in this port:

- Language: Complete rewrite from Python to Rust for performance and type safety.

- Validation: Added skill file reference validation ([link to proposal](https://github.com/agentskills/agentskills/discussions/282)).

- Customization: Implemented agent-specific skill customization (e.g., --vibe flag).

- CLI Tools: Added validate-staged for pre-commit integration.

[MIT](LICENSE) :: Copyright 2026 Blaine Jester
