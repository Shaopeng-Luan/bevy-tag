//! # Hierarchical Namespace ID System (bevy-tag)
//!
//! Provides stable, hierarchical identifiers (GID) for tree-structured namespaces.
//! Inspired by UE5 GameplayTags with O(1) subtree membership checks.
//!
//! ## Design
//!
//! A `GID` is a `u128` with embedded depth and fixed bit allocation across 8 levels:
//!
//! ```text
//! ┌─────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
//! │ Depth   │ Level 0  │ Level 1  │ Level 2  │ Level 3  │ Level 4  │ Level 5  │ Level 6  │ Level 7  │
//! │ 3 bits  │ 21 bits  │ 18 bits  │ 16 bits  │ 16 bits  │ 14 bits  │ 14 bits  │ 13 bits  │ 13 bits  │
//! └─────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
//! ```
//!
//! ## Self-Contained Operations
//!
//! Because depth is embedded in the GID, subtree checks are completely self-contained:
//!
//! ```ignore
//! use bevy_tag::{is_descendant_of, depth_of};
//!
//! // No registry needed!
//! if is_descendant_of(entity_tag, Movement::Tag::GID) {
//!     // entity is under Movement subtree
//! }
//!
//! let depth = depth_of(tag); // Extract depth directly from GID
//! ```

pub mod hash;
pub mod layout;
pub mod registry;
pub mod traits;

pub use hash::{fnv1a_64, hierarchical_gid, segment_hash};
pub use layout::{
    depth_of, encode_gid, gid_is_descendant_of, is_sibling, parent_of, DEPTH_BITS, DEPTH_MASK,
    DEPTH_SHIFT, LEVEL_MASKS, LEVEL_OFFSETS, LEVEL_WIDTHS, MAX_DEPTH,
};
pub use registry::{NamespaceDef, NamespaceEntry, NamespaceRegistry};
pub use traits::{Alias, HasData, IntoGid, IntoGids, NamespaceTag};

/// Global Identifier — a stable, hierarchical hash packed into u128.
///
/// The top 3 bits encode the depth (0-7), remaining 125 bits are
/// partitioned by tree level with a fixed layout.
///
/// Subtree membership is a single bitmask comparison (O(1)),
/// and requires no external registry lookup.
pub type GID = u128;

/// Root GID constant (all zeros, depth 0).
pub const ROOT_GID: GID = 0;
