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

/// Simple language-agnostic lexer that produces token kinds
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

/// Generate winnowing fingerprints for source code
pub fn generate_fingerprints(source: &str, language: Language, k: usize, w: usize) -> Vec<u32> {
    let tokens = tokenize(source, language);
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
    hashes.into_iter().zip(line_map.into_iter()).collect()
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

    #[test]
    fn test_tokenize_simple() {
        let code = "fn main() {\n    let x = 42;\n}";
        let tokens = tokenize(code, Language::Rust);
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Keyword)); // fn, let
        assert!(kinds.contains(&TokenKind::Number));  // 42
    }

    #[test]
    fn test_winnowing_deterministic() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let fp1 = generate_fingerprints(code, Language::Rust, 5, 4);
        let fp2 = generate_fingerprints(code, Language::Rust, 5, 4);
        assert_eq!(fp1, fp2);
    }

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
}
