use bevy_tag::*;
use bevy_tag_macro::namespace;
use serde::{Deserialize, Serialize};

// Define data types BEFORE the namespace macro
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbilityData {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MovementData {
    pub speed_multiplier: f32,
    pub stamina_cost: f32,
}

// Define namespace with metadata and data types
// Note: Types must be in scope when the macro expands
namespace! {
    pub mod GameplayTags {
        // Node with constant metadata
        #[damage = 50]
        #[cost = 10]
        #[range = 5.0]
        BasicAttack;

        // Node with both metadata and data type
        #[damage = 100]
        #[cooldown = 2.5]
        HeavyAttack<crate::AbilityData>;

        // Category with children
        Movement {
            #[speed_multiplier = 1.5]
            #[stamina_drain = 2.0]
            Sprint;

            #[speed_multiplier = 2.0]
            #[duration = 0.3]
            Dash<crate::MovementData>;
        }

        // Node with only data type
        Status<crate::AbilityData>;
    }
}

#[test]
fn test_constant_metadata() {
    // Access const metadata
    assert_eq!(GameplayTags::BasicAttack::DAMAGE, 50);
    assert_eq!(GameplayTags::BasicAttack::COST, 10);
    assert_eq!(GameplayTags::BasicAttack::RANGE, 5.0);

    assert_eq!(GameplayTags::HeavyAttack::DAMAGE, 100);
    assert_eq!(GameplayTags::HeavyAttack::COOLDOWN, 2.5);

    // Access via snake_case module
    assert_eq!(GameplayTags::movement::Sprint::SPEED_MULTIPLIER, 1.5);
    assert_eq!(GameplayTags::movement::Sprint::STAMINA_DRAIN, 2.0);

    assert_eq!(GameplayTags::movement::Dash::SPEED_MULTIPLIER, 2.0);
    assert_eq!(GameplayTags::movement::Dash::DURATION, 0.3);
}

#[test]
fn test_data_type_association() {
    // Type checking - these should compile
    fn requires_ability_data<T: HasData<Data = crate::AbilityData>>() {}
    fn requires_movement_data<T: HasData<Data = crate::MovementData>>() {}

    requires_ability_data::<GameplayTags::HeavyAttack>();
    requires_ability_data::<GameplayTags::Status>();
    requires_movement_data::<GameplayTags::Dash>();
}

#[test]
fn test_gid_still_works() {
    // Ensure GID generation still works correctly
    let basic_gid = GameplayTags::BasicAttack::GID;
    let heavy_gid = GameplayTags::HeavyAttack::GID;
    let sprint_gid = GameplayTags::Sprint::GID;

    // All GIDs should be unique
    assert_ne!(basic_gid, heavy_gid);
    assert_ne!(basic_gid, sprint_gid);
    assert_ne!(heavy_gid, sprint_gid);

    // GIDs should be non-zero
    assert_ne!(basic_gid, 0);
    assert_ne!(heavy_gid, 0);
}

#[test]
fn test_namespace_tag_trait_still_works() {
    assert_eq!(GameplayTags::BasicAttack::PATH, "BasicAttack");
    assert_eq!(GameplayTags::HeavyAttack::PATH, "HeavyAttack");
    assert_eq!(GameplayTags::Sprint::PATH, "Movement.Sprint");

    assert_eq!(GameplayTags::BasicAttack::DEPTH, 0);
    assert_eq!(GameplayTags::Sprint::DEPTH, 1);
}

#[test]
fn test_serialization() {
    // Test that data types can be serialized
    let ability = AbilityData {
        name: "Heavy Strike".to_string(),
        description: "A powerful melee attack".to_string(),
    };

    let json = serde_json::to_string(&ability).unwrap();
    let deserialized: AbilityData = serde_json::from_str(&json).unwrap();

    assert_eq!(ability, deserialized);
}

#[test]
fn test_mixed_features() {
    // Node with both metadata and data type
    assert_eq!(GameplayTags::HeavyAttack::DAMAGE, 100);
    assert_eq!(GameplayTags::HeavyAttack::COOLDOWN, 2.5);

    // Has associated data type
    fn check_has_data<T: HasData>() -> &'static str {
        T::PATH
    }

    assert_eq!(check_has_data::<GameplayTags::HeavyAttack>(), "HeavyAttack");
}
