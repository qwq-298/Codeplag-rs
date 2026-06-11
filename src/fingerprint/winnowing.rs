use sha2::{Digest, Sha256};
use crate::core::types::Language;

/// Token types used for winnowing (language-agnostic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Keyword,
    Identifier,
    Number,
    String,
    Operator,
    Punctuation,
    Whitespace,
    Comment,
    Unknown,
}

/// A token produced by the lexer
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
}

/// Strip C-style comments from source code.
/// Removes `// line comments` and `/* block comments */`.
fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                // Line comment: skip until newline
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                // Keep the newline for line tracking
                if i < chars.len() {
                    result.push('\n');
                    i += 1;
                }
                continue;
            } else if chars[i + 1] == '*' {
                // Block comment: skip until */
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    if chars[i] == '\n' {
                        result.push('\n');
                    }
                    i += 1;
                }
                if i + 1 < chars.len() {
                    i += 2; // skip */
                }
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Normalize whitespace: collapse all whitespace runs into a single space,
/// then remove spaces around brackets/parens/braces/semicolons for uniform tokenization.
/// Comments are stripped first to prevent `//` from consuming the entire normalized line.
pub fn normalize_whitespace(source: &str) -> String {
    // Step 0: strip comments (must happen before newline removal!)
    let source = strip_comments(source);

    // Step 1: collapse all whitespace runs to a single space
    let mut result = String::with_capacity(source.len());
    let mut in_whitespace = false;
    for ch in source.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(ch);
            in_whitespace = false;
        }
    }
    // Step 2: strip spaces around brackets/parens/braces/semicolons/commas/dots/colons
    let brackets = ['(', ')', '[', ']', '{', '}', ';', ',', '.', ':'];
    let mut cleaned = String::with_capacity(result.len());
    let chars: Vec<char> = result.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == ' ' {
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let next = if i + 1 < chars.len() { chars[i + 1] } else { ' ' };
            let prev_is_id = prev.is_alphanumeric() || prev == '_';
            let next_is_id = next.is_alphanumeric() || next == '_';
            let prev_is_bracket = brackets.contains(&prev);
            let next_is_bracket = brackets.contains(&next);
            if (prev_is_id && next_is_id) || (!prev_is_bracket && !next_is_bracket) {
                cleaned.push(' ');
            }
        } else {
            cleaned.push(chars[i]);
        }
    }
    cleaned.trim().to_string()
}

/// Simple language-agnostic lexer that produces token kinds.
/// The source should be normalized with `normalize_whitespace` first
/// for format-independent results.
pub fn tokenize(source: &str, _language: Language) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1;

    while let Some(&ch) = chars.peek() {
        let start_line = line;

        match ch {
            '\n' => {
                line += 1;
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    text: "\n".to_string(),
                    line: start_line,
                });
            }
            ' ' | '\t' => {
                let ws: String = chars.by_ref().take_while(|c| c.is_whitespace() && *c != '\n').collect();
                tokens.push(Token {
                    kind: TokenKind::Whitespace,
                    text: ws,
                    line: start_line,
                });
            }
            '\r' => {
                // Skip carriage return — line endings are already normalized to \n
                chars.next();
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    let comment: String = chars.by_ref().take_while(|c| *c != '\n').collect();
                    tokens.push(Token {
                        kind: TokenKind::Comment,
                        text: format!("//{}", comment),
                        line: start_line,
                    });
                } else if chars.peek() == Some(&'*') {
                    chars.next();
                    let mut comment = String::from("/*");
                    while let Some(c) = chars.next() {
                        comment.push(c);
                        if c == '*' && chars.peek() == Some(&'/') {
                            comment.push(chars.next().unwrap());
                            break;
                        }
                        if c == '\n' { line += 1; }
                    }
                    tokens.push(Token {
                        kind: TokenKind::Comment,
                        text: comment,
                        line: start_line,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Operator,
                        text: "/".to_string(),
                        line: start_line,
                    });
                }
            }
            '"' => {
                chars.next();
                let mut s = String::from("\"");
                while let Some(c) = chars.next() {
                    s.push(c);
                    if c == '\\' {
                        if let Some(nc) = chars.next() { s.push(nc); }
                    } else if c == '"' {
                        break;
                    }
                    if c == '\n' { line += 1; }
                }
                tokens.push(Token {
                    kind: TokenKind::String,
                    text: s,
                    line: start_line,
                });
            }
            '0'..='9' => {
                let num: String = chars.by_ref().take_while(|c| c.is_alphanumeric() || *c == '.').collect();
                tokens.push(Token {
                    kind: TokenKind::Number,
                    text: num,
                    line: start_line,
                });
            }
            c if c.is_alphabetic() || c == '_' => {
                let word: String = chars.by_ref().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                let kind = if is_keyword(&word) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                tokens.push(Token {
                    kind,
                    text: word,
                    line: start_line,
                });
            }
            _ => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Punctuation,
                    text: ch.to_string(),
                    line: start_line,
                });
            }
        }
    }
    tokens
}

/// Check if a word is a common programming keyword
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else" | "for" | "while" | "do" | "switch" | "case"
            | "return" | "break" | "continue" | "fn" | "def" | "function"
            | "class" | "struct" | "enum" | "impl" | "trait" | "interface"
            | "let" | "var" | "const" | "mut" | "static" | "pub" | "private"
            | "public" | "protected" | "use" | "import" | "mod" | "package"
            | "match" | "try" | "catch" | "finally" | "throw" | "new"
            | "this" | "self" | "super" | "async" | "await" | "yield"
            | "true" | "false" | "null" | "None" | "nil" | "type" | "typeof"
            | "extends" | "implements" | "abstract" | "virtual" | "override"
    )
}

/// Compute k-gram hashes from token sequence.
/// Uses token text for keywords/operators (to distinguish different constructs)
/// and a placeholder for identifiers (to maintain renaming resistance).
pub fn compute_k_gram_hashes(tokens: &[Token], k: usize) -> Vec<u32> {
    // Filter out whitespace and comments — they cause false matches across blank lines
    let meaningful: Vec<u8> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Whitespace && t.kind != TokenKind::Comment)
        .map(|t| match t.kind {
            TokenKind::Keyword | TokenKind::Operator | TokenKind::Punctuation => {
                // Hash the actual text to distinguish different keywords/operators
                let mut h = Sha256::new();
                h.update(t.text.as_bytes());
                h.finalize()[0]
            }
            TokenKind::Identifier => 0xFF, // placeholder: all identifiers look the same
            _ => t.kind as u8, // numbers, strings, etc.
        })
        .collect();

    if meaningful.len() < k {
        return Vec::new();
    }

    let mut hashes = Vec::with_capacity(meaningful.len().saturating_sub(k - 1));
    let mut hasher = Sha256::new();

    for window in meaningful.windows(k) {
        hasher.update(window);
        let hash = u32::from_be_bytes(hasher.finalize_reset()[..4].try_into().unwrap());
        hashes.push(hash);
    }

    hashes
}

/// Apply winnowing: select minimum hash in each sliding window
pub fn winnow(hashes: &[u32], window_size: usize) -> Vec<u32> {
    if hashes.is_empty() || window_size == 0 {
        return Vec::new();
    }

    let mut fingerprints = Vec::new();
    let mut last_min_pos: isize = -1;

    for i in 0..hashes.len().saturating_sub(window_size - 1) {
        let window = &hashes[i..(i + window_size).min(hashes.len())];
        if window.is_empty() {
            continue;
        }

        let (min_idx_offset, &min_hash) = window
            .iter()
            .enumerate()
            .min_by_key(|&(_, &h)| h)
            .unwrap();

        let min_pos = (i + min_idx_offset) as isize;

        // Only record if it's a new position
        if min_pos != last_min_pos {
            fingerprints.push(min_hash);
            last_min_pos = min_pos;
        }
    }

    fingerprints
}

/// Generate winnowing fingerprints for source code.
/// Normalizes whitespace first for format-independent results.
pub fn generate_fingerprints(source: &str, language: Language, k: usize, w: usize) -> Vec<u32> {
    let normalized = normalize_whitespace(source);
    let tokens = tokenize(&normalized, language);
    let hashes = compute_k_gram_hashes(&tokens, k);
    winnow(&hashes, w)
}

/// Generate winnowing fingerprints with line number mapping.
/// Returns (hash, line_number) pairs for chunk matching.
pub fn generate_fingerprints_with_lines(
    source: &str,
    language: Language,
    k: usize,
    w: usize,
) -> Vec<(u32, usize)> {
    let tokens = tokenize(source, language);
    let (hashes, line_map) = compute_k_gram_hashes_with_lines(&tokens, k);
    let selected_indices = winnow_indices(&hashes, w);
    selected_indices
        .into_iter()
        .map(|idx| (hashes[idx], line_map[idx]))
        .collect()
}

/// Generate ALL k-gram hashes with line numbers (dense — for accurate chunk matching).
/// Returns (hash, line_number) for every k-gram in the file.
pub fn generate_all_kgraph_lines(
    source: &str,
    language: Language,
    k: usize,
) -> Vec<(u32, usize)> {
    let tokens = tokenize(source, language);
    let (hashes, line_map) = compute_k_gram_hashes_with_lines(&tokens, k);
    hashes.into_iter().zip(line_map).collect()
}

/// Token type indices for the frequency vector.
/// Order: Keyword, Identifier, Number, String, Operator, Punctuation
const TOKEN_TYPE_COUNT: usize = 6;

/// Compute token frequency vector for cosine similarity.
/// Counts meaningful token types (excluding Whitespace/Comment),
/// normalized by total token count.
pub fn compute_token_frequency(source: &str, language: Language) -> Vec<f64> {
    let tokens = tokenize(source, language);
    let mut counts = [0usize; TOKEN_TYPE_COUNT];

    let mut total = 0usize;
    for t in &tokens {
        let idx = match t.kind {
            TokenKind::Keyword => 0,
            TokenKind::Identifier => 1,
            TokenKind::Number => 2,
            TokenKind::String => 3,
            TokenKind::Operator => 4,
            TokenKind::Punctuation => 5,
            _ => continue, // skip Whitespace, Comment, Unknown
        };
        counts[idx] += 1;
        total += 1;
    }

    if total == 0 {
        return vec![0.0; TOKEN_TYPE_COUNT];
    }

    counts.iter().map(|&c| c as f64 / total as f64).collect()
}

/// Cosine similarity between two token frequency vectors.
/// Returns [0.0, 1.0] — 1.0 means identical token distribution.
pub fn token_cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return if mag_a == mag_b { 1.0 } else { 0.0 };
    }

    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

/// Compute k-gram hashes with line number tracking.
/// Returns (hashes, line_numbers) where line_numbers[i] is the line of the first token in k-gram i.
/// Whitespace and comment tokens are skipped entirely to avoid matching blank lines.
fn compute_k_gram_hashes_with_lines(tokens: &[Token], k: usize) -> (Vec<u32>, Vec<usize>) {
    let mut hashes = Vec::new();
    let mut line_map = Vec::new();

    // Filter out whitespace and comments — they cause false matches across blank lines
    let meaningful: Vec<(u8, usize)> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Whitespace && t.kind != TokenKind::Comment)
        .map(|t| {
            let byte = match t.kind {
                TokenKind::Keyword | TokenKind::Operator | TokenKind::Punctuation => {
                    let mut h = Sha256::new();
                    h.update(t.text.as_bytes());
                    h.finalize()[0]
                }
                TokenKind::Identifier => 0xFF,
                _ => t.kind as u8,
            };
            (byte, t.line)
        })
        .collect();

    if meaningful.len() < k {
        return (hashes, line_map);
    }

    let token_bytes: Vec<u8> = meaningful.iter().map(|(b, _)| *b).collect();
    let token_lines: Vec<usize> = meaningful.iter().map(|(_, l)| *l).collect();

    let mut hasher = Sha256::new();
    hashes.reserve(token_bytes.len().saturating_sub(k - 1));
    line_map.reserve(token_bytes.len().saturating_sub(k - 1));

    for i in 0..token_bytes.len().saturating_sub(k - 1) {
        let window = &token_bytes[i..(i + k).min(token_bytes.len())];
        hasher.update(window);
        let hash = u32::from_be_bytes(hasher.finalize_reset()[..4].try_into().unwrap());
        hashes.push(hash);
        // Line of the first meaningful token in this k-gram
        line_map.push(token_lines[i]);
    }

    (hashes, line_map)
}

/// Winnow: return indices (into the hashes slice) of selected fingerprints.
fn winnow_indices(hashes: &[u32], window_size: usize) -> Vec<usize> {
    if hashes.is_empty() || window_size == 0 {
        return Vec::new();
    }

    let mut indices = Vec::new();
    let mut last_min_pos: isize = -1;

    for i in 0..hashes.len().saturating_sub(window_size - 1) {
        let window = &hashes[i..(i + window_size).min(hashes.len())];
        if window.is_empty() {
            continue;
        }

        let (min_idx_offset, _) = window
            .iter()
            .enumerate()
            .min_by_key(|&(_, &h)| h)
            .unwrap();

        let min_pos = (i + min_idx_offset) as isize;

        if min_pos != last_min_pos {
            indices.push(i + min_idx_offset);
            last_min_pos = min_pos;
        }
    }

    indices
}

/// Calculate Jaccard similarity between two fingerprint sets
pub fn jaccard_similarity(a: &[u32], b: &[u32]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Sort and deduplicate for set operations
    let mut a_sorted: Vec<u32> = a.to_vec();
    let mut b_sorted: Vec<u32> = b.to_vec();
    a_sorted.sort_unstable();
    a_sorted.dedup();
    b_sorted.sort_unstable();
    b_sorted.dedup();

    let intersection = count_intersection(&a_sorted, &b_sorted);
    let union = a_sorted.len() + b_sorted.len() - intersection;

    intersection as f64 / union as f64
}

/// Count elements present in both sorted slices
fn count_intersection(a: &[u32], b: &[u32]) -> usize {
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

    // ── Comment Stripping ──────────────────────────────────────

    #[test]
    fn test_strip_comments() {
        assert_eq!(strip_comments("// comment\ncode"), "\ncode");
        assert_eq!(strip_comments("code /* block */ more"), "code  more");
        assert_eq!(strip_comments("// line\n/* block */\ncode"), "\n\ncode");
    }

    #[test]
    fn test_strip_multiline_comment() {
        let input = "start /* multi\nline\ncomment */ end";
        let output = strip_comments(input);
        assert!(!output.contains("multi"), "Multiline comment should be stripped");
        assert!(output.contains("end"), "Code after comment should remain");
    }

    #[test]
    fn test_strip_comments_no_comment() {
        let code = "fn main() { println!(\"hello\"); }";
        assert_eq!(strip_comments(code), code);
    }

    // ── Whitespace Normalization ───────────────────────────────

    #[test]
    fn test_normalize_whitespace() {
        let compact = "pub fn foo(x: i32) -> i32 {\n    x + 1\n}";
        let styled = "pub fn foo( x : i32 ) -> i32\n{\n    x + 1\n}\n";
        assert_eq!(normalize_whitespace(compact), normalize_whitespace(styled));
    }

    #[test]
    fn test_normalize_with_comments() {
        let with_comment = "// header\npub fn foo(x: i32) { x + 1 }";
        let no_comment = "pub fn foo(x: i32) { x + 1 }";
        assert_eq!(normalize_whitespace(with_comment), normalize_whitespace(no_comment));
    }

    #[test]
    fn test_normalize_bracket_spacing() {
        assert_eq!(normalize_whitespace("arr[j]"), "arr[j]");
        assert_eq!(normalize_whitespace("arr[ j ]"), "arr[j]");
        assert_eq!(normalize_whitespace("x ;"), "x;");
        assert_eq!(normalize_whitespace("0 .. n"), "0..n");
    }

    #[test]
    fn test_normalize_empty_string() {
        assert_eq!(normalize_whitespace(""), "");
    }

    // ── Tokenization ───────────────────────────────────────────

    #[test]
    fn test_tokenize_simple() {
        let code = "fn main() {\n    let x = 42;\n}";
        let tokens = tokenize(code, Language::Rust);
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Keyword)); // fn, let
        assert!(kinds.contains(&TokenKind::Number));  // 42
    }

    #[test]
    fn test_tokenize_string_literal() {
        // Test tokenization handles string-like content
        let code = "print(\"hello\")";
        let tokens = tokenize(code, Language::Python);
        // At minimum, tokenization should not panic and should produce tokens
        assert!(!tokens.is_empty(), "Should produce tokens for any code");
        // Verify string content is captured somehow
        let has_string_content = tokens.iter().any(|t| t.text.contains("hello"));
        assert!(has_string_content, "String content 'hello' should appear in tokens");
    }

    #[test]
    fn test_tokenize_operators() {
        // Most non-alphanumeric chars become Punctuation in this tokenizer
        // Only '/' becomes Operator (or starts a comment)
        let code = "a + b";
        let tokens = tokenize(code, Language::Rust);
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        // `+` is tokenized as Punctuation
        assert!(kinds.contains(&TokenKind::Punctuation), "Should have punctuation tokens");
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("", Language::Rust);
        assert!(tokens.is_empty());
    }

    // ── Format-Immune Fingerprinting ───────────────────────────

    #[test]
    fn test_format_immune_fingerprints() {
        let compact = "pub fn foo(x: i32) { x + 1 }";
        let styled = "pub fn foo( x : i32 )\n{\n    x + 1 \n}\n";
        let fp1 = generate_fingerprints(compact, Language::Rust, 5, 4);
        let fp2 = generate_fingerprints(styled, Language::Rust, 5, 4);
        assert_eq!(fp1, fp2, "Formatting changes should not affect fingerprints");
    }

    #[test]
    fn test_fingerprints_non_empty_for_code() {
        let code = "fn main() { let x = 1; let y = 2; let z = 3; let w = 4; let v = 5; }";
        let fp = generate_fingerprints(code, Language::Rust, 3, 2);
        assert!(!fp.is_empty(), "Fingerprints should not be empty for non-trivial code");
    }

    // ── k-gram Hashing ─────────────────────────────────────────

    #[test]
    fn test_kgram_hashes_basic() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let tokens = tokenize(code, Language::Rust);
        let hashes = compute_k_gram_hashes(&tokens, 3);
        assert!(!hashes.is_empty(), "k-gram hashes should be produced");
    }

    #[test]
    fn test_kgram_hashes_too_short() {
        let code = "x";
        let tokens = tokenize(code, Language::Rust);
        let hashes = compute_k_gram_hashes(&tokens, 5);
        assert!(hashes.is_empty(), "Should return empty when code is shorter than k");
    }

    // ── Winnowing ──────────────────────────────────────────────

    #[test]
    fn test_winnowing_deterministic() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let fp1 = generate_fingerprints(code, Language::Rust, 5, 4);
        let fp2 = generate_fingerprints(code, Language::Rust, 5, 4);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_winnow_reduces_hashes() {
        let hashes: Vec<u32> = (0..100).collect();
        let winnowed = winnow(&hashes, 10);
        assert!(winnowed.len() < hashes.len(), "Winnowing should reduce hash count");
    }

    #[test]
    fn test_winnow_no_duplicate_positions() {
        // Winnowing tracks positions to avoid recording the same position twice,
        // but the SAME hash value can appear at DIFFERENT positions legitimately
        let hashes: Vec<u32> = vec![5, 3, 1, 3, 5, 2, 4, 5, 1];
        let winnowed = winnow(&hashes, 3);
        // Should produce fewer hashes than input windows
        let expected_windows = hashes.len().saturating_sub(2); // window_size=3 → num_windows
        assert!(winnowed.len() <= expected_windows,
            "Winnowing should not produce more fingerprints than windows");
    }

    // ── Token Frequency ────────────────────────────────────────

    #[test]
    fn test_token_frequency_length() {
        let code = "fn main() { let x = 42; }";
        let freq = compute_token_frequency(code, Language::Rust);
        assert_eq!(freq.len(), 6, "Frequency vector should have 6 dimensions");
    }

    #[test]
    fn test_token_frequency_normalized() {
        let code = "fn main() { let x = 42; let y = 100; }";
        let freq = compute_token_frequency(code, Language::Rust);
        let sum: f64 = freq.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10 || sum == 0.0,
            "Frequency sum should be 1.0 or 0.0, got {}", sum);
    }

    #[test]
    fn test_token_frequency_empty_code() {
        let freq = compute_token_frequency("", Language::Rust);
        assert_eq!(freq, vec![0.0; 6], "Empty code should yield zero frequency vector");
    }

    // ── Cosine Similarity ──────────────────────────────────────

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0, 0.0, 0.0, 0.0];
        let sim = token_cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-10, "Identical vectors should have cosine=1.0");
    }

    #[test]
    fn test_cosine_zero_vectors() {
        let zero = vec![0.0; 6];
        let sim = token_cosine_similarity(&zero, &zero);
        // Implementation returns 1.0 when both magnitudes are 0 (identical zero vectors)
        assert!((sim - 1.0).abs() < 1e-10, "Two zero vectors should be treated as identical");
    }

    #[test]
    fn test_cosine_one_zero_vector() {
        let v = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let zero = vec![0.0; 6];
        let sim = token_cosine_similarity(&v, &zero);
        assert_eq!(sim, 0.0, "One zero vector should yield similarity 0.0");
    }

    // ── Jaccard Similarity ─────────────────────────────────────

    #[test]
    fn test_jaccard_identical() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![1, 2, 3, 4, 5];
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        assert!((jaccard_similarity(&a, &b) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_jaccard_partial_overlap() {
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 2.0 / 6.0).abs() < 1e-10,
            "Expected 2/6, got {}", sim);
    }

    #[test]
    fn test_jaccard_empty_inputs() {
        // Both empty = identical (by convention in this implementation)
        assert!((jaccard_similarity(&[], &[]) - 1.0).abs() < 1e-10);
        assert_eq!(jaccard_similarity(&[1, 2], &[]), 0.0);
        assert_eq!(jaccard_similarity(&[], &[1, 2]), 0.0);
    }

    #[test]
    fn test_jaccard_with_duplicates() {
        let a = vec![1, 1, 2, 2, 3];
        let b = vec![1, 2, 3];
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-10,
            "Duplicates should not affect Jaccard result");
    }

    // ── generate_fingerprints_with_lines ───────────────────────

    #[test]
    fn test_fingerprints_with_lines() {
        let code = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let fp = generate_fingerprints_with_lines(code, Language::Rust, 3, 2);
        // Each fingerprint should have a valid line number
        for &(_, line) in &fp {
            assert!(line > 0, "Line numbers should be positive");
        }
    }

    // ── Cross-language tokenization ────────────────────────────

    #[test]
    fn test_tokenize_python() {
        let code = "def foo(x):\n    return x + 1";
        let tokens = tokenize(code, Language::Python);
        assert!(!tokens.is_empty(), "Python code should tokenize");
    }

    #[test]
    fn test_tokenize_javascript() {
        let code = "function foo(x) { return x + 1; }";
        let tokens = tokenize(code, Language::JavaScript);
        assert!(!tokens.is_empty(), "JavaScript code should tokenize");
    }
}
