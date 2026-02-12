//! Alias system demonstration.
//!
//! This example shows how to:
//! - Use `Alias<T>` for tag aliasing (same GID, different path)
//! - Use `#[deprecated]` to mark old tags with deprecation warnings
//! - Migrate from old paths to new paths while preserving GID stability
//!
//! ## Use Cases
//!
//! 1. **Renaming tags**: Keep the old path as an alias for backward compatibility
//! 2. **Restructuring**: Move tags to a new location while preserving GIDs
//! 3. **Soft deprecation**: Mark old paths as deprecated with helpful messages

use bevy_tag::*;
use bevy_tag_macro::namespace;

// =============================================================================
// Define a namespace with both current and deprecated paths
// =============================================================================

namespace! {
    pub mod Tags {
        // Current canonical paths
        Equipment {
            Weapon {
                Blade;
                Bow;
            }
            Armor {
                Helmet;
                Chestplate;
            }
        }

        Ability {
            Combat {
                Strike;
                Parry;
            }
            Magic {
                Fireball;
                Heal;
            }
        }

        // Deprecated paths (will trigger compiler warnings when used)
        #[deprecated(note = "Use Equipment.Weapon instead")]
        Item {
            #[deprecated(note = "Use Equipment.Weapon.Blade instead")]
            Sword;

            #[deprecated(note = "Use Equipment.Armor instead")]
            Shield;
        }

        #[deprecated(note = "Use Ability.Combat instead")]
        Skill {
            #[deprecated(note = "Use Ability.Combat.Strike instead")]
            Attack;
        }
    }
}

// =============================================================================
// Define type aliases for migration (same GID, different name)
// =============================================================================

/// Alias for backward compatibility: Item.Sword -> Equipment.Weapon.Blade
///
/// Using this alias will NOT trigger deprecation warnings, but both
/// `OldSword` and `Equipment.Weapon.Blade` share the same GID.
#[allow(dead_code)]
pub type OldSword = Alias<Tags::Blade>; // Provide alias from build.rs

/// Another alias example
#[allow(dead_code)]
pub type LegacyAttack = Alias<Tags::Strike>;

fn main() {
    println!("=== Alias System Example ===\n");

    // -------------------------------------------------------------------------
    // 1. Demonstrate that aliases share the same GID
    // -------------------------------------------------------------------------
    println!("1. Alias GID equality:");

    let blade_gid = Tags::equipment::weapon::Blade::GID;
    let alias_gid = OldSword::STABLE_GID;

    println!("   Equipment.Weapon.Blade GID: {:#034x}", blade_gid);
    println!("   OldSword (alias) GID:       {:#034x}", alias_gid);
    println!("   Are they equal? {}", blade_gid == alias_gid);
    println!();

    // -------------------------------------------------------------------------
    // 2. Both can be used interchangeably with IntoGid
    // -------------------------------------------------------------------------
    println!("2. Using aliases with IntoGid trait:");

    fn accept_gid(tag: impl IntoGid) -> GID {
        tag.into_gid()
    }

    let gid1 = accept_gid(Tags::Blade);
    let gid2 = accept_gid(OldSword::new());

    println!("   From canonical tag: {:#034x}", gid1);
    println!("   From alias:         {:#034x}", gid2);
    println!("   Same GID? {}", gid1 == gid2);
    println!();

    // -------------------------------------------------------------------------
    // 3. Deprecated paths (compiler will warn)
    // -------------------------------------------------------------------------
    println!("3. Deprecated paths (see compiler warnings above):");

    // Uncomment these lines to see deprecation warnings:
    // let _ = Tags::item::Sword::GID;  // warning: use Equipment.Weapon.Blade
    // let _ = Tags::skill::Attack::GID; // warning: use Ability.Combat.Strike

    // The deprecated and new paths have DIFFERENT GIDs (they are separate tags)
    #[allow(deprecated)]
    let deprecated_sword = Tags::item::Sword::GID;
    let new_blade = Tags::equipment::weapon::Blade::GID;

    println!(
        "   Item.Sword GID (deprecated):     {:#034x}",
        deprecated_sword
    );
    println!("   Equipment.Weapon.Blade GID:      {:#034x}", new_blade);
    println!(
        "   Same GID? {} (expected: false)",
        deprecated_sword == new_blade
    );
    println!();

    // -------------------------------------------------------------------------
    // 4. Path information
    // -------------------------------------------------------------------------
    println!("4. Path information:");
    println!(
        "   OldSword::PATH:           {}",
        <OldSword as NamespaceTag>::PATH
    );
    println!("   Blade::PATH:              {}", Tags::Blade::PATH);
    println!();

    // -------------------------------------------------------------------------
    // 5. Registry lookup works with both
    // -------------------------------------------------------------------------
    println!("5. Registry compatibility:");

    let registry = NamespaceRegistry::build(Tags::DEFINITIONS).unwrap();

    // Both alias and canonical tag can be used for lookups
    if let Some(path) = registry.path_of(OldSword::new()) {
        println!("   OldSword alias resolves to: '{}'", path);
    }

    // Subtree check works too
    let equipment = Tags::Equipment::GID;
    let blade = Tags::Blade::GID;

    println!(
        "   Is Blade descendant of Equipment? {}",
        registry.is_descendant_of(blade, equipment)
    );
    println!(
        "   Is OldSword descendant of Equipment? {}",
        registry.is_descendant_of(OldSword::new(), equipment)
    );
    println!();

    // -------------------------------------------------------------------------
    // 6. Summary of approaches
    // -------------------------------------------------------------------------
    println!("=== Summary ===");
    println!();
    println!("Use `#[deprecated]` when:");
    println!("  - You want compiler warnings for old path usage");
    println!("  - The old and new paths are completely different tags");
    println!("  - You're phasing out a tag entirely");
    println!();
    println!("Use `Alias<T>` when:");
    println!("  - You need the SAME GID for backward compatibility");
    println!("  - Old serialized data must still work (GID matches)");
    println!("  - You're renaming but want zero runtime cost");
}
