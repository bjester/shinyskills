// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
//! Validation, parsing, configuration, and prompt generation for Agent Skills.
pub mod config;
pub mod errors;
pub mod models;
pub mod parser;
pub mod prompt;
pub mod validator;

pub use models::{ParsedFrontmatter, ValidationMode, ValidationOptions, ValidationReport};
