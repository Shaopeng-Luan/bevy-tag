//! Bevy integration example.
//!
//! This example shows how to:
//! - Set up the `NamespacePlugin` with tag definitions
//! - Use `TagContainer` component for entity tags
//! - Query entities by their tags
//! - Check subtree membership in systems

use bevy::prelude::*;
use bevy_tag::bevy::{NamespacePlugin, TagContainer};
use bevy_tag::{gid_is_descendant_of, NamespaceRegistry};
use bevy_tag_macro::namespace;

// Define gameplay tags
namespace! {
    pub mod Tags {
        Movement {
            Idle;
            Walking;
            Running;
        }
        Combat {
            Attack;
            Defend;
        }
        Status {
            Poisoned;
            Burning;
            Frozen;
        }
    }
}

// Components for our entities
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Name(&'static str);

fn main() {
    App::new()
        // Use MinimalPlugins for this example (no window)
        .add_plugins(MinimalPlugins)
        // Initialize the namespace registry as a Resource
        .add_plugins(NamespacePlugin::from_definitions(Tags::DEFINITIONS))
        // Systems
        .add_systems(Startup, spawn_entities)
        .add_systems(
            Update,
            (
                print_all_tags,
                check_movement_tags,
                check_status_effects,
            )
                .chain(),
        )
        .run();
}

fn spawn_entities(mut commands: Commands) {
    println!("=== Bevy Integration Example ===\n");
    println!("Spawning entities...\n");

    // Player with movement tag
    commands.spawn((
        Player,
        Name("Hero"),
        TagContainer::single(Tags::Movement::Running::GID),
    ));

    // Enemy with multiple tags using TagContainer
    commands.spawn((
        Enemy,
        Name("Goblin"),
        TagContainer::new()
            .with(Tags::Movement::Idle::GID)
            .with(Tags::Combat::Attack::GID),
    ));

    // Another enemy with status effects
    commands.spawn((
        Enemy,
        Name("Skeleton"),
        TagContainer::new()
            .with(Tags::Movement::Walking::GID)
            .with(Tags::Status::Poisoned::GID)
            .with(Tags::Status::Burning::GID),
    ));

    // Entity with only status effects (no movement)
    commands.spawn((
        Enemy,
        Name("Ghost"),
        TagContainer::single(Tags::Status::Frozen::GID),
    ));
}

fn print_all_tags(
    registry: Res<NamespaceRegistry>,
    query: Query<(&Name, &TagContainer)>,
) {
    println!("--- All Entity Tags ---");

    for (name, container) in query.iter() {
        let paths: Vec<_> = container
            .iter()
            .filter_map(|gid| registry.path_of(gid))
            .collect();
        println!("  {} has tags: {:?}", name.0, paths);
    }
    println!();
}

fn check_movement_tags(query: Query<(&Name, &TagContainer)>) {
    println!("--- Movement Check (O(1) subtree test) ---");

    for (name, container) in query.iter() {
        if container.has_descendant_of(Tags::Movement::GID) {
            println!("  {} is moving", name.0);
        } else {
            println!("  {} is NOT moving", name.0);
        }
    }
    println!();
}

fn check_status_effects(
    registry: Res<NamespaceRegistry>,
    query: Query<(&Name, &TagContainer)>,
) {
    println!("--- Status Effects ---");

    for (name, container) in query.iter() {
        // Check if entity has any status effect
        if container.has_descendant_of(Tags::Status::GID) {
            let effects: Vec<_> = container
                .descendants_of(Tags::Status::GID)
                .filter_map(|gid| registry.path_of(gid))
                .collect();
            println!("  {} has status effects: {:?}", name.0, effects);
        }

        // Check specific status
        if container.has(Tags::Status::Poisoned::GID) {
            println!("    -> {} is poisoned!", name.0);
        }
        if container.has(Tags::Status::Burning::GID) {
            println!("    -> {} is burning!", name.0);
        }
        if container.has(Tags::Status::Frozen::GID) {
            println!("    -> {} is frozen!", name.0);
        }
    }
    println!();

    // Demonstrate standalone GID check (no registry needed)
    println!("--- Standalone GID Check (no registry) ---");
    let running_gid = Tags::Movement::Running::GID;
    let movement_gid = Tags::Movement::GID;
    let combat_gid = Tags::Combat::GID;

    println!(
        "  Is Running under Movement? {}",
        gid_is_descendant_of(running_gid, movement_gid)
    );
    println!(
        "  Is Running under Combat? {}",
        gid_is_descendant_of(running_gid, combat_gid)
    );
    println!();

    println!("=== Example Complete ===");
    std::process::exit(0);
}
