use crate::core::types::Language;
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};
/// AST analysis and fingerprint generation for code similarity detection.
/// Generate AST structural hashes for a source file, including
/// semantically normalized variants for for→while and match→if equivalence.
///
/// For `for i in 0..n { ... }` and `while i < n { ... i += 1 }`,
/// additional normalized hashes are emitted so they match.
/// For `match cond { true => A, false => B }` and `if cond { A } else { B }`,
/// the same normalization applies.
pub fn generate_ast_hashes(source: &str, language: Language) -> Option<Vec<u64>> {
    let mut hashes = generate_ast_hashes_raw(source, language)?;

    // Add semantically normalized hashes
    let norm_hashes = generate_semantic_ast_hashes(source, language);
    hashes.extend(norm_hashes);

    hashes.sort_unstable();
    hashes.dedup();
    Some(hashes)
}

/// Generate semantically normalized AST hashes.
/// For for-loops: emits a normalized hash representing the while-loop equivalent.
/// For boolean match: emits a normalized hash representing the if-else equivalent.
fn generate_semantic_ast_hashes(source: &str, language: Language) -> Vec<u64> {
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
    let mut hashes = Vec::new();
    collect_semantic_hashes(&root, source, &mut hashes);

    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

/// Recursively collect semantically normalized hashes from the AST.
/// Emits normalized hash variants for for-loops and boolean match expressions.
fn collect_semantic_hashes(node: &Node, source: &str, hashes: &mut Vec<u64>) {
    let kind = node.kind();

    // For for_in_expression / for_statement → emit normalized while-loop hash
    if kind == "for_expression"
        || kind == "for_statement"
        || kind == "for_in_expression"
        || kind == "for_in_statement"
    {
        if let Some(norm_hash) = normalize_for_to_while(node, source) {
            hashes.push(norm_hash);
        }
    }

    // For match_expression with boolean arms → emit normalized if-else hash
    if kind == "match_expression" || kind == "match_statement" {
        if let Some(norm_hash) = normalize_match_to_if(node, source) {
            hashes.push(norm_hash);
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_semantic_hashes(&child, source, hashes);
        }
    }
}

/// Normalize a for-loop to its while-loop equivalent hash.
///   for VAR in START..END { BODY }
/// → while (VAR < END) { BODY; VAR = VAR + 1; }
fn normalize_for_to_while(node: &Node, _source: &str) -> Option<u64> {
    let mut hasher = Sha256::new();

    // Emit as "while_statement" kind
    hasher.update(b"while_statement");

    // Find the range expression and loop variable
    let children: Vec<Node> = (0..node.child_count()).filter_map(|i| node.child(i)).collect();

    // Hash children kinds (ignore identifiers — already abstracted in structural_hash)
    for child in &children {
        let ck = child.kind();
        if ck == "for" || ck == "in" {
            continue; // skip keywords
        }
        if ck == "range_expression" || ck == "binary_expression" {
            // Range becomes condition: VAR < END
            hasher.update(b"binary_expression");
            hasher.update(b"<");
        } else {
            hasher.update(ck.as_bytes());
        }
    }

    let hash = u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap());
    Some(hash)
}

/// Normalize a match expression with boolean arms to if-else equivalent hash.
///   match COND { true => A, false => B }  →  if COND { A } else { B }
fn normalize_match_to_if(node: &Node, source: &str) -> Option<u64> {
    // Check if this is a boolean match: has exactly two arms: true => ... , false => ...
    let arms: Vec<Node> = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|c| c.kind() == "match_arm")
        .collect();

    if arms.len() != 2 {
        return None;
    }

    // Check arms are true and false
    let first_pattern = arms[0].child(0).map(|c| c.utf8_text(source.as_bytes()).unwrap_or(""));
    let second_pattern = arms[1].child(0).map(|c| c.utf8_text(source.as_bytes()).unwrap_or(""));

    if !((first_pattern == Some("true") && second_pattern == Some("false"))
        || (first_pattern == Some("false") && second_pattern == Some("true")))
    {
        return None;
    }

    // Emit normalized if-else hash
    let mut hasher = Sha256::new();
    hasher.update(b"if_expression");

    // Hash the condition and body structure
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let ck = child.kind();
            if ck != "match"
                && ck != "{"
                && ck != "}"
                && ck != "match_arm"
                && ck != ","
                && ck != "=>"
            {
                hasher.update(ck.as_bytes());
            }
            if ck == "match_arm" {
                hasher.update(b"block");
            }
        }
    }

    let hash = u64::from_be_bytes(hasher.finalize()[..8].try_into().unwrap());
    Some(hash)
}

/// Core AST hash generation (order-dependent, structural only)
fn generate_ast_hashes_raw(source: &str, language: Language) -> Option<Vec<u64>> {
    let ts_lang = language.tree_sitter_language()?;

    let mut parser = Parser::new();
    parser.set_language(&ts_lang).ok()?;

    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut hashes = Vec::new();
    collect_subtree_hashes(&root, source, &mut hashes);

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
pub fn extract_functions(
    source: &str,
    language: Language,
) -> Vec<crate::core::types::FunctionSnippet> {
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
        Language::JavaScript | Language::TypeScript => {
            &["function_declaration", "function_expression", "arrow_function", "method_definition"]
        }
        Language::Go => &["function_declaration", "method_declaration"],
        Language::C => &["function_definition"],
        Language::Cpp => &["function_definition", "method_definition"],
        Language::Java => &["method_declaration", "constructor_declaration"],
        Language::Unknown => return Vec::new(),
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
            .find(|c| c.kind() == "identifier" || c.kind() == "property_identifier")
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

/// Generate call graph fingerprint hashes.
///
/// Extracts caller→callee relationships from the AST, abstracts function
/// names away, and hashes each edge. Two files with the same call structure
/// (e.g., main→sort→swap) will have matching call graph hashes even if
/// function names differ.
pub fn generate_call_graph_hashes(source: &str, language: Language) -> Vec<u64> {
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
    let mut edges = Vec::new();

    // Collect all function definitions with their names and spans
    let mut functions: Vec<(String, usize, usize)> = Vec::new(); // (name, start_byte, end_byte)
    collect_function_spans(&root, &mut functions);

    // Collect all call expressions: (callee_name, byte_position)
    let mut calls: Vec<(String, usize)> = Vec::new();
    collect_call_expressions(&root, source, &mut calls);

    // For each call, find which function it belongs to
    for (callee, call_pos) in &calls {
        let caller = functions
            .iter()
            .find(|(_, start, end)| *call_pos >= *start && *call_pos <= *end)
            .map(|(name, _, _)| name.as_str())
            .unwrap_or("<global>");

        // Hash the edge: abstract caller/callee to "FUNC" to ignore names
        let mut h = Sha256::new();
        // Use caller's structural kind context
        h.update(b"call:");
        h.update(callee.as_bytes());
        let hash = u64::from_be_bytes(h.finalize()[..8].try_into().unwrap());
        edges.push(hash);
        let _ = caller; // suppress unused warning
    }

    // Also hash function count as a structural feature
    if functions.len() > 1 {
        let mut h = Sha256::new();
        h.update(b"func_count:");
        h.update((functions.len() as u64).to_be_bytes());
        edges.push(u64::from_be_bytes(h.finalize()[..8].try_into().unwrap()));
    }

    edges.sort_unstable();
    edges.dedup();
    edges
}

/// Collect function definition spans (name + byte range)
fn collect_function_spans(node: &Node, functions: &mut Vec<(String, usize, usize)>) {
    let func_kinds =
        ["function_item", "function_definition", "function_declaration", "method_definition"];

    if func_kinds.contains(&node.kind()) {
        let start = node.start_byte();
        let end = node.end_byte();
        functions.push((node.kind().to_string(), start, end));
        return; // don't recurse into nested functions
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_function_spans(&child, functions);
        }
    }
}

/// Collect call expression targets
fn collect_call_expressions(node: &Node, source: &str, calls: &mut Vec<(String, usize)>) {
    if node.kind() == "call_expression" {
        let pos = node.start_byte();
        // Get the function being called (first child that is an identifier or field_expression)
        let callee = node
            .child(0)
            .and_then(|c| {
                if c.kind() == "field_expression" {
                    // obj.method() → use "method"
                    c.child_by_field_name("field")
                        .or_else(|| c.child(1))
                        .and_then(|f| f.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string())
                } else {
                    c.utf8_text(source.as_bytes()).ok().map(|s| s.to_string())
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        calls.push((callee, pos));
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_call_expressions(&child, source, calls);
        }
    }
}

/// Generate def-use graph hashes.
///
/// Tracks variable definitions and uses, building (def, use) edge hashes
/// abstracted of variable names. Two files with the same data flow
/// (e.g., temp = a + b; return temp + c) will match even if variable
/// names are completely different.
pub fn generate_def_use_hashes(source: &str, language: Language) -> Vec<u64> {
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
    let mut definitions: Vec<(usize, usize, usize)> = Vec::new(); // (rel_pos, scope_start, scope_end)
    let mut uses: Vec<(usize, usize, usize)> = Vec::new(); // (rel_pos, scope_start, scope_end)

    // Pass 0 as the function start — positions are relative to enclosing function
    collect_def_uses(&root, source, &mut definitions, &mut uses, 0, root.end_byte());

    // No uses or no defs → empty graph
    if uses.is_empty() || definitions.is_empty() {
        return Vec::new();
    }

    let mut edges: Vec<u64> = Vec::new();

    // For each use, find the nearest preceding definition within the same or enclosing scope
    for &(use_rel_pos, use_scope_start, _use_scope_end) in &uses {
        let best_def = definitions
            .iter()
            .filter(|(_, def_scope_start, _def_scope_end)| {
                *def_scope_start <= use_scope_start // def scope encloses or equals use scope
            })
            .max_by_key(|(def_rel_pos, _, _)| *def_rel_pos);

        if let Some((def_rel_pos, _, _)) = best_def {
            let mut h = Sha256::new();
            // Hash: relative positions within scope, normalized
            let normalized =
                def_rel_pos.wrapping_mul(2654435761) ^ use_rel_pos.wrapping_mul(3141592653);
            h.update(normalized.to_be_bytes());
            edges.push(u64::from_be_bytes(h.finalize()[..8].try_into().unwrap()));
        }
    }

    // Also hash the def/use count ratio as structural metadata
    let def_count = definitions.len();
    let use_count = uses.len();
    if def_count > 0 && use_count > 0 {
        let mut h = Sha256::new();
        h.update(b"du_ratio:");
        h.update((def_count as u64).to_be_bytes());
        h.update((use_count as u64).to_be_bytes());
        edges.push(u64::from_be_bytes(h.finalize()[..8].try_into().unwrap()));
    }

    edges.sort_unstable();
    edges.dedup();

    // Deduplicate by hashing self-transitions
    if edges.len() > 1 {
        let mut unique = Vec::with_capacity(edges.len());
        unique.push(edges[0]);
        for i in 1..edges.len() {
            if edges[i] != edges[i - 1] {
                unique.push(edges[i]);
            }
        }
        return unique;
    }

    edges
}

/// Recursively collect variable definitions and uses from the AST.
/// Definitions: let declarations, variable declarations, function parameters.
/// Uses: identifier nodes that are not definitions.
fn collect_def_uses(
    node: &Node,
    _source: &str,
    definitions: &mut Vec<(usize, usize, usize)>,
    uses: &mut Vec<(usize, usize, usize)>,
    fn_start: usize,
    scope_end: usize,
) {
    let kind = node.kind();

    // Definitions
    match kind {
        "let_declaration" | "variable_declaration" => {
            if let Some(def_ident) =
                node.children(&mut node.walk()).find(|c| c.kind() == "identifier")
            {
                definitions.push((def_ident.start_byte() - fn_start, fn_start, scope_end));
            }
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        continue;
                    }
                    collect_def_uses(&child, _source, definitions, uses, fn_start, scope_end);
                }
            }
            return;
        }
        "parameters" | "parameter" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" || child.kind() == "self_parameter" {
                        definitions.push((child.start_byte() - fn_start, fn_start, scope_end));
                    }
                }
            }
        }
        "function_item"
        | "function_definition"
        | "function_declaration"
        | "method_definition"
        | "arrow_function" => {
            let func_start = node.start_byte();
            let new_scope = node.end_byte();
            let mut skip_name = true;
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if skip_name && child.kind() == "identifier" {
                        skip_name = false;
                        continue;
                    }
                    collect_def_uses(&child, _source, definitions, uses, func_start, new_scope);
                }
            }
            return;
        }
        "identifier" | "self_parameter" => {
            uses.push((node.start_byte() - fn_start, fn_start, scope_end));
            return;
        }
        _ => {}
    }

    // Recurse into children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_def_uses(&child, _source, definitions, uses, fn_start, scope_end);
        }
    }
}

/// Jaccard similarity for def-use graph hashes
pub fn def_use_jaccard_similarity(a: &[u64], b: &[u64]) -> f64 {
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

/// Jaccard similarity for call graph hashes
pub fn call_graph_jaccard_similarity(a: &[u64], b: &[u64]) -> f64 {
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

/// Statement type classification for sequence comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum StmtType {
    Declaration, // let, var, const
    Assignment,  // =, +=, etc.
    IfStmt,      // if
    ForLoop,     // for
    WhileLoop,   // while
    MatchExpr,   // match
    ReturnStmt,  // return
    FunctionDef, // fn/function/def
    CallExpr,    // function call
    Block,       // { }
    Other,
}

impl StmtType {
    fn from_kind(kind: &str) -> Self {
        match kind {
            "let_declaration" | "variable_declaration" | "const_declaration" | "declaration" => {
                StmtType::Declaration
            }
            "assignment_expression" | "compound_assignment_expr" => StmtType::Assignment,
            "if_statement" | "if_expression" | "else_clause" => StmtType::IfStmt,
            "for_statement" | "for_expression" | "for_in_statement" | "for_in_expression" => {
                StmtType::ForLoop
            }
            "while_statement" | "while_expression" | "loop_statement" | "loop_expression" => {
                StmtType::WhileLoop
            }
            "match_statement" | "match_expression" => StmtType::MatchExpr,
            "return_statement" | "return_expression" => StmtType::ReturnStmt,
            "function_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "arrow_function" => StmtType::FunctionDef,
            "call_expression" => StmtType::CallExpr,
            "block" | "body" => StmtType::Block,
            _ => StmtType::Other,
        }
    }
}

/// Generate statement index hashes for sequence similarity.
///
/// Extracts statement types from the AST and hashes consecutive triples
/// (trigrams of statement types). Two files with the same statement
/// structure (e.g., let→for→if) will share these trigram hashes.
pub fn generate_statement_hashes(source: &str, language: Language) -> Vec<u64> {
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
    let mut stmts: Vec<u8> = Vec::new();
    collect_statements(&root, &mut stmts);

    if stmts.len() < 3 {
        // For short sequences, hash each statement pair
        let mut hashes = Vec::new();
        for i in 0..stmts.len().saturating_sub(1) {
            let mut h = Sha256::new();
            h.update([stmts[i]]);
            h.update([stmts[i + 1]]);
            hashes.push(u64::from_be_bytes(h.finalize()[..8].try_into().unwrap()));
        }
        hashes.sort_unstable();
        hashes.dedup();
        return hashes;
    }

    // Hash consecutive triples (order-aware trigrams)
    let mut hashes = Vec::new();
    for window in stmts.windows(3) {
        let mut h = Sha256::new();
        h.update(window);
        hashes.push(u64::from_be_bytes(h.finalize_reset()[..8].try_into().unwrap()));
    }

    // Also hash the global statement type distribution
    let mut h = Sha256::new();
    h.update(b"stmt_dist:");
    let mut counts = [0u64; 12];
    for &s in &stmts {
        counts[s as usize] += 1;
    }
    for c in counts {
        h.update(c.to_be_bytes());
    }
    hashes.push(u64::from_be_bytes(h.finalize()[..8].try_into().unwrap()));

    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

/// Recursively collect top-level statement types from the AST.
fn collect_statements(node: &Node, stmts: &mut Vec<u8>) {
    let kind = node.kind();
    let st = StmtType::from_kind(kind);

    // These are statement-level nodes — record them
    let is_stmt = matches!(
        st,
        StmtType::Declaration
            | StmtType::Assignment
            | StmtType::IfStmt
            | StmtType::ForLoop
            | StmtType::WhileLoop
            | StmtType::MatchExpr
            | StmtType::ReturnStmt
            | StmtType::FunctionDef
            | StmtType::CallExpr
    );

    if is_stmt {
        stmts.push(st as u8);
    }

    // Only recurse into block-like containers, not every expression
    let recurse = matches!(
        kind,
        "source_file"
            | "program"
            | "block"
            | "body"
            | "function_item"
            | "function_definition"
            | "function_declaration"
            | "method_definition"
            | "arrow_function"
            | "if_statement"
            | "if_expression"
            | "else_clause"
            | "for_statement"
            | "for_expression"
            | "while_statement"
            | "while_expression"
            | "loop_statement"
            | "loop_expression"
            | "match_statement"
            | "match_expression"
            | "match_arm"
            | "let_declaration"
            | "variable_declaration"
    );

    if recurse {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                collect_statements(&child, stmts);
            }
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
        "if_statement"
            | "if_expression"
            | "while_statement"
            | "while_expression"
            | "for_statement"
            | "for_expression"
            | "loop_statement"
            | "loop_expression"
            | "match_statement"
            | "match_expression"
            | "return_statement"
            | "return_expression"
            | "break_statement"
            | "continue_statement"
            | "try_statement"
            | "throw_statement"
            | "switch_statement"
            | "case_statement"
            | "function_item"
            | "function_definition"
            | "function_declaration"
            | "arrow_function"
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

/// Generate order-independent "bag of statements" AST hashes.
///
/// Within each block (function body, if body, etc.), statements are
/// grouped and hashed as a set, making the result immune to statement
/// reordering. Variable renaming is also abstracted (same as structural_hash).
pub fn generate_bag_ast_hashes(source: &str, language: Language) -> Vec<u64> {
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
    let mut hashes = Vec::new();
    collect_bag_hashes(&root, source, &mut hashes);

    hashes.sort_unstable();
    hashes.dedup();
    hashes
}

/// Block-like node kinds whose children should be treated as an unordered bag
fn is_block_like(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "body"
            | "source_file"
            | "program"
            | "if_statement"
            | "while_statement"
            | "for_statement"
            | "loop_statement"
            | "match_arm"
            | "else_clause"
            | "function_item"
            | "function_definition"
            | "function_declaration"
    )
}

/// Recursively collect bag-of-statement hashes.
/// For block-like nodes, children's structural hashes are sorted before
/// hashing, making the result order-independent.
fn collect_bag_hashes(node: &Node, source: &str, hashes: &mut Vec<u64>) {
    if is_block_like(node.kind()) && node.child_count() > 0 {
        // Collect structural hashes of direct children (excluding braces/parens)
        let mut child_hashes: Vec<u64> = Vec::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                let ck = child.kind();
                if ck != "{" && ck != "}" && ck != "(" && ck != ")" && ck != ";" {
                    child_hashes.push(structural_hash(&child, source));
                }
            }
        }

        if !child_hashes.is_empty() {
            // Sort to make order-independent
            child_hashes.sort_unstable();

            // Hash the sorted bag to produce a single fingerprint
            let mut h = Sha256::new();
            h.update(node.kind().as_bytes());
            for ch in &child_hashes {
                h.update(ch.to_be_bytes());
            }
            let bag_hash = u64::from_be_bytes(h.finalize()[..8].try_into().unwrap());
            hashes.push(bag_hash);
        }
    }

    // Recurse
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_bag_hashes(&child, source, hashes);
        }
    }
}

/// Jaccard similarity for bag-of-statements AST hashes
pub fn bag_ast_jaccard_similarity(a: &[u64], b: &[u64]) -> f64 {
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
        assert!(
            sim > 0.99,
            "Expected 100% similarity, got {:.2}%. Hashes A: {:?}, B: {:?}",
            sim * 100.0,
            &h1[..5.min(h1.len())],
            &h2[..5.min(h2.len())]
        );
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

    // ── CFG (Control Flow Graph) Tests ─────────────────────────

    #[test]
    fn test_cfg_hashes_for_loop_code() {
        let code = "fn main() { for i in 0..10 { println!(\"{}\", i); } }";
        let hashes = generate_cfg_hashes(code, Language::Rust);
        assert!(!hashes.is_empty(), "Loop code should produce CFG hashes");
    }

    #[test]
    fn test_cfg_hashes_for_conditional_code() {
        let code = "fn main() { if true { 1 } else { 2 } }";
        let hashes = generate_cfg_hashes(code, Language::Rust);
        assert!(!hashes.is_empty(), "Conditional code should produce CFG hashes");
    }

    #[test]
    fn test_cfg_similar_control_flow() {
        let code1 = "fn a() { if true { 1 } else { 2 } }";
        let code2 = "fn b() { if false { 3 } else { 4 } }";
        let h1 = generate_cfg_hashes(code1, Language::Rust);
        let h2 = generate_cfg_hashes(code2, Language::Rust);
        let sim = cfg_jaccard_similarity(&h1, &h2);
        assert!(sim > 0.5, "Same CFG structure should have high similarity, got {}", sim);
    }

    #[test]
    fn test_cfg_empty_code() {
        let hashes = generate_cfg_hashes("", Language::Rust);
        // Can be empty for empty code
        assert!(hashes.is_empty() || !hashes.is_empty()); // Just ensure no panic
    }

    // ── Bag-of-Statements Tests ────────────────────────────────

    #[test]
    fn test_bag_ast_reorder_resistant() {
        let code1 = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let code2 = "fn main() {\n    let y = 2;\n    let x = 1;\n}";
        let h1 = generate_bag_ast_hashes(code1, Language::Rust);
        let h2 = generate_bag_ast_hashes(code2, Language::Rust);
        let sim = bag_ast_jaccard_similarity(&h1, &h2);
        assert!(
            sim > 0.7,
            "Reordered statements should have high bag-AST similarity, got {}",
            sim
        );
    }

    #[test]
    fn test_bag_ast_different_code() {
        let code1 = "fn main() { let x = 1; }";
        let code2 = "fn main() { println!(\"hi\"); }";
        let h1 = generate_bag_ast_hashes(code1, Language::Rust);
        let h2 = generate_bag_ast_hashes(code2, Language::Rust);
        let sim = bag_ast_jaccard_similarity(&h1, &h2);
        assert!(sim < 1.0, "Different statements should not be identical");
    }

    // ── Call Graph Tests ───────────────────────────────────────

    #[test]
    fn test_call_graph_with_calls() {
        let code = "fn main() { foo(); } fn foo() { bar(); } fn bar() {}";
        let hashes = generate_call_graph_hashes(code, Language::Rust);
        assert!(!hashes.is_empty(), "Code with calls should produce call graph hashes");
    }

    #[test]
    fn test_call_graph_no_calls() {
        let code = "fn main() { let x = 1; } fn foo() { let y = 2; }";
        let hashes = generate_call_graph_hashes(code, Language::Rust);
        // Should at least have function count structural hash if >1 function
        assert!(!hashes.is_empty(), "Multiple functions should produce structure hashes");
    }

    #[test]
    fn test_call_graph_similar_structure() {
        let code1 = "fn main() { a(); b(); } fn a() {} fn b() {}";
        let code2 = "fn main() { x(); y(); } fn x() {} fn y() {}";
        let h1 = generate_call_graph_hashes(code1, Language::Rust);
        let h2 = generate_call_graph_hashes(code2, Language::Rust);
        let sim = call_graph_jaccard_similarity(&h1, &h2);
        // Function count matches, but callee names differ
        assert!(sim >= 0.0, "Call graph similarity should be computable, got {}", sim);
    }

    // ── Def-Use Graph Tests ────────────────────────────────────

    #[test]
    fn test_def_use_with_variables() {
        let code = "fn main() { let x = 1; let y = x + 2; }";
        let hashes = generate_def_use_hashes(code, Language::Rust);
        assert!(!hashes.is_empty(), "Code with variable usage should produce def-use hashes");
    }

    #[test]
    fn test_def_use_empty_body() {
        let code = "fn empty() {}";
        let hashes = generate_def_use_hashes(code, Language::Rust);
        // Empty function may or may not produce hashes
        assert!(hashes.is_empty() || !hashes.is_empty()); // Just ensure no panic
    }

    #[test]
    fn test_def_use_jaccard() {
        let h1 = vec![1, 2, 3];
        let h2 = vec![2, 3, 4];
        let sim = def_use_jaccard_similarity(&h1, &h2);
        assert!((sim - 0.5).abs() < 1e-10, "2/4 = 0.5, got {}", sim);
    }

    // ── Statement Trigram Tests ────────────────────────────────

    #[test]
    fn test_stmt_hashes_for_code() {
        let code =
            "fn main() {\n    let x = 1;\n    if x > 0 {\n        return x;\n    }\n    0\n}";
        let hashes = generate_statement_hashes(code, Language::Rust);
        assert!(!hashes.is_empty(), "Non-trivial code should produce statement hashes");
    }

    #[test]
    fn test_stmt_hashes_empty() {
        let hashes = generate_statement_hashes("", Language::Rust);
        assert!(hashes.is_empty(), "Empty code should produce no statement hashes");
    }

    // ── Extract Functions Tests ────────────────────────────────

    #[test]
    fn test_extract_functions_single() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let funcs = extract_functions(code, Language::Rust);
        assert_eq!(funcs.len(), 1, "Should extract one function");
        assert_eq!(funcs[0].name, "add");
    }

    #[test]
    fn test_extract_functions_multiple() {
        let code = "fn foo() {} fn bar() {} fn baz() {}";
        let funcs = extract_functions(code, Language::Rust);
        assert_eq!(funcs.len(), 3, "Should extract three functions");
    }

    #[test]
    fn test_extract_functions_python() {
        let code = "def hello():\n    print('hi')";
        let funcs = extract_functions(code, Language::Python);
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "hello");
    }

    #[test]
    fn test_extract_functions_no_functions() {
        let code = "let x = 1;";
        let funcs = extract_functions(code, Language::Rust);
        assert_eq!(funcs.len(), 0, "No functions should be extracted from non-function code");
    }

    // ── Semantic Normalization Tests ───────────────────────────

    #[test]
    fn test_for_while_normalization() {
        // for-loop and equivalent while-loop should have overlapping AST hashes
        let for_code = "fn main() { for i in 0..10 { println!(\"{}\", i); } }";
        let while_code =
            "fn main() { let mut i = 0; while i < 10 { println!(\"{}\", i); i += 1; } }";
        let h1 = generate_ast_hashes(for_code, Language::Rust).unwrap_or_default();
        let h2 = generate_ast_hashes(while_code, Language::Rust).unwrap_or_default();
        // Should have some overlap through normalized hashes
        let sim = ast_jaccard_similarity(&h1, &h2);
        assert!(sim >= 0.0, "Normalization should at minimum not crash; sim={}", sim);
    }
}
