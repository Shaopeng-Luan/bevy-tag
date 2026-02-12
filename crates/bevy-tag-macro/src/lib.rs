use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, token, Expr, Ident, Result, Token, Type, Visibility};

use proc_macro_crate::{crate_name, FoundCrate};

// =============================================================================
// Constants - must match bevy-tag's layout.rs
// =============================================================================

/// Maximum supported tree depth.
const MAX_DEPTH: usize = 16;

// =============================================================================
// Parsing
// =============================================================================

/// Metadata attribute: #[key = value]
#[derive(Clone)]
struct MetaAttr {
    key: Ident,
    value: Expr,
}

/// Deprecation attribute: #[deprecated(note = "...")]
#[derive(Clone, Default)]
struct DeprecationAttr {
    /// Whether the node is deprecated
    is_deprecated: bool,
    /// Optional deprecation note
    note: Option<String>,
}

struct Node {
    name: Ident,
    /// User-defined metadata attributes (#[key = value])
    attrs: Vec<MetaAttr>,
    /// Deprecation attribute (#[deprecated] or #[deprecated(note = "...")])
    deprecation: DeprecationAttr,
    /// Optional: Node<DataType>
    data_type: Option<Type>,
    children: Vec<Node>,
}

struct NamespaceInput {
    vis: Visibility,
    root: Ident,
    nodes: Vec<Node>,
}

impl Parse for NamespaceInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis: Visibility = input.parse()?;
        input.parse::<Token![mod]>()?;
        let root: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let nodes = parse_nodes(&content)?;
        Ok(Self { vis, root, nodes })
    }
}

fn parse_nodes(input: ParseStream) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        // Parse attributes
        let (attrs, deprecation) = parse_all_attrs(input)?;

        // Parse node name
        let name: Ident = input.parse()?;

        // Parse optional type parameter: Node<Type>
        let data_type = if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            let ty: Type = input.parse()?;
            input.parse::<Token![>]>()?;
            Some(ty)
        } else {
            None
        };

        // Parse children or semicolon
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            let children = parse_nodes(&content)?;
            nodes.push(Node {
                name,
                attrs,
                deprecation,
                data_type,
                children,
            });
        } else {
            input.parse::<Token![;]>()?;
            nodes.push(Node {
                name,
                attrs,
                deprecation,
                data_type,
                children: Vec::new(),
            });
        }
    }
    Ok(nodes)
}

/// Parse all attributes, separating #[deprecated(...)] from #[key = value]
fn parse_all_attrs(input: ParseStream) -> Result<(Vec<MetaAttr>, DeprecationAttr)> {
    let mut attrs = Vec::new();
    let mut deprecation = DeprecationAttr::default();

    while input.peek(Token![#]) {
        input.parse::<Token![#]>()?;
        let content;
        syn::bracketed!(content in input);

        let key: Ident = content.parse()?;

        if key == "deprecated" {
            deprecation.is_deprecated = true;

            // Check for (note = "...")
            if content.peek(syn::token::Paren) {
                let inner;
                syn::parenthesized!(inner in content);

                // Parse note = "..."
                if !inner.is_empty() {
                    let note_key: Ident = inner.parse()?;
                    if note_key == "note" {
                        inner.parse::<Token![=]>()?;
                        let note_value: syn::LitStr = inner.parse()?;
                        deprecation.note = Some(note_value.value());
                    }
                }
            }
        } else {
            // Regular metadata attribute: #[key = value]
            content.parse::<Token![=]>()?;
            let value: Expr = content.parse()?;
            attrs.push(MetaAttr { key, value });
        }
    }

    Ok((attrs, deprecation))
}

// =============================================================================
// Tree analysis (runs at macro expansion time)
// =============================================================================

/// Flattened node with computed metadata.
struct FlatNode {
    /// Path segments: ["Movement", "Idle"]
    segments: Vec<String>,
    /// Depth: 0 for roots, 1 for children, etc.
    depth: u8,
}

/// Flatten the parsed tree into a list with depth/path info.
fn flatten_nodes(nodes: &[Node], prefix: &str, depth: u8, out: &mut Vec<FlatNode>) {
    for node in nodes {
        let path = if prefix.is_empty() {
            node.name.to_string()
        } else {
            format!("{}.{}", prefix, node.name)
        };

        let segments: Vec<String> = path.split('.').map(String::from).collect();

        out.push(FlatNode { segments, depth });

        flatten_nodes(&node.children, &path, depth + 1, out);
    }
}

// =============================================================================
// Crate path resolution
// =============================================================================

fn namespace_crate_path() -> TokenStream2 {
    match crate_name("bevy-tag") {
        Ok(FoundCrate::Itself) => {
            quote!(::bevy_tag)
        }
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::bevy_tag),
    }
}

// =============================================================================
// Code generation
// =============================================================================

/// Generate tag struct and its implementations.
fn generate_tag_impl(
    node_ident: &Ident,
    path_lit: &syn::LitStr,
    depth_lit: u8,
    seg_count: usize,
    seg_lits: &[syn::LitByteStr],
    metadata: &TokenStream2,
    ns_crate: &TokenStream2,
) -> TokenStream2 {
    quote! {
        /// Zero-sized tag type for this namespace node.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct #node_ident;

        impl #node_ident {
            /// Full dot-separated path.
            pub const PATH: &'static str = #path_lit;

            /// Depth in the namespace tree (0 = top-level).
            pub const DEPTH: u8 = #depth_lit;

            /// Stable hierarchical GID, computed at compile time.
            pub const GID: #ns_crate::GID = {
                const SEGS: [&[u8]; #seg_count] = [#(#seg_lits),*];
                #ns_crate::hierarchical_gid(&SEGS)
            };

            /// Get the GID (convenience method).
            #[inline]
            pub const fn gid() -> #ns_crate::GID {
                Self::GID
            }

            #metadata
        }

        impl core::fmt::Display for #node_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(Self::PATH)
            }
        }

        impl core::fmt::LowerHex for #node_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&Self::GID, f)
            }
        }

        impl core::fmt::UpperHex for #node_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperHex::fmt(&Self::GID, f)
            }
        }

        impl #ns_crate::NamespaceTag for #node_ident {
            const PATH: &'static str = #path_lit;
            const DEPTH: u8 = #depth_lit;
            const STABLE_GID: #ns_crate::GID = #node_ident::GID;
        }
    }
}

/// Convert CamelCase to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Recursively generate tag types using CamelCase struct + snake_case module pattern.
///
/// Strategy:
/// - All nodes get a CamelCase struct (the tag type)
/// - Branch nodes also get a snake_case module containing re-exports of children
///
/// Example:
/// ```ignore
/// namespace! {
///     pub mod Tags {
///         Movement {       // Branch: has children
///             Idle;        // Leaf
///             Running;     // Leaf
///         }
///         Simple;          // Leaf at root
///     }
/// }
///
/// // Generates:
/// pub mod Tags {
///     pub struct Movement;  // CamelCase struct
///     impl Movement { pub const PATH, GID, ... }
///
///     pub mod movement {    // snake_case module for children
///         pub use super::Idle;
///         pub use super::Running;
///     }
///
///     pub struct Idle;
///     impl Idle { pub const PATH = "Movement.Idle", ... }
///
///     pub struct Running;
///     impl Running { ... }
///
///     pub struct Simple;
///     impl Simple { ... }
/// }
///
/// // Usage:
/// Tags::Movement              // The Movement tag (type)
/// Tags::Movement::GID         // Movement's GID
/// Tags::movement::Idle        // Child via snake_case module
/// Tags::movement::Idle::GID   // Child's GID
/// Tags::Simple                // Leaf at root
/// ```
fn generate_tags_recursive(
    nodes: &[Node],
    prefix: &str,
    depth: u8,
    ns_crate: &TokenStream2,
) -> Vec<TokenStream2> {
    if depth as usize >= MAX_DEPTH {
        panic!(
            "namespace tree depth ({}) exceeds maximum ({})",
            depth, MAX_DEPTH
        );
    }

    let mut output = Vec::new();

    for node in nodes {
        let node_ident = &node.name;
        let path = if prefix.is_empty() {
            node.name.to_string()
        } else {
            format!("{}.{}", prefix, node.name)
        };
        let path_lit = syn::LitStr::new(&path, Span::call_site());

        // Build segments as byte string literals for const fn call
        let segments: Vec<&str> = path.split('.').collect();
        let seg_count = segments.len();
        let seg_lits: Vec<syn::LitByteStr> = segments
            .iter()
            .map(|s| syn::LitByteStr::new(s.as_bytes(), Span::call_site()))
            .collect();

        let depth_lit = depth;

        // Generate metadata constants from attributes
        let metadata = generate_metadata_consts(&node.attrs);

        // Generate deprecation attribute if present
        let deprecation_attr = if node.deprecation.is_deprecated {
            if let Some(ref note) = node.deprecation.note {
                let note_lit = syn::LitStr::new(note, Span::call_site());
                quote! { #[deprecated(note = #note_lit)] }
            } else {
                quote! { #[deprecated] }
            }
        } else {
            quote! {}
        };

        // Generate data type association if present
        let data_type_def = if let Some(ref ty) = node.data_type {
            quote! {
                impl #ns_crate::HasData for #node_ident {
                    type Data = #ty;
                }
            }
        } else {
            quote! {}
        };

        // Generate tag implementation
        let tag_impl = generate_tag_impl(
            node_ident,
            &path_lit,
            depth_lit,
            seg_count,
            &seg_lits,
            &metadata,
            ns_crate,
        );

        // Generate children recursively (they are flat siblings)
        let children_output = if !node.children.is_empty() {
            generate_tags_recursive(&node.children, &path, depth + 1, ns_crate)
        } else {
            Vec::new()
        };

        // Generate snake_case module with re-exports for branch nodes
        let child_module = if !node.children.is_empty() {
            let snake_name = to_snake_case(&node.name.to_string());
            let mod_ident = Ident::new(&snake_name, node.name.span());

            // Collect all descendant names for re-export (direct children only)
            let reexports: Vec<TokenStream2> = node
                .children
                .iter()
                .map(|child| {
                    let child_ident = &child.name;
                    let child_deprecation = if child.deprecation.is_deprecated {
                        if let Some(ref note) = child.deprecation.note {
                            let note_lit = syn::LitStr::new(note, Span::call_site());
                            quote! { #[deprecated(note = #note_lit)] }
                        } else {
                            quote! { #[deprecated] }
                        }
                    } else {
                        quote! {}
                    };

                    // If child has children, also re-export its snake_case module
                    if !child.children.is_empty() {
                        let child_snake = to_snake_case(&child.name.to_string());
                        let child_mod_ident = Ident::new(&child_snake, child.name.span());
                        quote! {
                            #child_deprecation
                            pub use super::#child_ident;
                            #child_deprecation
                            pub use super::#child_mod_ident;
                        }
                    } else {
                        quote! {
                            #child_deprecation
                            pub use super::#child_ident;
                        }
                    }
                })
                .collect();

            quote! {
                #deprecation_attr
                #[allow(non_camel_case_types)]
                pub mod #mod_ident {
                    #(#reexports)*
                }
            }
        } else {
            quote! {}
        };

        // Output: first all children (flat), then this node, then child module
        output.extend(children_output);
        output.push(quote! {
            #deprecation_attr
            #tag_impl
            #data_type_def
        });
        if !node.children.is_empty() {
            output.push(child_module);
        }
    }

    output
}

/// Generate const fields from metadata attributes.
fn generate_metadata_consts(attrs: &[MetaAttr]) -> TokenStream2 {
    let consts: Vec<TokenStream2> = attrs
        .iter()
        .map(|attr| {
            let key = &attr.key;
            let value = &attr.value;

            // Convert ident to SCREAMING_SNAKE_CASE for const name
            let const_name = Ident::new(&key.to_string().to_uppercase(), key.span());

            // Try to infer type from expression
            let ty = infer_type_from_expr(value);

            quote! {
                #[doc = concat!("Metadata: ", stringify!(#key))]
                pub const #const_name: #ty = #value;
            }
        })
        .collect();

    quote! { #(#consts)* }
}

/// Infer Rust type from expression (best-effort).
fn infer_type_from_expr(expr: &Expr) -> TokenStream2 {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(_) => quote!(i32),
            syn::Lit::Float(_) => quote!(f64),
            syn::Lit::Bool(_) => quote!(bool),
            syn::Lit::Str(_) => quote!(&'static str),
            syn::Lit::Char(_) => quote!(char),
            _ => quote!(i32), // fallback
        },
        _ => quote!(i32), // fallback for complex expressions
    }
}

/// Generate `NamespaceDef` entries.
fn collect_defs(
    nodes: &[Node],
    prefix: &str,
    parent: Option<&str>,
    ns_crate: &TokenStream2,
    out: &mut Vec<TokenStream2>,
) {
    for node in nodes {
        let path = if prefix.is_empty() {
            node.name.to_string()
        } else {
            format!("{}.{}", prefix, node.name)
        };
        let path_lit = syn::LitStr::new(&path, Span::call_site());

        let parent_tokens = match parent {
            Some(pp) => {
                let parent_lit = syn::LitStr::new(pp, Span::call_site());
                quote!(Some(#parent_lit))
            }
            None => quote!(None),
        };

        out.push(quote! {
            #ns_crate::NamespaceDef {
                path: #path_lit,
                parent: #parent_tokens,
            },
        });

        collect_defs(&node.children, &path, Some(&path), ns_crate, out);
    }
}

/// Generate compile-time collision detection.
fn generate_collision_check(flat: &[FlatNode], ns_crate: &TokenStream2) -> TokenStream2 {
    let n = flat.len();

    let gid_computations: Vec<TokenStream2> = flat
        .iter()
        .map(|node| {
            let seg_count = node.segments.len();
            let seg_lits: Vec<syn::LitByteStr> = node
                .segments
                .iter()
                .map(|s| syn::LitByteStr::new(s.as_bytes(), Span::call_site()))
                .collect();

            quote! {
                {
                    const SEGS: [&[u8]; #seg_count] = [#(#seg_lits),*];
                    #ns_crate::hierarchical_gid(&SEGS)
                }
            }
        })
        .collect();

    quote! {
        const _: () = {
            const GIDS: [#ns_crate::GID; #n] = [
                #(#gid_computations),*
            ];

            const fn check_collisions(gids: &[u128], count: usize) {
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

            check_collisions(&GIDS, #n);
        };
    }
}

// =============================================================================
// Entry point
// =============================================================================

#[proc_macro]
pub fn namespace(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as NamespaceInput);
    let ns_crate = namespace_crate_path();

    // 1. Flatten tree and analyze shape
    let mut flat = Vec::new();
    flatten_nodes(&input.nodes, "", 0, &mut flat);

    // Validate depth
    let max_depth = flat.iter().map(|n| n.depth).max().unwrap_or(0);
    if max_depth as usize >= MAX_DEPTH {
        panic!(
            "namespace tree depth ({}) exceeds maximum ({})",
            max_depth + 1,
            MAX_DEPTH
        );
    }

    let tree_depth = (max_depth + 1) as usize;
    let node_count = flat.len();

    // 2. Generate tags
    let tags = generate_tags_recursive(&input.nodes, "", 0, &ns_crate);

    // 3. Generate NamespaceDef entries
    let mut defs = Vec::new();
    collect_defs(&input.nodes, "", None, &ns_crate, &mut defs);

    // 4. Generate collision detection
    let collision_check = generate_collision_check(&flat, &ns_crate);

    // 5. Assemble
    let vis = input.vis;
    let root = input.root;

    let expanded = quote! {
        #[allow(non_snake_case, non_camel_case_types)]
        #vis mod #root {
            /// Number of tree levels in this namespace.
            pub const TREE_DEPTH: usize = #tree_depth;

            /// Total number of namespace nodes.
            pub const NODE_COUNT: usize = #node_count;

            /// Flat NamespaceDef table (for runtime registry).
            pub const DEFINITIONS: &'static [#ns_crate::NamespaceDef] = &[
                #(#defs)*
            ];

            #collision_check

            #(#tags)*
        }
    };

    expanded.into()
}
