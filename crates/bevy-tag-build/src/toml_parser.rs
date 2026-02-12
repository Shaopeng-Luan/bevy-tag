//! TOML configuration parser for tags.toml.

use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

/// Behavior when a path is removed from config but exists in lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnRemove {
    /// Emit a compile error (default, safest)
    #[default]
    Error,
    /// Emit a warning via #[deprecated], but allow compilation
    Warn,
}

/// Parsed tags configuration.
#[derive(Debug, Clone)]
pub struct TagsConfig {
    /// Module name for the generated namespace
    pub module_name: String,
    /// Behavior when paths are removed
    pub on_remove: OnRemove,
    /// All tag entries (including auto-generated parents)
    entries: Vec<TagEntry>,
}

/// A single tag entry with computed properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEntry {
    /// Full dot-separated path (e.g., "Item.Weapon.Sword")
    pub path: String,
    /// Tree depth (0 = root)
    pub depth: u8,
    /// Parent path (None for root nodes)
    pub parent: Option<String>,
}

/// Raw TOML structure.
#[derive(Debug, Deserialize)]
struct RawTagsConfig {
    /// Optional module name (defaults to "Tags")
    module_name: Option<String>,
    /// Behavior when paths are removed: "error" (default) or "warn"
    on_remove: Option<String>,
    /// Tag definitions
    tags: RawTags,
}

#[derive(Debug, Deserialize)]
struct RawTags {
    /// List of dot-separated paths
    paths: Vec<String>,
}

impl TagsConfig {
    /// Parse from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TagsConfigError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            TagsConfigError::Io(format!("Failed to read {}: {}", path.as_ref().display(), e))
        })?;
        Self::from_str(&content)
    }

    /// Parse from a TOML string.
    pub fn from_str(content: &str) -> Result<Self, TagsConfigError> {
        let raw: RawTagsConfig =
            toml::from_str(content).map_err(|e| TagsConfigError::Parse(e.to_string()))?;

        let module_name = raw.module_name.unwrap_or_else(|| "Tags".to_string());

        // Parse on_remove strategy
        let on_remove = match raw.on_remove.as_deref() {
            None | Some("error") => OnRemove::Error,
            Some("warn") => OnRemove::Warn,
            Some(other) => {
                return Err(TagsConfigError::Validation(format!(
                    "Invalid on_remove value '{}': expected 'error' or 'warn'",
                    other
                )));
            }
        };

        // Validate and expand paths
        let entries = Self::expand_paths(&raw.tags.paths)?;

        Ok(Self {
            module_name,
            on_remove,
            entries,
        })
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &TagEntry> {
        self.entries.iter()
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Expand paths to include all parent nodes.
    ///
    /// e.g., "A.B.C" expands to ["A", "A.B", "A.B.C"]
    fn expand_paths(paths: &[String]) -> Result<Vec<TagEntry>, TagsConfigError> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut entries: Vec<TagEntry> = Vec::new();

        for path in paths {
            // Validate path
            if path.is_empty() {
                return Err(TagsConfigError::Validation("Empty path not allowed".into()));
            }
            if path.starts_with('.') || path.ends_with('.') {
                return Err(TagsConfigError::Validation(format!(
                    "Invalid path '{}': cannot start or end with '.'",
                    path
                )));
            }
            if path.contains("..") {
                return Err(TagsConfigError::Validation(format!(
                    "Invalid path '{}': contains '..'",
                    path
                )));
            }

            // Split into segments
            let segments: Vec<&str> = path.split('.').collect();

            // Validate segments
            for seg in &segments {
                if seg.is_empty() {
                    return Err(TagsConfigError::Validation(format!(
                        "Invalid path '{}': empty segment",
                        path
                    )));
                }
                // Check valid identifier (starts with letter/underscore, contains alphanumeric/_)
                let mut chars = seg.chars();
                if let Some(first) = chars.next() {
                    if !first.is_alphabetic() && first != '_' {
                        return Err(TagsConfigError::Validation(format!(
                            "Invalid path '{}': segment '{}' must start with letter or underscore",
                            path, seg
                        )));
                    }
                }
                for c in chars {
                    if !c.is_alphanumeric() && c != '_' {
                        return Err(TagsConfigError::Validation(format!(
                            "Invalid path '{}': segment '{}' contains invalid character '{}'",
                            path, seg, c
                        )));
                    }
                }
            }

            // Add all ancestors and the path itself
            for depth in 0..segments.len() {
                let ancestor_path = segments[..=depth].join(".");
                if seen.insert(ancestor_path.clone()) {
                    let parent = if depth == 0 {
                        None
                    } else {
                        Some(segments[..depth].join("."))
                    };
                    entries.push(TagEntry {
                        path: ancestor_path,
                        depth: depth as u8,
                        parent,
                    });
                }
            }
        }

        // Sort by path for deterministic output
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(entries)
    }
}

/// Errors during config parsing.
#[derive(Debug)]
pub enum TagsConfigError {
    /// IO error
    Io(String),
    /// TOML parse error
    Parse(String),
    /// Validation error
    Validation(String),
}

impl std::fmt::Display for TagsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for TagsConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_config() {
        let toml = r#"
[tags]
paths = [
    "Item.Weapon.Sword",
    "Item.Weapon.Axe",
    "Skill.Combat",
]
"#;
        let config = TagsConfig::from_str(toml).unwrap();

        assert_eq!(config.module_name, "Tags");
        assert_eq!(config.len(), 6); // Item, Item.Weapon, Item.Weapon.Sword, Item.Weapon.Axe, Skill, Skill.Combat

        let paths: Vec<_> = config.entries().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"Item"));
        assert!(paths.contains(&"Item.Weapon"));
        assert!(paths.contains(&"Item.Weapon.Sword"));
        assert!(paths.contains(&"Item.Weapon.Axe"));
        assert!(paths.contains(&"Skill"));
        assert!(paths.contains(&"Skill.Combat"));
    }

    #[test]
    fn parse_with_module_name() {
        let toml = r#"
module_name = "GameTags"

[tags]
paths = ["A.B"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();
        assert_eq!(config.module_name, "GameTags");
    }

    #[test]
    fn expand_creates_parents() {
        let toml = r#"
[tags]
paths = ["A.B.C.D"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();

        let entries: Vec<_> = config.entries().collect();
        assert_eq!(entries.len(), 4);

        assert_eq!(entries[0].path, "A");
        assert_eq!(entries[0].depth, 0);
        assert_eq!(entries[0].parent, None);

        assert_eq!(entries[1].path, "A.B");
        assert_eq!(entries[1].depth, 1);
        assert_eq!(entries[1].parent, Some("A".into()));

        assert_eq!(entries[2].path, "A.B.C");
        assert_eq!(entries[2].depth, 2);
        assert_eq!(entries[2].parent, Some("A.B".into()));

        assert_eq!(entries[3].path, "A.B.C.D");
        assert_eq!(entries[3].depth, 3);
        assert_eq!(entries[3].parent, Some("A.B.C".into()));
    }

    #[test]
    fn deduplicates_parents() {
        let toml = r#"
[tags]
paths = ["A.B.C", "A.B.D", "A.X"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();

        // A, A.B, A.B.C, A.B.D, A.X = 5 entries (A and A.B not duplicated)
        assert_eq!(config.len(), 5);
    }

    #[test]
    fn rejects_empty_path() {
        let toml = r#"
[tags]
paths = [""]
"#;
        assert!(TagsConfig::from_str(toml).is_err());
    }

    #[test]
    fn rejects_invalid_path() {
        let cases = [
            ".A",        // starts with dot
            "A.",        // ends with dot
            "A..B",      // double dot
            "A.1B",      // segment starts with number
            "A.B-C",     // contains hyphen
            "A.B C",     // contains space
        ];

        for case in cases {
            let toml = format!(
                r#"
[tags]
paths = ["{}"]
"#,
                case
            );
            assert!(
                TagsConfig::from_str(&toml).is_err(),
                "Should reject: {}",
                case
            );
        }
    }

    #[test]
    fn accepts_valid_identifiers() {
        let toml = r#"
[tags]
paths = ["_Private.Item", "CamelCase.snake_case", "With123Numbers"]
"#;
        assert!(TagsConfig::from_str(toml).is_ok());
    }

    #[test]
    fn on_remove_defaults_to_error() {
        let toml = r#"
[tags]
paths = ["A"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();
        assert_eq!(config.on_remove, OnRemove::Error);
    }

    #[test]
    fn on_remove_explicit_error() {
        let toml = r#"
on_remove = "error"

[tags]
paths = ["A"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();
        assert_eq!(config.on_remove, OnRemove::Error);
    }

    #[test]
    fn on_remove_warn() {
        let toml = r#"
on_remove = "warn"

[tags]
paths = ["A"]
"#;
        let config = TagsConfig::from_str(toml).unwrap();
        assert_eq!(config.on_remove, OnRemove::Warn);
    }

    #[test]
    fn on_remove_invalid_value() {
        let toml = r#"
on_remove = "invalid"

[tags]
paths = ["A"]
"#;
        let result = TagsConfig::from_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }
}
