use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, token, Expr, Ident, Result, Token, Type, Visibility};

use proc_macro_crate::{crate_name, FoundCrate};

/// Maximum supported tree depth (0-7, encoded in 3 bits).
const MAX_DEPTH: usize = 8;

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

/// Parsed attributes for a node.
#[derive(Clone, Default)]
struct NodeAttrs {
    /// User-defined metadata attributes (#[key = value])
    meta: Vec<MetaAttr>,
    /// Deprecation attribute (#[deprecated] or #[deprecated(note = "...")])
    deprecation: DeprecationAttr,
    /// Redirect target path (#[redirect = "Path.To.Target"])
    redirect_to: Option<String>,
}

struct Node {
    name: Ident,
    /// All parsed attributes
    attrs: NodeAttrs,
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
        let attrs = parse_all_attrs(input)?;

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
                data_type,
                children,
            });
        } else {
            input.parse::<Token![;]>()?;
            nodes.push(Node {
                name,
                attrs,
                data_type,
                children: Vec::new(),
            });
        }
    }
    Ok(nodes)
}

/// Parse all attributes into NodeAttrs.
///
/// Handles:
/// - `#[deprecated]` or `#[deprecated(note = "...")]`
/// - `#[redirect = "Path.To.Target"]`
/// - `#[key = value]` (metadata)
fn parse_all_attrs(input: ParseStream) -> Result<NodeAttrs> {
    let mut result = NodeAttrs::default();

    while input.peek(Token![#]) {
        input.parse::<Token![#]>()?;
        let content;
        syn::bracketed!(content in input);

        let key: Ident = content.parse()?;

        if key == "deprecated" {
            result.deprecation.is_deprecated = true;

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
                        result.deprecation.note = Some(note_value.value());
                    }
                }
            }
        } else if key == "redirect" {
            // #[redirect = "Path.To.Target"]
            content.parse::<Token![=]>()?;
            let target: syn::LitStr = content.parse()?;
            result.redirect_to = Some(target.value());
        } else {
            // Regular metadata attribute: #[key = value]
            content.parse::<Token![=]>()?;
            let value: Expr = content.parse()?;
            result.meta.push(MetaAttr { key, value });
        }
    }

    Ok(result)
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
/// Skips redirect nodes (they don't have their own GID).
fn flatten_nodes(nodes: &[Node], prefix: &str, depth: u8, out: &mut Vec<FlatNode>) {
    for node in nodes {
        // Skip redirect nodes - they use target's GID
        if node.attrs.redirect_to.is_some() {
            continue;
        }

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

/// Recursively generate tag types using module-based pattern.
///
/// Strategy:
/// - Each node becomes a module containing `pub struct Tag` and convenience consts
/// - Children are nested modules inside the parent module
/// - No snake_case/CamelCase mismatch - module names match path segments exactly
///
/// Example:
/// ```ignore
/// namespace! {
///     pub mod Tags {
///         Movement {       // Branch: has children
///             Idle;        // Leaf
///             Running;     // Leaf
///         }
///         Combat {
///             Idle;        // Same name as Movement.Idle - no conflict!
///         }
///         Simple;          // Leaf at root
///     }
/// }
///
/// // Generates:
/// #[allow(non_snake_case)]
/// pub mod Tags {
///     pub mod Movement {
///         pub struct Tag;
///         pub const GID: GID = Tag::GID;
///         pub const PATH: &str = Tag::PATH;
///
///         pub mod Idle {
///             pub struct Tag;
///             pub const GID: GID = Tag::GID;
///         }
///         pub mod Running { ... }
///     }
///
///     pub mod Combat {
///         pub struct Tag;
///         pub mod Idle { ... }  // No conflict with Movement::Idle!
///     }
///
///     pub mod Simple {
///         pub struct Tag;
///         pub const GID: GID = Tag::GID;
///     }
/// }
///
/// // Usage:
/// Tags::Movement::Tag         // The Movement tag type
/// Tags::Movement::GID         // Convenience const
/// Tags::Movement::Idle::Tag   // Child tag type
/// Tags::Movement::Idle::GID   // Child's GID
/// ```
/// Convert a dot-separated path to a Rust type path relative to the module root.
///
/// Example: "Equipment.Weapon.Blade" -> Equipment::Weapon::Blade::Tag
fn path_to_rust_type_path(path: &str) -> TokenStream2 {
    let segments: Vec<&str> = path.split('.').collect();
    let modules: Vec<Ident> = segments
        .iter()
        .map(|s| Ident::new(s, Span::call_site()))
        .collect();
    quote! { #(#modules::)*Tag }
}

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

        // Generate deprecation attribute if present
        let deprecation_attr = if node.attrs.deprecation.is_deprecated {
            if let Some(ref note) = node.attrs.deprecation.note {
                let note_lit = syn::LitStr::new(note, Span::call_site());
                quote! { #[deprecated(note = #note_lit)] }
            } else {
                quote! { #[deprecated] }
            }
        } else {
            quote! {}
        };

        // Check if this node is a redirect
        if let Some(ref target_path) = node.attrs.redirect_to {
            // Generate module with type alias: pub mod OldName { pub type Tag = Redirect<...>; }
            let target_type = path_to_rust_type_path(target_path);

            // Add deprecation note about redirect if not already deprecated
            let redirect_deprecation = if node.attrs.deprecation.is_deprecated {
                deprecation_attr.clone()
            } else {
                let note = format!("redirected to {}", target_path);
                let note_lit = syn::LitStr::new(&note, Span::call_site());
                quote! { #[deprecated(note = #note_lit)] }
            };

            output.push(quote! {
                #redirect_deprecation
                #[allow(non_snake_case)]
                pub mod #node_ident {
                    use super::*;
                    pub type Tag = #ns_crate::Redirect<#target_type>;
                    pub const GID: #ns_crate::GID = <Tag as #ns_crate::NamespaceTag>::GID;
                    pub const PATH: &'static str = <Tag as #ns_crate::NamespaceTag>::PATH;
                    pub const DEPTH: u8 = <Tag as #ns_crate::NamespaceTag>::DEPTH;
                }
            });

            // Redirects cannot have children
            if !node.children.is_empty() {
                panic!(
                    "Node '{}' has #[redirect] but also has children. Redirects must be leaf nodes.",
                    path
                );
            }

            continue;
        }

        // Regular node generation (not a redirect)
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
        let metadata = generate_metadata_consts(&node.attrs.meta);

        // Generate data type association if present
        let data_type_impl = if let Some(ref ty) = node.data_type {
            quote! {
                impl #ns_crate::HasData for Tag {
                    type Data = #ty;
                }
            }
        } else {
            quote! {}
        };

        // Generate children recursively
        let children_output = if !node.children.is_empty() {
            generate_tags_recursive(&node.children, &path, depth + 1, ns_crate)
        } else {
            Vec::new()
        };

        // Generate the module containing Tag struct and children
        output.push(quote! {
            #deprecation_attr
            #[allow(non_snake_case)]
            pub mod #node_ident {
                use super::*;

                /// Zero-sized tag type for this namespace node.
                #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
                pub struct Tag;

                impl Tag {
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

                impl core::fmt::Display for Tag {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        f.write_str(Self::PATH)
                    }
                }

                impl core::fmt::LowerHex for Tag {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        core::fmt::LowerHex::fmt(&Self::GID, f)
                    }
                }

                impl core::fmt::UpperHex for Tag {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        core::fmt::UpperHex::fmt(&Self::GID, f)
                    }
                }

                impl #ns_crate::NamespaceTag for Tag {
                    const PATH: &'static str = #path_lit;
                    const DEPTH: u8 = #depth_lit;
                    const GID: #ns_crate::GID = Tag::GID;
                }

                #data_type_impl

                // Module-level convenience constants
                pub const GID: #ns_crate::GID = Tag::GID;
                pub const PATH: &'static str = Tag::PATH;
                pub const DEPTH: u8 = Tag::DEPTH;

                // Nested child modules
                #(#children_output)*
            }
        });
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
/// Skips redirect nodes (they don't have their own definition).
fn collect_defs(
    nodes: &[Node],
    prefix: &str,
    parent: Option<&str>,
    ns_crate: &TokenStream2,
    out: &mut Vec<TokenStream2>,
) {
    for node in nodes {
        // Skip redirect nodes - they point to another definition
        if node.attrs.redirect_to.is_some() {
            continue;
        }

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

/// Generate compile-time collision detection with detailed error messages.
fn generate_collision_check(flat: &[FlatNode], ns_crate: &TokenStream2) -> TokenStream2 {
    // Generate individual collision checks for each pair with specific error messages
    let mut checks = Vec::new();

    for i in 0..flat.len() {
        for j in (i + 1)..flat.len() {
            let path_i = flat[i].segments.join(".");
            let path_j = flat[j].segments.join(".");

            let seg_count_i = flat[i].segments.len();
            let seg_lits_i: Vec<syn::LitByteStr> = flat[i]
                .segments
                .iter()
                .map(|s| syn::LitByteStr::new(s.as_bytes(), Span::call_site()))
                .collect();

            let seg_count_j = flat[j].segments.len();
            let seg_lits_j: Vec<syn::LitByteStr> = flat[j]
                .segments
                .iter()
                .map(|s| syn::LitByteStr::new(s.as_bytes(), Span::call_site()))
                .collect();

            let error_msg = format!(
                "GID collision detected: '{}' and '{}' hash to the same value",
                path_i, path_j
            );

            checks.push(quote! {
                const _: () = {
                    const GID_A: #ns_crate::GID = {
                        const SEGS: [&[u8]; #seg_count_i] = [#(#seg_lits_i),*];
                        #ns_crate::hierarchical_gid(&SEGS)
                    };
                    const GID_B: #ns_crate::GID = {
                        const SEGS: [&[u8]; #seg_count_j] = [#(#seg_lits_j),*];
                        #ns_crate::hierarchical_gid(&SEGS)
                    };
                    assert!(GID_A != GID_B, #error_msg);
                };
            });
        }
    }

    quote! {
        #(#checks)*
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that same-named children under different parents generate unique paths.
    /// This verifies the module-based code generation doesn't cause naming conflicts.
    #[test]
    fn test_same_name_different_roots_no_conflict() {
        // Simulate: Combat { Attack; } Movement { Attack; }
        let nodes = vec![
            Node {
                name: Ident::new("Combat", Span::call_site()),
                data_type: None,
                attrs: NodeAttrs::default(),
                children: vec![Node {
                    name: Ident::new("Attack", Span::call_site()),
                    data_type: None,
                    attrs: NodeAttrs::default(),
                    children: vec![],
                }],
            },
            Node {
                name: Ident::new("Movement", Span::call_site()),
                data_type: None,
                attrs: NodeAttrs::default(),
                children: vec![Node {
                    name: Ident::new("Attack", Span::call_site()),
                    data_type: None,
                    attrs: NodeAttrs::default(),
                    children: vec![],
                }],
            },
        ];

        let ns_crate = quote!(::bevy_tag);
        let output = generate_tags_recursive(&nodes, "", 0, &ns_crate);

        // Should generate 2 top-level modules (Combat and Movement)
        assert_eq!(output.len(), 2);

        let code = quote! { #(#output)* }.to_string();

        // Verify both modules exist
        assert!(code.contains("pub mod Combat"));
        assert!(code.contains("pub mod Movement"));

        // Verify nested Attack modules have correct paths
        assert!(code.contains("\"Combat.Attack\""));
        assert!(code.contains("\"Movement.Attack\""));

        // Verify no flat re-exports that would cause conflicts
        // The old buggy code would have generated conflicting `pub use combat::Attack`
        assert!(!code.contains("pub use"));
    }

    /// Test deeply nested same names don't conflict.
    #[test]
    fn test_deeply_nested_same_names_no_conflict() {
        // Simulate: A { X { Y; } } B { X { Y; } }
        let nodes = vec![
            Node {
                name: Ident::new("A", Span::call_site()),
                data_type: None,
                attrs: NodeAttrs::default(),
                children: vec![Node {
                    name: Ident::new("X", Span::call_site()),
                    data_type: None,
                    attrs: NodeAttrs::default(),
                    children: vec![Node {
                        name: Ident::new("Y", Span::call_site()),
                        data_type: None,
                        attrs: NodeAttrs::default(),
                        children: vec![],
                    }],
                }],
            },
            Node {
                name: Ident::new("B", Span::call_site()),
                data_type: None,
                attrs: NodeAttrs::default(),
                children: vec![Node {
                    name: Ident::new("X", Span::call_site()),
                    data_type: None,
                    attrs: NodeAttrs::default(),
                    children: vec![Node {
                        name: Ident::new("Y", Span::call_site()),
                        data_type: None,
                        attrs: NodeAttrs::default(),
                        children: vec![],
                    }],
                }],
            },
        ];

        let ns_crate = quote!(::bevy_tag);
        let output = generate_tags_recursive(&nodes, "", 0, &ns_crate);

        let code = quote! { #(#output)* }.to_string();

        // Verify all paths are unique
        assert!(code.contains("\"A.X.Y\""));
        assert!(code.contains("\"B.X.Y\""));
        assert!(code.contains("\"A.X\""));
        assert!(code.contains("\"B.X\""));
    }
}
