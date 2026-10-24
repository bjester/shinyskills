# Repository guidance

`shinyskills` is both a CLI and a Rust library that validates Agent Skill directories: frontmatter metadata, file references, and orphaned files. It ships as one binary (`shinyskills`) plus a public crate.

Rust source follows [Google's Rust Style Guide](./docs/rustguide.md). That alignment is not decoration: `rustfmt.toml` and the `[lints]` tables in `Cargo.toml` enforce it, so style work is mostly mechanical and reviewable.

## Commands

| Task | Command |
| --- | --- |
| Build | `cargo build --release` |
| Unit tests | `cargo test --lib` |
| Integration tests | `cargo test --test integration_tests` |
| Everything | `cargo test --all-features` |
| Format in place | `cargo fmt --all` |
| Format check | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Docs check | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` |

A change is done when fmt, clippy and `cargo test --all-features` all pass from a clean build. CI runs those three plus a release build. Run the doc check whenever you touch doc comments or intra-doc links, because CI does not.

Toolchain is stable Rust, edition 2021. Nightly-only formatting options break `cargo fmt` on stable, so do not add them to `rustfmt.toml`.

## Layout and dependency direction

```
main.rs ──► validator.rs ──► parser.rs ──► models.rs / errors.rs
   │            │
   ├──► prompt.rs
   └──► config.rs ─────────► models.rs / errors.rs
```

`lib.rs` is only module declarations and re-exports; it is a non-leaf module.

Keep that direction. A leaf module (`models.rs`, `errors.rs`) must not reach back into `parser.rs`, `validator.rs`, or `config.rs`.

- `src/main.rs` - argument parsing, process exit codes, nothing else. Validation logic here is a bug.
- `src/config.rs` - `.shinyskills.toml` discovery and resolution to `ValidationOptions`.
- `src/parser.rs` - `SKILL.md` frontmatter parsing, `SkillProperties`, reference scanning.
- `src/prompt.rs` - `<available_skills>` XML generation.
- `src/validator.rs` - metadata and reference validation, returns `ValidationReport`.
- `src/models.rs` - shared data types, including `ValidationReport` and `ParsedFrontmatter`.
- `src/errors.rs` - `SkillError`.
- `tests/integration_tests.rs` - library behavior (`test_*`) and CLI contracts (`cli_*`).

New source files start with the four-line header used in `src/`: `Copyright 2026 Blaine Jester`, `SPDX-License-Identifier: MIT`, and the `Derived from agentskills/skills-ref (Apache-2.0)` line. Test files do not carry the header.

## Rules the build enforces

- Public enums and structs that may grow get `#[non_exhaustive]`. Add it when adding a public type.
- Functions returning a value without side effects get `#[must_use]`.
- `unsafe` code is forbidden by lint level.
- Public items need doc comments; each module starts with a `//!` line stating its job.
- `unwrap()` fails the build outside test modules. Inside `#[cfg(test)] mod tests`, prefer an `#[allow(clippy::unwrap_used)]` on the module over per-call allows.

Fix the cause of a warning rather than silencing it with `#[allow]`. When an allow is genuinely right, state the reason on the line above.

## Conventions this repo holds beyond the lints

- Call through the module, not through free-function imports: `parser::read_properties(skill_dir)`, `config::load_config(path)`. Type imports stay unqualified: `use std::path::Path;`.
- Names are words, not abbreviations: `context`, `relative_path`, `description`, `subdirectory_files`.
- Bind non-obvious iterators and long collections to a named local before looping, so the loop header stays readable.
- Prefer `let ... else` over a `match` with one interesting arm and a `panic!` in the rest.
- No glob imports (`use foo::*`) except inside `#[cfg(test)] mod tests`.
- When a panic is unreachable because an invariant holds, say the invariant in the message: `.expect("escaped basename produces a valid regex")`.

## The CLI contract is the API

The library API is allowed to change; the command line surface is not. Keep subcommands, flags, exact stdout and stderr text, and exit codes identical unless a task explicitly changes them.

- `validate <path>`, `validate-staged <files...>`: exit 1 on errors, otherwise 0. Warnings go to stderr and never change the exit code.
- `validate-staged` exits 0 silently when no skill directory is affected.
- `read-properties <path>`: prints a JSON object; exit 1 on parse or read failure.
- `to-prompt <paths...>`: prints the XML block.
- Configuration resolution: `<repo-root>/.shinyskills.toml` only, found by walking up to `.git`, which may be a file for worktrees. Nested files are ignored on purpose. `--config <PATH>` skips discovery. Unknown keys must fail with the key named.

`read-properties` keys come from a `HashMap`, so their order is not stable. Do not write a test that depends on JSON key order.

## Testing

- 22 unit tests (`cargo test --lib`), 58 integration tests. Both counts are load-bearing signals: a refactor that drops a test count must explain why.
- `cli_*` tests spawn the built binary through `env!("CARGO_BIN_EXE_shinyskills")`. They are the guard for the contract above.
- When a change alters a public signature, update the call site and the test in the same commit. Never leave the tree in a state where `cargo test` does not compile.
- For a sweep touching many call sites, write a deterministic transform, apply it once, then read the diff. Manual edits across 30 sites are slower and unverifiable.

## Before you act

Ask first, or state clearly what you assumed, before:

- Adding any dependency to `Cargo.toml`.
- Editing `.github/workflows/ci.yml` or `release.yml`.
- Changing what a subcommand prints or returns, or bumping `version` in `Cargo.toml` and tagging a release.

Source-breaking library changes (for example the `to_dict` to `to_map` rename and the move from tuples to `ValidationReport`) need a version bump and a matching tag for the pre-commit hook example in `README.md` to stay honest.

