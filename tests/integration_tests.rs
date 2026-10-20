use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use shinyskills::parser::{
    find_skill_dir_for_file, find_skill_md, is_path_referenced, ReferenceStatus,
};
use shinyskills::prompt::to_prompt;
use shinyskills::validator::{validate, validate_with_options, ValidationMode};

fn create_valid_skill(dir: &TempDir) -> PathBuf {
    let skill_dir = dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
# My Skill

Some content here.
"#,
    )
    .unwrap();
    skill_dir
}

#[test]
fn test_valid_skill() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = create_valid_skill(&tmp_dir);

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_nonexistent_path() {
    let tmp_dir = TempDir::new().unwrap();
    let path = tmp_dir.path().join("nonexistent");

    let (warnings, errors) = validate(&path, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("does not exist"));
}

#[test]
fn test_not_a_directory() {
    let tmp_dir = TempDir::new().unwrap();
    let file_path = tmp_dir.path().join("file.txt");
    fs::write(&file_path, "test").unwrap();

    let (warnings, errors) = validate(&file_path, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("Not a directory"));
}

#[test]
fn test_missing_skill_md() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("Missing required file: SKILL.md"));
}

#[test]
fn test_invalid_name_uppercase() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("MySkill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: MySkill
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("lowercase")));
}

#[test]
fn test_name_too_long() {
    let tmp_dir = TempDir::new().unwrap();
    let long_name = "a".repeat(70);
    let skill_dir = tmp_dir.path().join(&long_name);
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            r#"---
name: {}
description: A test skill
---
Body
"#,
            long_name
        ),
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.contains("exceeds") && e.contains("character limit")));
}

#[test]
fn test_name_leading_hyphen() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("-my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: -my-skill
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.contains("cannot start or end with a hyphen")));
}

#[test]
fn test_name_consecutive_hyphens() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my--skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my--skill
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("consecutive hyphens")));
}

#[test]
fn test_name_invalid_characters() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my_skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my_skill
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("invalid characters")));
}

#[test]
fn test_name_directory_mismatch() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("wrong-name");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: correct-name
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("must match skill name")));
}

#[test]
fn test_unexpected_fields() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
unknown_field: should not be here
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("Unexpected fields")));
}

#[test]
fn test_valid_with_all_fields() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
license: MIT
metadata:
  author: Test
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_realistic_tweetclaw_skill_fixture_references_files() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("tweetclaw");
    fs::create_dir(&skill_dir).unwrap();

    let refs_dir = skill_dir.join("references");
    fs::create_dir(&refs_dir).unwrap();
    fs::write(
        refs_dir.join("openclaw-approval.md"),
        "# OpenClaw Approval\n\nUse source gathering before approved account actions.\n",
    )
    .unwrap();

    let scripts_dir = skill_dir.join("scripts");
    fs::create_dir(&scripts_dir).unwrap();
    fs::write(
        scripts_dir.join("validate-export.mjs"),
        "JSON.parse(process.argv[2] || '{}');\n",
    )
    .unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: tweetclaw
description: Use TweetClaw for reviewed OpenClaw X/Twitter source packets, monitor summaries, and approval-gated account actions.
license: MIT
compatibility: OpenClaw plugin published as npm:@xquik/tweetclaw
metadata:
  package: "@xquik/tweetclaw"
  clawhub: "https://clawhub.ai/plugins/@xquik/tweetclaw"
---
# TweetClaw

Review [OpenClaw approval guidance](references/openclaw-approval.md) before
using write-capable tools. Validate exported JSON with
`scripts/validate-export.mjs` before passing source packets to another skill.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_allowed_tools_accepted() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
allowed-tools: Bash(jq:*) Bash(git:*)
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_i18n_chinese_name() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("技能");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: 技能
description: A skill with Chinese name
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_i18n_russian_name_with_hyphens() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("мой-навык");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: мой-навык
description: A skill with Russian name
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_i18n_russian_lowercase_valid() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("навык");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: навык
description: A skill with Russian lowercase name
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_i18n_russian_uppercase_rejected() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("НАВЫК");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: НАВЫК
description: A skill with Russian uppercase name
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.iter().any(|e| e.contains("lowercase")));
}

#[test]
fn test_description_too_long() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    let long_desc = "x".repeat(1100);
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            r#"---
name: my-skill
description: {}
---
Body
"#,
            long_desc
        ),
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.contains("exceeds") && e.contains("1024")));
}

#[test]
fn test_valid_compatibility() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
compatibility: Requires Python 3.11+
---
Body
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_compatibility_too_long() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    let long_compat = "x".repeat(550);
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            r#"---
name: my-skill
description: A test skill
compatibility: {}
---
Body
"#,
            long_compat
        ),
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.contains("exceeds") && e.contains("500")));
}

#[test]
fn test_all_files_referenced() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let refs_dir = skill_dir.join("references");
    fs::create_dir(&refs_dir).unwrap();
    let scripts_dir = skill_dir.join("scripts");
    fs::create_dir(&scripts_dir).unwrap();

    fs::write(refs_dir.join("GUIDE.md"), "# Guide").unwrap();
    fs::write(scripts_dir.join("run.py"), "# Script").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
See [the guide](references/GUIDE.md) and run `scripts/run.py`.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_orphaned_file_warning() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let refs_dir = skill_dir.join("references");
    fs::create_dir(&refs_dir).unwrap();
    fs::write(refs_dir.join("GUIDE.md"), "# Guide").unwrap();
    fs::write(refs_dir.join("UNUSED.md"), "# Unused").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
See [the guide](references/GUIDE.md) for details.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("Orphaned file"));
    assert!(warnings[0].contains("references/UNUSED.md"));
    assert!(errors.is_empty());
}

#[test]
fn test_multiple_orphaned_files() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let refs_dir = skill_dir.join("references");
    fs::create_dir(&refs_dir).unwrap();
    let scripts_dir = skill_dir.join("scripts");
    fs::create_dir(&scripts_dir).unwrap();

    fs::write(refs_dir.join("UNUSED1.md"), "# Unused 1").unwrap();
    fs::write(refs_dir.join("UNUSED2.md"), "# Unused 2").unwrap();
    fs::write(scripts_dir.join("orphan.py"), "# Orphan").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
No file references here.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert_eq!(warnings.len(), 3);
    assert!(warnings.iter().any(|w| w.contains("UNUSED1.md")));
    assert!(warnings.iter().any(|w| w.contains("UNUSED2.md")));
    assert!(warnings.iter().any(|w| w.contains("orphan.py")));
    assert!(errors.is_empty());
}

#[test]
fn test_example_paths_not_flagged() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
Example usage:

```
python scripts/example.py --input data/file.txt
```

This is just an example, the files don't need to exist.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_hidden_files_ignored() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(skill_dir.join(".hidden"), "hidden content").unwrap();
    fs::write(skill_dir.join(".gitignore"), "*.pyc").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
No file references.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_venv_files_ignored() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let venv_dir = skill_dir.join("venv");
    fs::create_dir(&venv_dir).unwrap();
    fs::write(venv_dir.join("script.py"), "# venv script").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
No file references.
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_file_referenced_in_other_scannable_file() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    let refs_dir = skill_dir.join("references");
    fs::create_dir(&refs_dir).unwrap();

    fs::write(refs_dir.join("GUIDE.md"), "# Guide").unwrap();
    fs::write(
        refs_dir.join("INDEX.md"),
        "# Index\n\nSee [the guide](references/GUIDE.md).",
    )
    .unwrap();
    fs::write(refs_dir.join("UNUSED.md"), "# Unused").unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
Read [the index](references/INDEX.md).
"#,
    )
    .unwrap();

    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("UNUSED.md"));
    assert!(!warnings[0].contains("GUIDE.md"));
    assert!(errors.is_empty());
}

#[test]
fn test_find_skill_md() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    // Test with SKILL.md
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();
    assert!(find_skill_md(&skill_dir).is_some());

    // Test with skill.md
    fs::remove_file(skill_dir.join("SKILL.md")).unwrap();
    fs::write(skill_dir.join("skill.md"), "---\nname: test\n---\nBody").unwrap();
    assert!(find_skill_md(&skill_dir).is_some());

    // Test with neither
    fs::remove_file(skill_dir.join("skill.md")).unwrap();
    assert!(find_skill_md(&skill_dir).is_none());
}

#[test]
fn test_is_path_referenced_found() {
    let body = "See [the guide](references/GUIDE.md) for details.";
    let path = PathBuf::from("references/GUIDE.md");

    assert_eq!(is_path_referenced(body, &path), ReferenceStatus::Found);
}

#[test]
fn test_is_path_referenced_orphaned() {
    let body = "No references here.";
    let path = PathBuf::from("references/GUIDE.md");

    assert_eq!(is_path_referenced(body, &path), ReferenceStatus::Orphaned);
}

#[test]
fn test_is_path_referenced_broken_path() {
    let body = "See docs/GUIDE.md for details.";
    let path = PathBuf::from("references/GUIDE.md");

    assert_eq!(is_path_referenced(body, &path), ReferenceStatus::BrokenPath);
}

#[test]
fn test_is_path_referenced_backslash_normalized() {
    let body = "See references\\GUIDE.md for details.";
    let path = PathBuf::from("references/GUIDE.md");

    assert_eq!(is_path_referenced(body, &path), ReferenceStatus::Found);
}

#[test]
fn test_is_path_referenced_leading_dot_slash() {
    let path = PathBuf::from("scripts/find-polluter.sh");

    // Relative forms are the idiomatic way to refer to skill assets.
    let backtick = "Run `./scripts/find-polluter.sh` first.";
    assert_eq!(
        is_path_referenced(backtick, &path),
        ReferenceStatus::Found,
        "`./` prefix must normalize away"
    );

    let link = "[Run](././scripts/find-polluter.sh)";
    assert_eq!(
        is_path_referenced(link, &path),
        ReferenceStatus::Found,
        "repeated `./` prefixes must normalize away"
    );

    let windows = "Run .\\scripts\\find-polluter.sh once.";
    assert_eq!(
        is_path_referenced(windows, &path),
        ReferenceStatus::Found,
        "Windows-style `./` separators must normalize away"
    );
}

#[test]
fn test_to_prompt_empty() {
    let result = to_prompt(&[]);
    assert_eq!(result, "<available_skills>\n</available_skills>");
}

#[test]
fn test_to_prompt_single_skill() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
Body
"#,
    )
    .unwrap();

    let result = to_prompt(&[&skill_dir]);
    assert!(result.contains("<available_skills>"));
    assert!(result.contains("<skill>"));
    assert!(result.contains("<name>my-skill</name>"));
    assert!(result.contains("<description>A test skill</description>"));
    assert!(result.contains("</available_skills>"));
}

#[test]
fn test_to_prompt_xml_escaping() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A <special> & "quoted" skill
---
Body
"#,
    )
    .unwrap();

    let result = to_prompt(&[&skill_dir]);
    assert!(result.contains("&lt;special&gt;"));
    assert!(result.contains("&amp;"));
    assert!(result.contains("&quot;quoted&quot;"));
}

#[test]
fn test_find_skill_dir_for_file_in_skill_root() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();

    // File directly in skill directory
    let file_path = skill_dir.join("script.py");
    fs::write(&file_path, "# script").unwrap();

    let result = find_skill_dir_for_file(&file_path, None, 3);
    assert_eq!(result, Some(skill_dir));
}

#[test]
fn test_find_skill_dir_for_file_in_subdir() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    let scripts_dir = skill_dir.join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();

    // File in subdirectory
    let file_path = scripts_dir.join("run.py");
    fs::write(&file_path, "# script").unwrap();

    let result = find_skill_dir_for_file(&file_path, None, 3);
    assert_eq!(result, Some(skill_dir));
}

#[test]
fn test_find_skill_dir_for_file_not_in_skill() {
    let tmp_dir = TempDir::new().unwrap();
    let some_dir = tmp_dir.path().join("some-dir");
    fs::create_dir(&some_dir).unwrap();

    let file_path = some_dir.join("file.txt");
    fs::write(&file_path, "content").unwrap();

    // No SKILL.md anywhere
    let result = find_skill_dir_for_file(&file_path, None, 3);
    assert_eq!(result, None);
}

#[test]
fn test_find_skill_dir_for_file_with_context_dir() {
    let tmp_dir = TempDir::new().unwrap();
    let project_dir = tmp_dir.path().join("project");
    let skill_dir = project_dir.join("my-skill");
    let subdir = skill_dir.join("scripts");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();

    // File in subdirectory of skill
    let file_path = subdir.join("run.py");
    fs::write(&file_path, "# script").unwrap();

    // With context_dir set to project, should find it (walking up from scripts/ to my-skill/)
    let result = find_skill_dir_for_file(&file_path, Some(&project_dir), 3);
    assert_eq!(result, Some(skill_dir));

    // With context_dir set to a subdirectory that doesn't contain the skill,
    // should not find it because we'd have to walk outside the context
    let deep_context = project_dir.join("my-skill").join("scripts");
    let result = find_skill_dir_for_file(&file_path, Some(&deep_context), 3);
    // The file is in scripts/, and we check if scripts/ has SKILL.md (no)
    // Then we check if my-skill/ has SKILL.md (yes), but my-skill/ is outside deep_context
    // So we should NOT find it
    assert_eq!(result, None);
}

#[test]
fn test_find_skill_dir_for_file_with_max_depth() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    let deep_dir = skill_dir.join("level1").join("level2").join("level3");
    fs::create_dir_all(&deep_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();

    let file_path = deep_dir.join("file.py");
    fs::write(&file_path, "# script").unwrap();

    // With max_depth=3, should find it (3 levels up from file: level3->level2->level1->skill_dir)
    let result = find_skill_dir_for_file(&file_path, None, 3);
    assert_eq!(result, Some(skill_dir));

    // With max_depth=2, should NOT find it (needs 3 levels)
    let result = find_skill_dir_for_file(&file_path, None, 2);
    assert_eq!(result, None);
}

#[test]
fn test_find_skill_dir_for_file_max_depth_zero() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\nBody").unwrap();

    // File in skill directory with max_depth=0
    let file_path = skill_dir.join("script.py");
    fs::write(&file_path, "# script").unwrap();

    // Should still find it because we check the file's directory first (depth 0)
    let result = find_skill_dir_for_file(&file_path, None, 0);
    assert_eq!(result, Some(skill_dir.clone()));

    // File in subdirectory with max_depth=0
    let subdir = skill_dir.join("subdir");
    fs::create_dir(&subdir).unwrap();
    let subfile = subdir.join("file.py");
    fs::write(&subfile, "# script").unwrap();

    // Should NOT find it because we'd need to walk up at least 1 level
    let result = find_skill_dir_for_file(&subfile, None, 0);
    assert_eq!(result, None);
}

// Tests for validate-staged behavior

#[test]
fn test_multiple_files_same_skill_deduplication() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    let scripts_dir = skill_dir.join("scripts");
    let refs_dir = skill_dir.join("references");
    fs::create_dir_all(&scripts_dir).unwrap();
    fs::create_dir_all(&refs_dir).unwrap();

    // Create SKILL.md with references to all files
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
---
See [guide](references/GUIDE.md) and run `scripts/run.py`.
"#,
    )
    .unwrap();

    fs::write(refs_dir.join("GUIDE.md"), "# Guide").unwrap();
    fs::write(scripts_dir.join("run.py"), "# Script").unwrap();

    // Simulate finding skill dirs from multiple files in the same skill
    let file1 = refs_dir.join("GUIDE.md");
    let file2 = scripts_dir.join("run.py");

    let skill_dir1 = find_skill_dir_for_file(&file1, None, 3);
    let skill_dir2 = find_skill_dir_for_file(&file2, None, 3);

    // Both should find the same skill directory
    assert_eq!(skill_dir1, skill_dir2);
    assert_eq!(skill_dir1, Some(skill_dir.clone()));

    // Validate the skill once (simulating deduplication)
    let (warnings, errors) = validate(&skill_dir, ValidationMode::Default);
    assert!(warnings.is_empty());
    assert!(errors.is_empty());
}

#[test]
fn test_multiple_files_different_skills() {
    let tmp_dir = TempDir::new().unwrap();

    // Create two separate skills
    let skill1_dir = tmp_dir.path().join("skill-one");
    let skill2_dir = tmp_dir.path().join("skill-two");
    fs::create_dir(&skill1_dir).unwrap();
    fs::create_dir(&skill2_dir).unwrap();

    fs::write(
        skill1_dir.join("SKILL.md"),
        r#"---
name: skill-one
description: First skill
---
Body
"#,
    )
    .unwrap();

    fs::write(
        skill2_dir.join("SKILL.md"),
        r#"---
name: skill-two
description: Second skill
---
Body
"#,
    )
    .unwrap();

    // Simulate finding skill dirs from files in different skills
    let file1 = skill1_dir.join("file.txt");
    let file2 = skill2_dir.join("file.txt");
    fs::write(&file1, "content").unwrap();
    fs::write(&file2, "content").unwrap();

    let skill_dir1 = find_skill_dir_for_file(&file1, None, 3);
    let skill_dir2 = find_skill_dir_for_file(&file2, None, 3);

    // Should find different skill directories
    assert_eq!(skill_dir1, Some(skill1_dir.clone()));
    assert_eq!(skill_dir2, Some(skill2_dir.clone()));
    assert_ne!(skill_dir1, skill_dir2);

    // Both should validate successfully
    let (_, errors1) = validate(&skill1_dir, ValidationMode::Default);
    let (_, errors2) = validate(&skill2_dir, ValidationMode::Default);
    assert!(errors1.is_empty());
    assert!(errors2.is_empty());
}

#[test]
fn test_vibe_mode_allows_user_invocable() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("my-skill");
    fs::create_dir(&skill_dir).unwrap();

    // Create SKILL.md with user-invocable field
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: my-skill
description: A test skill
user-invocable: true
---
Body
"#,
    )
    .unwrap();

    // Default mode should reject user-invocable
    let (_, errors_default) = validate(&skill_dir, ValidationMode::Default);
    assert!(errors_default.iter().any(|e| e.contains("user-invocable")));

    // Vibe mode should accept user-invocable
    let (_, errors_vibe) = validate(&skill_dir, ValidationMode::Vibe);
    assert!(!errors_vibe.iter().any(|e| e.contains("user-invocable")));
}

#[test]
fn test_validate_staged_with_context_dir() {
    let tmp_dir = TempDir::new().unwrap();
    let project_dir = tmp_dir.path().join("project");
    let skill_dir = project_dir.join("my-skill");
    let subdir = skill_dir.join("scripts");
    fs::create_dir_all(&subdir).unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: Test\n---\nBody",
    )
    .unwrap();

    let file_path = subdir.join("run.py");
    fs::write(&file_path, "# script").unwrap();

    // With context_dir set to project, should find the skill
    let result = find_skill_dir_for_file(&file_path, Some(&project_dir), 3);
    assert_eq!(result, Some(skill_dir));

    // With context_dir set to a directory that doesn't contain the skill,
    // should not find it
    let outside_dir = tmp_dir.path().join("outside");
    fs::create_dir(&outside_dir).unwrap();
    let result = find_skill_dir_for_file(&file_path, Some(&outside_dir), 3);
    assert_eq!(result, None);
}

#[test]
fn test_scoped_scanning_and_ignore_files() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("scoped-skill");
    let scripts_dir = skill_dir.join("scripts");
    let tests_dir = skill_dir.join("tests");
    fs::create_dir_all(&scripts_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    fs::write(
        scripts_dir.join("run.sh"),
        "#!/bin/sh\n# placeholder asset for scoped-skill\n",
    )
    .unwrap();
    // Correct basename, wrong directory: must not produce a broken-path error.
    fs::write(
        tests_dir.join("run.test.cjs"),
        "const runner = require(\"./wrong/run.sh\");\ntesting,\n",
    )
    .unwrap();
    fs::write(
        skill_dir.join("CHANGELOG.md"),
        "# Changelog\n\nNothing yet.\n",
    )
    .unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: scoped-skill
description: Demonstrates scoped scanning and ignored maintenance files.
---
Run `./scripts/run.sh`, then see [the harness](tests/run.test.cjs).
"#,
    )
    .unwrap();

    let config_path = tmp_dir.path().join(".shinyskills.toml");
    fs::write(
        &config_path,
        "[validation]\nignore_files = [\"CHANGELOG.md\"]\nscan_extensions = [\".md\"]\n",
    )
    .unwrap();
    let config = shinyskills::config::load_config(&config_path).unwrap();
    let options =
        shinyskills::config::resolve_validation_options(Some(&config), ValidationMode::Default)
            .unwrap();

    let (warnings, errors) = validate_with_options(&skill_dir, &options);

    assert_eq!(warnings, Vec::<String>::new(), "got: {:?}", warnings);
    assert_eq!(errors, Vec::<String>::new(), "got: {:?}", errors);
}

#[test]
fn test_non_scannable_file_contents_do_not_resolve_orphans() {
    let tmp_dir = TempDir::new().unwrap();
    let skill_dir = tmp_dir.path().join("scan-skill");
    let scripts_dir = skill_dir.join("scripts");
    let assets_dir = skill_dir.join("assets");
    fs::create_dir_all(&scripts_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: scan-skill\ndescription: Verifies scoped content scanning.\n---\nRun `scripts/run.sh`.\n",
    )
    .unwrap();
    fs::write(
        scripts_dir.join("run.sh"),
        "#!/bin/sh\n# assets/orphan.txt\n",
    )
    .unwrap();
    fs::write(assets_dir.join("orphan.txt"), "orphan\n").unwrap();

    let options =
        shinyskills::config::resolve_validation_options(None, ValidationMode::Default).unwrap();
    let (warnings, errors) = validate_with_options(&skill_dir, &options);

    assert!(errors.is_empty(), "errors: {:?}", errors);
    assert_eq!(
        warnings,
        vec!["Orphaned file (not referenced): assets/orphan.txt"]
    );
}

// ---------------------------------------------------------------------------
// Compiled CLI behaviour
// ---------------------------------------------------------------------------

/// Create `<dir>/.git` so the directory is recognised as a repository root.
fn make_git_repo(dir: &std::path::Path) {
    fs::create_dir_all(dir.join(".git")).unwrap();
}

/// Write a skill whose frontmatter carries an extra `user-invocable` field and
/// whose only extra file is a maintenance changelog nothing references.
fn write_vibe_skill(skill_dir: &std::path::Path) {
    fs::create_dir_all(skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: vibe-skill
description: A skill that declares an extra frontmatter field.
user-invocable: true
---
Nothing references the changelog.
"#,
    )
    .unwrap();
    fs::write(skill_dir.join("CHANGELOG.md"), "# Changelog\n").unwrap();
}

fn run_cli(args: &[&std::path::Path]) -> std::process::Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_shinyskills"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().unwrap()
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn cli_validate_uses_repository_root_config_and_ignores_nested() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    make_git_repo(&repo);
    let skill_dir = repo.join("skills").join("vibe-skill");
    write_vibe_skill(&skill_dir);

    fs::write(
        repo.join(".shinyskills.toml"),
        "[validation]\nallowed_fields = [\"user-invocable\"]\nignore_files = [\"CHANGELOG.md\"]\n",
    )
    .unwrap();

    // Conflicting configurations that must never be consulted.
    fs::write(
        repo.join("skills").join(".shinyskills.toml"),
        "[validation]\nallowed_fields = [\"name\", \"description\"]\n",
    )
    .unwrap();
    fs::write(
        skill_dir.join(".shinyskills.toml"),
        "[validation]\nallowed_fields = [\"name\", \"description\"]\n",
    )
    .unwrap();

    let output = run_cli(&[std::path::Path::new("validate"), &skill_dir]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    // With the root configuration removed, the nested files still do not count,
    // so the extra field is rejected again.
    fs::remove_file(repo.join(".shinyskills.toml")).unwrap();
    let output = run_cli(&[std::path::Path::new("validate"), &skill_dir]);
    assert!(!output.status.success());
    assert!(stderr_of(&output).contains("user-invocable"));
}

#[test]
fn cli_validate_config_flag_overrides_discovery() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    make_git_repo(&repo);
    let skill_dir = repo.join("skills").join("vibe-skill");
    write_vibe_skill(&skill_dir);

    let config_path = tmp.path().join("custom.toml");
    fs::write(
        &config_path,
        "[validation]\nallowed_fields = [\"user-invocable\"]\nignore_files = [\"CHANGELOG.md\"]\n",
    )
    .unwrap();

    let output = run_cli(&[
        std::path::Path::new("validate"),
        &skill_dir,
        std::path::Path::new("--config"),
        &config_path,
    ]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).is_empty(),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn cli_validate_reports_missing_and_malformed_config() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    make_git_repo(&repo);
    let skill_dir = repo.join("skills").join("vibe-skill");
    write_vibe_skill(&skill_dir);

    let missing = tmp.path().join("absent.toml");
    let output = run_cli(&[
        std::path::Path::new("validate"),
        &skill_dir,
        std::path::Path::new("--config"),
        &missing,
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("absent.toml"), "stderr: {}", stderr);

    let malformed = tmp.path().join("broken.toml");
    fs::write(&malformed, "not = valid = toml\n").unwrap();
    let output = run_cli(&[
        std::path::Path::new("validate"),
        &skill_dir,
        std::path::Path::new("--config"),
        &malformed,
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("broken.toml"), "stderr: {}", stderr);
}

#[test]
fn cli_validate_staged_rejects_missing_explicit_config_without_files() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("absent.toml");

    let output = run_cli(&[
        std::path::Path::new("validate-staged"),
        std::path::Path::new("--config"),
        &missing,
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("absent.toml"), "stderr: {}", stderr);
}

#[test]
fn cli_validate_warns_about_unreferenced_changelog_without_config() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    make_git_repo(&repo);
    let skill_dir = repo.join("skills").join("vibe-skill");

    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: vibe-skill\ndescription: Plain skill.\n---\nBody text.\n",
    )
    .unwrap();
    fs::write(skill_dir.join("CHANGELOG.md"), "# Changelog\n").unwrap();

    let output = run_cli(&[std::path::Path::new("validate"), &skill_dir]);
    assert!(output.status.success());
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("Orphaned file") && stderr.contains("CHANGELOG.md"),
        "stderr: {}",
        stderr
    );

    // Declaring the ignore removes the warning.
    fs::write(
        repo.join(".shinyskills.toml"),
        "[validation]\nignore_files = [\"CHANGELOG.md\"]\n",
    )
    .unwrap();
    let output = run_cli(&[std::path::Path::new("validate"), &skill_dir]);
    assert!(output.status.success());
    assert!(
        stderr_of(&output).is_empty(),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn cli_validate_staged_resolves_root_config_once_and_ignores_nested() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    make_git_repo(&repo);
    let skill_a = repo.join("skills").join("skill-a");
    let skill_b = repo.join("skills").join("skill-b");
    for dir in [&skill_a, &skill_b] {
        fs::create_dir_all(dir).unwrap();
        let name = dir.file_name().unwrap().to_str().unwrap().to_string();
        fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: A plain skill.\n---\nBody text.\n",
                name
            ),
        )
        .unwrap();
        fs::write(dir.join("CHANGELOG.md"), "# Changelog\n").unwrap();
    }

    let changed_a = skill_a.join("SKILL.md");
    let changed_b = skill_b.join("SKILL.md");

    // No configuration: both skills report the unreferenced changelog.
    let output = run_cli(&[
        std::path::Path::new("validate-staged"),
        &changed_a,
        &changed_b,
    ]);
    assert!(!output.status.success() || !stderr_of(&output).is_empty());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("skill-a"), "stderr: {}", stderr);
    assert!(stderr.contains("skill-b"), "stderr: {}", stderr);

    // Root configuration applies to both skills in one resolution.
    fs::write(
        repo.join(".shinyskills.toml"),
        "[validation]\nignore_files = [\"CHANGELOG.md\"]\n",
    )
    .unwrap();
    let output = run_cli(&[
        std::path::Path::new("validate-staged"),
        &changed_a,
        &changed_b,
    ]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    assert!(
        stderr_of(&output).is_empty(),
        "stderr: {}",
        stderr_of(&output)
    );
}
