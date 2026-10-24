# Contributing to ShinySkills

Thank you for your interest in this project. Contributions are welcome.

This project is a Rust port of [agentskills/skills-ref](https://github.com/agentskills/agentskills/tree/main/skills-ref). The upstream project does not accept contributions, so this repository is where changes happen.

## Prerequisites

1. Install a stable Rust toolchain. The project uses edition 2021.
2. Do not use nightly-only formatting options. They break `cargo fmt` on stable.

## Set up your environment

```bash
git clone https://github.com/bjester/shinyskills.git
cd shinyskills
cargo build --release
```

## Where to start

1. Read `AGENTS.md`. It holds the build commands, the module layout, and the rules the build enforces.
2. Read `README.md`. It holds the CLI contract and the configuration format.
3. Pick a change. Small, focused changes merge faster than large ones.

## Before you commit

A change is done when all of these pass from a clean build:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

If you touch doc comments or intra-doc links, also run:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

The rules live in the repository, so local runs report what CI reports. `rustfmt.toml` sets the line width to 100 columns. `Cargo.toml` forbids `unsafe` code and fails the build on `unwrap()` outside tests.

## Style rules

1. Follow [Google's Rust Style Guide](./docs/rustguide.md). `rustfmt` and `clippy` enforce most of it.
2. Call through the module, not through free-function imports: `parser::read_properties(skill_dir)`.
3. Use full words, not abbreviations, for names.
4. Public items need doc comments. New public types get `#[non_exhaustive]` when they may grow. Pure functions get `#[must_use]`.
5. New source files start with the four-line header used in `src/`. Test files do not carry the header.

## The CLI contract is the API

The library API may change. The command line surface may not. Keep subcommands, flags, output text, and exit codes identical unless your change explicitly changes them. If a change breaks the library API, the version in `Cargo.toml` must be bumped and tagged.

## Tests

- Unit tests run with `cargo test --lib`. Integration tests run with `cargo test --test integration_tests`.
- Test counts are load-bearing: 22 unit tests and 58 integration tests. If your change drops a count, explain why in the pull request.
- When a change alters a public signature, update the call site and the test in the same commit.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Write a clear title and a short description of the change.
3. Make sure that the checks in "Before you commit" pass.
4. Open the pull request against `main`.

## Attribution

Add your name and GitHub account to `AUTHORS.md` with your first pull request. All contributions fall under the [MIT](LICENSE) license, with the attribution terms in that file.

## Reporting a bug

Open a GitHub issue. Include the command you ran, the output you saw, and the exit code. A minimal skill directory that reproduces the problem helps the most.