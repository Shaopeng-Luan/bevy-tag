//! Integration tests for bevy-tag-build.

use bevy_tag_build::{generate_with_lock, GenerateError, LockFile};
use std::fs;
use tempfile::TempDir;

/// Create a temp directory with tags.toml
fn setup_config(paths: &[&str]) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("tags.toml");

    let paths_str = paths
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(", ");

    let content = format!(
        r#"
[tags]
paths = [{}]
"#,
        paths_str
    );

    fs::write(&config_path, content).unwrap();
    (dir, config_path)
}

#[test]
fn first_build_creates_lock_file() {
    let (dir, config_path) = setup_config(&["Item.Weapon.Sword", "Skill.Combat"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build - should create lock file
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Lock file should exist
    assert!(lock_path.exists());

    // Output file should exist
    assert!(output_path.exists());

    // Lock file should contain all entries
    let lock = LockFile::from_file(&lock_path).unwrap();
    assert!(lock.get("Item").is_some());
    assert!(lock.get("Item.Weapon").is_some());
    assert!(lock.get("Item.Weapon.Sword").is_some());
    assert!(lock.get("Skill").is_some());
    assert!(lock.get("Skill.Combat").is_some());
}

#[test]
fn adding_paths_updates_lock() {
    let (dir, config_path) = setup_config(&["Item.Weapon"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    let lock_v1 = LockFile::from_file(&lock_path).unwrap();
    assert_eq!(lock_v1.entries.len(), 2); // Item, Item.Weapon

    // Update config with new path
    fs::write(
        &config_path,
        r#"
[tags]
paths = ["Item.Weapon", "Skill.Combat"]
"#,
    )
    .unwrap();

    // Second build - should add new entries
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    let lock_v2 = LockFile::from_file(&lock_path).unwrap();
    assert_eq!(lock_v2.entries.len(), 4); // Item, Item.Weapon, Skill, Skill.Combat
    assert!(lock_v2.get("Skill").is_some());
    assert!(lock_v2.get("Skill.Combat").is_some());
}

#[test]
fn removing_paths_causes_error() {
    let (dir, config_path) = setup_config(&["Item.Weapon", "Skill.Combat"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Update config - remove Skill.Combat
    fs::write(
        &config_path,
        r#"
[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Second build - should fail
    let result = generate_with_lock(&config_path, &lock_path, &output_path);

    assert!(result.is_err());
    match result.unwrap_err() {
        GenerateError::LockMismatch(msg) => {
            assert!(msg.contains("Skill"), "Error should mention removed path");
            assert!(msg.contains("Skill.Combat"), "Error should mention removed path");
        }
        other => panic!("Expected LockMismatch, got: {:?}", other),
    }
}

#[test]
fn generated_code_has_correct_structure() {
    let (dir, config_path) = setup_config(&["Item.Weapon.Sword", "Item.Armor.Helmet"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    let code = fs::read_to_string(&output_path).unwrap();

    // Check structure
    assert!(code.contains("namespace!"));
    assert!(code.contains("pub mod Tags"));
    assert!(code.contains("Item {"));
    assert!(code.contains("Weapon {"));
    assert!(code.contains("Sword;"));
    assert!(code.contains("Armor {"));
    assert!(code.contains("Helmet;"));
}

#[test]
fn unchanged_config_works() {
    let (dir, config_path) = setup_config(&["Item.Weapon"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Second build with same config - should work
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Third build - still works
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();
}

#[test]
fn deleting_lock_allows_breaking_change() {
    let (dir, config_path) = setup_config(&["Item.Weapon", "Skill.Combat"]);
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Remove Skill.Combat from config
    fs::write(
        &config_path,
        r#"
[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Delete lock file (intentional breaking change)
    fs::remove_file(&lock_path).unwrap();

    // Now build should succeed
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // New lock should only have remaining paths
    let lock = LockFile::from_file(&lock_path).unwrap();
    assert!(lock.get("Item").is_some());
    assert!(lock.get("Item.Weapon").is_some());
    assert!(lock.get("Skill").is_none());
    assert!(lock.get("Skill.Combat").is_none());
}

/// Helper to create config with on_remove option
fn setup_config_with_on_remove(paths: &[&str], on_remove: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("tags.toml");

    let paths_str = paths
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(", ");

    let content = format!(
        r#"
on_remove = "{}"

[tags]
paths = [{}]
"#,
        on_remove, paths_str
    );

    fs::write(&config_path, content).unwrap();
    (dir, config_path)
}

#[test]
fn warn_mode_allows_removed_paths() {
    let (dir, config_path) = setup_config_with_on_remove(&["Item.Weapon", "Skill.Combat"], "warn");
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Update config - remove Skill.Combat (with warn mode)
    fs::write(
        &config_path,
        r#"
on_remove = "warn"

[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Second build - should succeed (warn mode)
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Lock file should still have entries, but Skill paths marked deprecated
    let lock = LockFile::from_file(&lock_path).unwrap();
    assert!(lock.get("Item").is_some());
    assert!(lock.get("Item.Weapon").is_some());

    // Removed paths should be deprecated
    let skill = lock.get("Skill").unwrap();
    assert!(skill.deprecated, "Skill should be deprecated");
    let skill_combat = lock.get("Skill.Combat").unwrap();
    assert!(skill_combat.deprecated, "Skill.Combat should be deprecated");
}

#[test]
fn warn_mode_generates_deprecated_attributes() {
    let (dir, config_path) = setup_config_with_on_remove(&["Item.Weapon", "Skill.Combat"], "warn");
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Update config - remove Skill.Combat
    fs::write(
        &config_path,
        r#"
on_remove = "warn"

[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Second build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Check generated code has #[deprecated] for removed paths
    let code = fs::read_to_string(&output_path).unwrap();

    println!("{}", code);

    // Active paths should NOT be deprecated (Item, Weapon)
    // We check that there's no deprecated before the active nodes
    assert!(code.contains("Item {"));
    assert!(code.contains("Weapon;"));

    // Removed paths should have #[deprecated(note = "...")]
    assert!(code.contains("#[deprecated(note = \"This tag is deprecated.\")]"));

    // Skill and Combat nodes should exist (deprecated)
    assert!(code.contains("Skill {"));
    assert!(code.contains("Combat;"));
}

#[test]
fn warn_mode_preserves_deprecated_on_rebuild() {
    let (dir, config_path) = setup_config_with_on_remove(&["Item.Weapon", "Skill.Combat"], "warn");
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Remove Skill.Combat
    fs::write(
        &config_path,
        r#"
on_remove = "warn"

[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Second build - marks as deprecated
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Third build - deprecated should persist
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    let lock = LockFile::from_file(&lock_path).unwrap();
    let skill = lock.get("Skill").unwrap();
    assert!(skill.deprecated, "Skill should still be deprecated after rebuild");
}

#[test]
fn deprecated_entries_iterator_works() {
    let (dir, config_path) = setup_config_with_on_remove(&["Item.Weapon", "Skill.Combat"], "warn");
    let lock_path = dir.path().join("tags.lock.toml");
    let output_path = dir.path().join("generated.rs");

    // First build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    // Remove Skill.Combat
    fs::write(
        &config_path,
        r#"
on_remove = "warn"

[tags]
paths = ["Item.Weapon"]
"#,
    )
    .unwrap();

    // Second build
    generate_with_lock(&config_path, &lock_path, &output_path).unwrap();

    let lock = LockFile::from_file(&lock_path).unwrap();

    // Check deprecated_entries iterator
    let deprecated: Vec<_> = lock.deprecated_entries().map(|e| e.path.as_str()).collect();
    assert!(deprecated.contains(&"Skill"));
    assert!(deprecated.contains(&"Skill.Combat"));
    assert!(!deprecated.contains(&"Item"));
    assert!(!deprecated.contains(&"Item.Weapon"));

    // Check active_entries iterator
    let active: Vec<_> = lock.active_entries().map(|e| e.path.as_str()).collect();
    assert!(active.contains(&"Item"));
    assert!(active.contains(&"Item.Weapon"));
    assert!(!active.contains(&"Skill"));
    assert!(!active.contains(&"Skill.Combat"));
}
