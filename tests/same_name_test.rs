//! Test for same-name children under different parents.
//!
//! This was a design flaw that has been fixed by nesting children
//! inside their parent's module.

use bevy_tag::*;
use bevy_tag_macro::namespace;

// Define namespace with same-name children under different parents
namespace! {
    pub mod Tags {
        Combat {
            Attack;
            Idle;
        }
        Movement {
            Attack;  // Same name as Combat.Attack - should NOT conflict!
            Idle;    // Same name as Combat.Idle - should NOT conflict!
        }
        Status {
            Active {
                Running;
            }
            Passive {
                Running;  // Same name as Status.Active.Running
            }
        }
    }
}

#[test]
fn test_same_name_different_parents_compile() {
    // This test verifies that same-name children under different parents
    // can coexist without compilation errors.

    // Access via nested module paths (CamelCase)
    let combat_attack = Tags::Combat::Attack::GID;
    let movement_attack = Tags::Movement::Attack::GID;

    let combat_idle = Tags::Combat::Idle::GID;
    let movement_idle = Tags::Movement::Idle::GID;

    // All GIDs should be unique
    assert_ne!(combat_attack, movement_attack);
    assert_ne!(combat_idle, movement_idle);
    assert_ne!(combat_attack, combat_idle);
    assert_ne!(movement_attack, movement_idle);
}

#[test]
fn test_same_name_correct_paths() {
    // Verify paths are correct
    assert_eq!(Tags::Combat::Attack::PATH, "Combat.Attack");
    assert_eq!(Tags::Movement::Attack::PATH, "Movement.Attack");

    assert_eq!(Tags::Combat::Idle::PATH, "Combat.Idle");
    assert_eq!(Tags::Movement::Idle::PATH, "Movement.Idle");
}

#[test]
fn test_same_name_correct_depths() {
    // Verify depths are correct
    assert_eq!(Tags::Combat::Attack::DEPTH, 1);
    assert_eq!(Tags::Movement::Attack::DEPTH, 1);

    assert_eq!(Tags::Combat::DEPTH, 0);
    assert_eq!(Tags::Movement::DEPTH, 0);
}

#[test]
fn test_deeply_nested_same_names() {
    // Test same names at deeper nesting levels
    let active_running = Tags::Status::Active::Running::GID;
    let passive_running = Tags::Status::Passive::Running::GID;

    assert_ne!(active_running, passive_running);

    assert_eq!(Tags::Status::Active::Running::PATH, "Status.Active.Running");
    assert_eq!(Tags::Status::Passive::Running::PATH, "Status.Passive.Running");

    assert_eq!(Tags::Status::Active::Running::DEPTH, 2);
    assert_eq!(Tags::Status::Passive::Running::DEPTH, 2);
}

#[test]
fn test_hierarchy_check_same_names() {
    // Verify hierarchy checks work correctly with same names
    let combat = Tags::Combat::GID;
    let movement = Tags::Movement::GID;
    let combat_attack = Tags::Combat::Attack::GID;
    let movement_attack = Tags::Movement::Attack::GID;

    // Combat.Attack is descendant of Combat
    assert!(gid_is_descendant_of(combat_attack, combat));

    // Movement.Attack is descendant of Movement
    assert!(gid_is_descendant_of(movement_attack, movement));

    // Combat.Attack is NOT descendant of Movement
    assert!(!gid_is_descendant_of(combat_attack, movement));

    // Movement.Attack is NOT descendant of Combat
    assert!(!gid_is_descendant_of(movement_attack, combat));
}

#[test]
fn test_registry_with_same_names() {
    // Verify registry works correctly with same names
    let registry = NamespaceRegistry::build(Tags::DEFINITIONS).unwrap();

    // All paths should be resolvable
    assert!(registry.gid_of("Combat.Attack").is_some());
    assert!(registry.gid_of("Movement.Attack").is_some());
    assert!(registry.gid_of("Combat.Idle").is_some());
    assert!(registry.gid_of("Movement.Idle").is_some());

    // GIDs should match
    assert_eq!(
        registry.gid_of("Combat.Attack"),
        Some(Tags::Combat::Attack::GID)
    );
    assert_eq!(
        registry.gid_of("Movement.Attack"),
        Some(Tags::Movement::Attack::GID)
    );
}

#[test]
fn test_tag_type_access() {
    // Test accessing the Tag type
    fn requires_namespace_tag<T: NamespaceTag>() -> &'static str {
        T::PATH
    }

    assert_eq!(requires_namespace_tag::<Tags::Combat::Tag>(), "Combat");
    assert_eq!(requires_namespace_tag::<Tags::Combat::Attack::Tag>(), "Combat.Attack");
    assert_eq!(requires_namespace_tag::<Tags::Movement::Tag>(), "Movement");
    assert_eq!(requires_namespace_tag::<Tags::Movement::Attack::Tag>(), "Movement.Attack");
}
