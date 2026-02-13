//! Tag metadata: static (compile-time) and dynamic (runtime).
//!
//! This example shows two approaches to attach data to tags:
//! 1. **Static metadata** - `#[key = value]` attributes in macro, accessed via `Tag::KEY`
//! 2. **Dynamic metadata** - Runtime `set_meta`/`get_meta` with zerocopy typed access

use bevy_tag::NamespaceRegistry;
use bevy_tag_macro::namespace;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// =============================================================================
// Static Metadata (compile-time constants via macro attributes)
// =============================================================================

namespace! {
    pub mod Abilities {
        // Static metadata defined with #[key = value] syntax
        #[mana_cost = 10]
        #[cooldown = 1.5]
        Fireball;

        #[mana_cost = 25]
        #[cooldown = 3.0]
        #[is_ultimate = true]
        MeteorStrike;

        #[mana_cost = 5]
        #[cooldown = 0.5]
        IceShard;
    }
}

// =============================================================================
// Custom struct for dynamic metadata (must derive zerocopy traits)
// =============================================================================

#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
struct AbilityStats {
    damage: i32,
    range: f32,
    aoe_radius: f32,
}

fn main() {
    println!("=== Metadata Example ===\n");

    // =========================================================================
    // Part 1: Static Metadata (compile-time)
    // =========================================================================

    println!("--- Static Metadata (compile-time constants) ---\n");

    // Access static metadata directly on Tag struct
    println!("Fireball:");
    println!("  Mana cost: {}", Abilities::Fireball::Tag::MANA_COST);
    println!("  Cooldown:  {}s", Abilities::Fireball::Tag::COOLDOWN);
    println!();

    println!("MeteorStrike:");
    println!("  Mana cost:   {}", Abilities::MeteorStrike::Tag::MANA_COST);
    println!("  Cooldown:    {}s", Abilities::MeteorStrike::Tag::COOLDOWN);
    println!("  Is ultimate: {}", Abilities::MeteorStrike::Tag::IS_ULTIMATE);
    println!();

    println!("IceShard:");
    println!("  Mana cost: {}", Abilities::IceShard::Tag::MANA_COST);
    println!("  Cooldown:  {}s", Abilities::IceShard::Tag::COOLDOWN);
    println!();

    // =========================================================================
    // Part 2: Dynamic Metadata (runtime)
    // =========================================================================

    println!("--- Dynamic Metadata (runtime zerocopy) ---\n");

    let mut registry = NamespaceRegistry::build(Abilities::DEFINITIONS).unwrap();

    // Set primitive typed metadata - use module-level GID const
    registry.set_meta(Abilities::Fireball::GID, "damage", &100i32);
    registry.set_meta(Abilities::Fireball::GID, "range", &15.0f32);

    // Get primitive typed metadata
    let damage = registry.get_meta::<i32>(Abilities::Fireball::GID, "damage");
    let range = registry.get_meta::<f32>(Abilities::Fireball::GID, "range");
    println!("Fireball (primitives):");
    println!("  Damage: {:?}", damage);
    println!("  Range:  {:?}", range);
    println!();

    // Set custom struct metadata
    let fireball_stats = AbilityStats {
        damage: 100,
        range: 15.0,
        aoe_radius: 3.0,
    };
    registry.set_meta(Abilities::Fireball::GID, "stats", &fireball_stats);

    let meteor_stats = AbilityStats {
        damage: 500,
        range: 30.0,
        aoe_radius: 10.0,
    };
    registry.set_meta(Abilities::MeteorStrike::GID, "stats", &meteor_stats);

    // Get custom struct metadata
    if let Some(stats) = registry.get_meta::<AbilityStats>(Abilities::Fireball::GID, "stats") {
        println!("Fireball (struct):");
        println!("  Damage:     {}", stats.damage);
        println!("  Range:      {}", stats.range);
        println!("  AoE radius: {}", stats.aoe_radius);
    }
    println!();

    if let Some(stats) = registry.get_meta::<AbilityStats>(Abilities::MeteorStrike::GID, "stats") {
        println!("MeteorStrike (struct):");
        println!("  Damage:     {}", stats.damage);
        println!("  Range:      {}", stats.range);
        println!("  AoE radius: {}", stats.aoe_radius);
    }
    println!();

    // =========================================================================
    // Part 3: Metadata utilities
    // =========================================================================

    println!("--- Metadata Utilities ---\n");

    // Check if metadata exists
    println!(
        "Fireball has 'damage': {}",
        registry.has_meta(Abilities::Fireball::GID, "damage")
    );
    println!(
        "IceShard has 'damage': {}",
        registry.has_meta(Abilities::IceShard::GID, "damage")
    );
    println!();

    // List all metadata keys
    if let Some(keys) = registry.meta_keys(Abilities::Fireball::GID) {
        let keys: Vec<_> = keys.collect();
        println!("Fireball metadata keys: {:?}", keys);
    }
    println!();

    // Overwrite metadata
    println!("Updating Fireball damage from 100 to 150...");
    registry.set_meta(Abilities::Fireball::GID, "damage", &150i32);
    let new_damage = registry.get_meta::<i32>(Abilities::Fireball::GID, "damage");
    println!("New damage: {:?}", new_damage);
    println!();

    // Remove metadata
    println!("Removing Fireball 'range' metadata...");
    let removed = registry.remove_meta(Abilities::Fireball::GID, "range");
    println!("Removed: {}", removed.is_some());
    println!(
        "Fireball has 'range': {}",
        registry.has_meta(Abilities::Fireball::GID, "range")
    );
    println!();

    // =========================================================================
    // Part 4: Type safety demo
    // =========================================================================

    println!("--- Type Safety ---\n");

    // Wrong type returns None (size mismatch)
    let wrong_type = registry.get_meta::<u64>(Abilities::Fireball::GID, "damage");
    println!(
        "Reading i32 damage as u64: {:?} (None = type mismatch)",
        wrong_type
    );

    // Correct type works
    let correct_type = registry.get_meta::<i32>(Abilities::Fireball::GID, "damage");
    println!("Reading i32 damage as i32: {:?}", correct_type);
}
