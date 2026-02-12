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

    // 1. Access GIDs directly via the struct type (CamelCase)
    println!("Direct GID access:");
    println!("  Movement::GID        = {:#034x}", GameTags::Movement);
    println!("  Combat::GID          = {:#034x}", GameTags::Combat);
    println!();

    // 2. Access children via snake_case module
    println!("Child access via snake_case module:");
    println!("  combat::Attack::GID  = {:#034x}", GameTags::combat::Attack);
    println!("  movement::Running::PATH = {}", GameTags::movement::Running::PATH);
    println!("  movement::Running::DEPTH = {}", GameTags::movement::Running::DEPTH);
    println!();

    // 3. Nested children
    println!("Nested children:");
    println!("  combat::attack::Melee::PATH  = {}", GameTags::combat::attack::Melee::PATH);
    println!("  combat::attack::Melee::DEPTH = {}", GameTags::combat::attack::Melee::DEPTH);
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
    let gid = GameTags::status::Buff;
    if let Some(path) = registry.path_of(gid) {
        println!("  {:#034x} → '{}'", gid, path);
    }
    println!();

    // 7. Use NamespaceTag trait
    println!("Using NamespaceTag trait:");
    print_tag_info::<GameTags::Block>();
    print_tag_info::<GameTags::Jumping>();
}

fn print_tag_info<T: NamespaceTag>() {
    println!("  {} (depth={}, gid={:#034x})", T::PATH, T::DEPTH, T::gid());
}
