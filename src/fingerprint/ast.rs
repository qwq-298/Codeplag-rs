use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};
use crate::core::types::Language;

/// Generate AST structural hashes for a source file
///
/// Parses the source code into an AST and computes structural hashes
/// for subtrees, ignoring identifier names to detect structural similarity
/// even after variable renaming.
pub fn generate_ast_hashes(source: &str, language: Language) -> Option<Vec<u64>> {
    let ts_lang = language.tree_sitter_language()?;

    let mut parser = Parser::new();
    parser.set_language(&ts_lang).ok()?;

    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut hashes = Vec::new();
    collect_subtree_hashes(&root, source, &mut hashes);

    // Sort and deduplicate for set-based comparison
    hashes.sort_unstable();
    hashes.dedup();

    Some(hashes)
}

/// Recursively collect structural hashes from AST nodes
fn collect_subtree_hashes(node: &Node, source: &str, hashes: &mut Vec<u64>) {
    // Compute structural hash for this node
    let hash = structural_hash(node, source);
    hashes.push(hash);

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_subtree_hashes(&child, source, hashes);
        }
    }
}

/// Compute a structural hash for an AST node that ignores identifier names
///
/// The hash includes:
/// - Node kind (e.g., "function_item", "let_declaration")
/// - Child node kinds in order
/// - Literal values (numbers, strings)
/// - NOT identifier names (to catch variable renaming)
fn structural_hash(node: &Node, source: &str) -> u64 {
    let mut hasher = Sha256::new();

    // Hash the node kind
    hasher.update(node.kind().as_bytes());

    // Hash children's kinds (structural fingerprint)
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            hasher.update(child.kind().as_bytes());

            // Only include literal text for literals, not identifiers
            if is_literal_kind(child.kind()) {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    hasher.update(text.as_bytes());
                }
            }
        }
    }

    // Take first 8 bytes as u64
    let hash_bytes = hasher.finalize();
    u64::from_be_bytes(hash_bytes[..8].try_into().unwrap())
}

/// Check if a node kind represents a literal value
fn is_literal_kind(kind: &str) -> bool {
    kind.contains("integer")
        || kind.contains("float")
        || kind.contains("string")
        || kind.contains("char")
        || kind.contains("boolean")
        || kind == "number"
        || kind == "true"
        || kind == "false"
        || kind == "nil"
}

/// Calculate Jaccard similarity between two AST hash sets
pub fn ast_jaccard_similarity(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let intersection = count_intersection(a, b);
    let union = a.len() + b.len() - intersection;

    intersection as f64 / union as f64
}

/// Count elements present in both sorted slices
fn count_intersection(a: &[u64], b: &[u64]) -> usize {
    let mut count = 0;
    let (mut i, mut j) = (0, 0);

    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_hash_rust() {
        let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let hashes = generate_ast_hashes(code, Language::Rust);
        assert!(hashes.is_some());
        assert!(!hashes.unwrap().is_empty());
    }

    #[test]
    fn test_variable_rename_resistant() {
        let code1 = r#"
fn calculate(x: i32) -> i32 {
    x * 2
}
"#;
        let code2 = r#"
fn compute(y: i32) -> i32 {
    y * 2
}
"#;
        let h1 = generate_ast_hashes(code1, Language::Rust).unwrap();
        let h2 = generate_ast_hashes(code2, Language::Rust).unwrap();
        let sim = ast_jaccard_similarity(&h1, &h2);
        // Should be highly similar despite different names
        assert!(sim > 0.5, "Expected similarity > 0.5, got {}", sim);
    }

    #[test]
    fn test_completely_different_code() {
        let code1 = "fn foo() -> i32 { 1 + 1 }";
        let code2 = "struct Bar { x: String, y: Vec<u8> }";
        let h1 = generate_ast_hashes(code1, Language::Rust).unwrap();
        let h2 = generate_ast_hashes(code2, Language::Rust).unwrap();
        let sim = ast_jaccard_similarity(&h1, &h2);
        assert!(sim < 0.5, "Expected low similarity, got {}", sim);
    }
}
