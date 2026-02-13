//! Basic namespace definition and GID lookup.
//!
//! This example shows how to:
//! - Define a namespace hierarchy with the `namespace!` macro
//! - Access GIDs (Global Identifiers) for each tag
//! - Perform path ↔ GID lookups

use bevy_tag::*;
use bevy_tag_macro::namespace;

// Define a gameplay tag hierarchy
namespace! {
    pub mod GameTags {
        Movement {
            Idle;
            Walking;
            Running;
            Jumping;
        }
        Combat {
            Attack {
                Melee;
                Ranged;
            }
            Defend {
                Block;
                Dodge;
            }
        }
        Status {
            Buff;
            Debuff;
        }
    }
}

fn main() {
    println!("=== Basic Namespace Example ===\n");

    // 1. Access GIDs via module-level constants
    println!("Direct GID access:");
    println!("  Movement::GID        = {:#034x}", GameTags::Movement::GID);
    println!("  Combat::GID          = {:#034x}", GameTags::Combat::GID);
    println!();

    // 2. Access children via nested modules (CamelCase)
    println!("Child access via nested modules:");
    println!("  Combat::Attack::GID  = {:#034x}", GameTags::Combat::Attack::GID);
    println!("  Movement::Running::PATH = {}", GameTags::Movement::Running::PATH);
    println!("  Movement::Running::DEPTH = {}", GameTags::Movement::Running::DEPTH);
    println!();

    // 3. Deeply nested children
    println!("Deeply nested children:");
    println!("  Combat::Attack::Melee::PATH  = {}", GameTags::Combat::Attack::Melee::PATH);
    println!("  Combat::Attack::Melee::DEPTH = {}", GameTags::Combat::Attack::Melee::DEPTH);
    println!();

    // 4. Build a registry for runtime lookups
    let registry = NamespaceRegistry::build(GameTags::DEFINITIONS).unwrap();

    println!("Registry info:");
    println!("  Total nodes: {}", registry.len());
    println!("  Tree depth:  {}", GameTags::TREE_DEPTH);
    println!();

    // 5. Path → GID lookup
    println!("Path → GID lookup:");
    if let Some(gid) = registry.gid_of("Combat.Attack.Melee") {
        println!("  'Combat.Attack.Melee' → {:#034x}", gid);
    }
    println!();

    // 6. GID → Path lookup
    println!("GID → Path lookup:");
    let gid = GameTags::Status::Buff::GID;
    if let Some(path) = registry.path_of(gid) {
        println!("  {:#034x} → '{}'", gid, path);
    }
    println!();

    // 7. Use NamespaceTag trait with Tag types
    println!("Using NamespaceTag trait:");
    print_tag_info::<GameTags::Combat::Defend::Block::Tag>();
    print_tag_info::<GameTags::Movement::Jumping::Tag>();
}

fn print_tag_info<T: NamespaceTag>() {
    println!("  {} (depth={}, gid={:#034x})", T::PATH, T::DEPTH, T::gid());
}
