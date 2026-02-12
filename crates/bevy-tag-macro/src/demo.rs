/// ═══════════════════════════════════════════════════════════════════
/// INPUT (what the user writes)
/// ═══════════════════════════════════════════════════════════════════
///
/// ```rust
/// namespace! {
///     pub mod game_ids {
///         Movement {
///             Idle;
///             Running;
///             Jumping;
///         }
///         Combat {
///             Attack;
///             Block;
///         }
///     }
/// }
/// ```
///
/// ═══════════════════════════════════════════════════════════════════
/// TREE ANALYSIS (happens at macro expansion time)
/// ═══════════════════════════════════════════════════════════════════
///
/// Tree shape:
///   Level 0: Movement, Combat         → 2 nodes
///   Level 1: Idle, Running, Jumping,  → 5 nodes
///            Attack, Block
///
/// Layout computation:
///   bits_for_count(2)  = clamp(2*2 + 16, 4, 32) = 20 bits
///   bits_for_count(5)  = clamp(2*3 + 16, 4, 32) = 22 bits
///   Total = 42 bits ≤ 64 ✓
///
///   LAYOUT_WIDTHS  = [20, 22]
///   LAYOUT_OFFSETS = [0,  20]
///
/// ═══════════════════════════════════════════════════════════════════
/// OUTPUT (what the macro generates)
/// ═══════════════════════════════════════════════════════════════════

pub mod game_ids {
    // ── Layout constants ──

    pub const LAYOUT_WIDTHS: [u8; 2] = [20u8, 22u8];
    pub const LAYOUT_OFFSETS: [u8; 2] = [0u8, 20u8];
    pub const TREE_DEPTH: usize = 2;
    pub const TOTAL_BITS: u16 = 42;
    pub const NODE_COUNT: usize = 7;
    pub const NODES_PER_LEVEL: [usize; 2] = [2usize, 5usize];

    // ── Flat NamespaceDef table ──

    pub const DEFINITIONS: &'static [NamespaceDef] = &[
        NamespaceDef { path: "Movement",         parent: None },
        NamespaceDef { path: "Movement.Idle",    parent: Some("Movement") },
        NamespaceDef { path: "Movement.Running", parent: Some("Movement") },
        NamespaceDef { path: "Movement.Jumping", parent: Some("Movement") },
        NamespaceDef { path: "Combat",           parent: None },
        NamespaceDef { path: "Combat.Attack",    parent: Some("Combat") },
        NamespaceDef { path: "Combat.Block",     parent: Some("Combat") },
    ];

    // ── Compile-time collision detection ──

    const _: () = {
        const GIDS: [u64; 7] = [
            { const SEGS: [&[u8]; 1] = [b"Movement"];  hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 2] = [b"Movement", b"Idle"];    hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 2] = [b"Movement", b"Running"]; hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 2] = [b"Movement", b"Jumping"]; hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 1] = [b"Combat"];  hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 2] = [b"Combat", b"Attack"]; hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
            { const SEGS: [&[u8]; 2] = [b"Combat", b"Block"];  hierarchical_gid(&SEGS, &LAYOUT_WIDTHS, &LAYOUT_OFFSETS) },
        ];

        const fn check_collisions(gids: &[u64], count: usize) {
            let mut i = 0;
            while i < count {
                let mut j = i + 1;
                while j < count {
                    if gids[i] == gids[j] {
                        panic!("namespace GID collision detected!");
                    }
                    j += 1;
                }
                i += 1;
            }
        }

        check_collisions(&GIDS, 7);
    };

    // ── Node modules ──

    #[allow(non_snake_case)]
    pub mod Movement {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct Tag;

        impl Tag {
            pub const PATH: &'static str = "Movement";
            pub const DEPTH: u8 = 0;
            pub const GID: u64 = {
                const SEGS: [&[u8]; 1] = [b"Movement"];
                hierarchical_gid(&SEGS, &super::LAYOUT_WIDTHS, &super::LAYOUT_OFFSETS)
            };

            #[inline]
            pub fn gid() -> u64 { Self::GID }
        }

        impl NamespaceTag for Tag {
            const PATH: &'static str = "Movement";
            const DEPTH: u8 = 0;
            const STABLE_GID: u64 = Tag::GID;
        }

        #[allow(non_snake_case)]
        pub mod Idle {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            pub struct Tag;

            impl Tag {
                pub const PATH: &'static str = "Movement.Idle";
                pub const DEPTH: u8 = 1;
                pub const GID: u64 = {
                    const SEGS: [&[u8]; 2] = [b"Movement", b"Idle"];
                    hierarchical_gid(&SEGS, &super::super::LAYOUT_WIDTHS, &super::super::LAYOUT_OFFSETS)
                    //                        ^^^^^^^^^^^^^ one super per nesting level
                };

                #[inline]
                pub fn gid() -> u64 { Self::GID }
            }

            impl NamespaceTag for Tag {
                const PATH: &'static str = "Movement.Idle";
                const DEPTH: u8 = 1;
                const STABLE_GID: u64 = Tag::GID;
            }
        }

        // pub mod Running { ... }
        // pub mod Jumping { ... }
    }

    // pub mod Combat { ... }
}

/// ═══════════════════════════════════════════════════════════════════
/// USAGE (what the user gets)
/// ═══════════════════════════════════════════════════════════════════
///
/// ```rust
/// use game_ids::*;
///
/// // Compile-time constant GID — no registry lookup needed
/// const IDLE_GID: u64 = Movement::Idle::Tag::GID;
///
/// // As function call (same value)
/// let gid = Movement::Idle::Tag::gid();
///
/// // Path and depth available as constants
/// assert_eq!(Movement::Idle::Tag::PATH, "Movement.Idle");
/// assert_eq!(Movement::Idle::Tag::DEPTH, 1);
///
/// // Subtree check via registry (O(1) bitmask)
/// let registry = NamespaceRegistry::build(game_ids::DEFINITIONS).unwrap();
/// assert!(registry.is_descendant_of(
///     Movement::Idle::Tag::GID,
///     Movement::Tag::GID,
/// ));
///
/// // GIDs are STABLE — adding a sibling won't change existing GIDs
/// // GIDs are ORDER-INDEPENDENT — shuffling definitions produces same GIDs
/// ```
///
/// ═══════════════════════════════════════════════════════════════════
/// COMPARISON: BEFORE vs AFTER
/// ═══════════════════════════════════════════════════════════════════
///
/// ```text
/// BEFORE (DFS counter):
///   Movement::Idle::Tag::gid()
///     → calls with_namespace_registry(|r| r.gid_of("Movement.Idle"))
///     → runtime HashMap lookup
///     → value changes if you add a sibling before it
///
/// AFTER (hierarchical hash):
///   Movement::Idle::Tag::GID
///     → const 0x????_????_????_????
///     → zero-cost, no lookup
///     → value is hash("Movement") | hash("Idle") << offset
///     → never changes unless you rename the path
/// ```
fn _doc_only() {}