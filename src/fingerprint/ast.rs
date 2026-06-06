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

/// Recursively collect structural hashes from AST nodes.
/// Only hashes internal nodes (those with children) — leaf nodes like
/// identifiers and literals are too common and would inflate similarity.
fn collect_subtree_hashes(node: &Node, source: &str, hashes: &mut Vec<u64>) {
    // Only hash internal nodes (with children), skip leaf nodes
    if node.child_count() > 0 {
        let hash = structural_hash(node, source);
        hashes.push(hash);
    }

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

/// Extract function definitions from source code.
///
/// Uses tree-sitter to identify function/method nodes and returns
/// their names, source text, and line ranges.
pub fn extract_functions(source: &str, language: Language) -> Vec<crate::core::types::FunctionSnippet> {
    let ts_lang = match language.tree_sitter_language() {
        Some(l) => l,
        None => return Vec::new(),
    };

    let mut parser = Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return Vec::new();
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let root = tree.root_node();
    let mut functions = Vec::new();

    let function_kinds = match language {
        Language::Rust => &["function_item"][..],
        Language::Python => &["function_definition"][..],
        Language::JavaScript | Language::TypeScript => &[
            "function_declaration",
            "function_expression",
            "arrow_function",
            "method_definition",
        ],
        _ => return Vec::new(),
    };

    collect_function_nodes(&root, function_kinds, source, &mut functions);

    // Set the language on each function
    for f in &mut functions {
        f.language = language;
    }

    functions
}

/// Recursively find function nodes in the AST
fn collect_function_nodes(
    node: &Node,
    function_kinds: &[&str],
    source: &str,
    functions: &mut Vec<crate::core::types::FunctionSnippet>,
) {
    use crate::core::types::FunctionSnippet;

    if function_kinds.contains(&node.kind()) {
        // Extract function name.
        // For function_declaration/function_item: first identifier child is the name.
        // For arrow functions/expressions: might be assigned to a variable — leave as anonymous.
        let name = node
            .children(&mut node.walk())
            .find(|c| {
                c.kind() == "identifier"
                    || c.kind() == "property_identifier"
            })
            .and_then(|c| c.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // For Python/C, try alternate: the `name` field
                node.child_by_field_name("name")
                    .and_then(|c| c.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "anonymous".to_string())
            });

        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        let content: String = source
            .lines()
            .skip(start_line - 1)
            .take(end_line - start_line + 1)
            .map(|l| format!("{}\n", l))
            .collect();

        functions.push(FunctionSnippet {
            name,
            content,
            start_line,
            end_line,
            language: Language::Unknown, // filled in by caller
        });
        return; // don't recurse into nested functions (e.g., closures)
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_function_nodes(&child, function_kinds, source, functions);
        }
    }
}

/// Generate CFG fingerprint hashes for a source file.
///
/// Walks the AST to extract control flow structure, producing a set
/// of structural hashes that represent the control flow graph.
/// This captures the logical flow of the program independent of
/// variable names and exact syntax.
pub fn generate_cfg_hashes(source: &str, language: Language) -> Vec<u64> {
    let ts_lang = match language.tree_sitter_language() {
        Some(l) => l,
        None => return Vec::new(),
    };

    let mut parser = Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return Vec::new();
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let root = tree.root_node();

    // Collect control flow node sequences
    let mut patterns = Vec::new();
    collect_cfg_patterns(&root, &mut patterns);

    // Hash each pattern into a u64
    let mut hashes: Vec<u64> = patterns
        .iter()
        .map(|p| {
            let mut h = Sha256::new();
            h.update(p.as_bytes());
            u64::from_be_bytes(h.finalize()[..8].try_into().unwrap())
        })
        .collect();

    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

/// Control flow node kinds that create branches/jumps
fn is_control_flow(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement" | "if_expression"
            | "while_statement" | "while_expression"
            | "for_statement" | "for_expression"
            | "loop_statement" | "loop_expression"
            | "match_statement" | "match_expression"
            | "return_statement" | "return_expression"
            | "break_statement" | "continue_statement"
            | "try_statement" | "throw_statement"
            | "switch_statement" | "case_statement"
            | "function_item" | "function_definition"
            | "function_declaration" | "arrow_function"
            | "method_definition"
    )
}

/// Recursively collect CFG patterns: for each control-flow node,
/// record the pattern "parent_kind -> child_kind" and recurse.
fn collect_cfg_patterns(node: &Node, patterns: &mut Vec<String>) {
    let kind = node.kind().to_string();

    if is_control_flow(&kind) {
        // Record the control flow structure
        let mut children_kinds = Vec::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let ck = child.kind();
                if ck != "{" && ck != "}" && ck != "(" && ck != ")" {
                    children_kinds.push(ck.to_string());
                }
            }
        }
        if !children_kinds.is_empty() {
            patterns.push(format!("{}[{}]", kind, children_kinds.join(",")));
        } else {
            patterns.push(kind.clone());
        }
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_cfg_patterns(&child, patterns);
        }
    }
}

/// Compute Jaccard similarity between two CFG hash sets
pub fn cfg_jaccard_similarity(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let intersection = count_cfg_intersection(a, b);
    let union = a.len() + b.len() - intersection;
    intersection as f64 / union as f64
}

/// Count elements present in both sorted slices (CFG hashes)
fn count_cfg_intersection(a: &[u64], b: &[u64]) -> usize {
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
        // Should be 100% when only variable/function names differ
        assert!(sim > 0.99, "Expected 100% similarity, got {:.2}%. Hashes A: {:?}, B: {:?}", sim * 100.0, &h1[..5.min(h1.len())], &h2[..5.min(h2.len())]);
    }

    #[test]
    fn test_local_variable_rename_resistant() {
        let code1 = "let student_score = 100;";
        let code2 = "let x = 100;";
        let h1 = generate_ast_hashes(code1, Language::Rust).unwrap();
        let h2 = generate_ast_hashes(code2, Language::Rust).unwrap();
        let sim = ast_jaccard_similarity(&h1, &h2);
        assert!(sim > 0.99, "let with renamed var should be 100%, got {:.2}%", sim * 100.0);
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
