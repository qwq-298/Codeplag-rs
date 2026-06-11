use clap::Parser;
use codeplag::cli::{Cli, Commands};
use codeplag::core::types::AnalyzerConfig;
use codeplag::engine::{FingerprintCache, SimilarityEngine};
use codeplag::fetcher::github::GitHubFetcher;
use std::path::PathBuf;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if cli.verbose {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let config = AnalyzerConfig {
        k_gram_size: cli.k_gram,
        window_size: cli.window,
        threshold: cli.threshold,
        ..Default::default()
    };

    let work_dir = PathBuf::from(".codeplag_work");

    match &cli.command {
        Commands::Analyze { path, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files = fetcher.collect_local(path).unwrap_or_else(|e| {
                eprintln!("Error collecting files: {}", e);
                std::process::exit(1);
            });

            println!("Found {} source files in {}", files.len(), path);

            if cli.functions {
                let engine = SimilarityEngine::new(config)
                    .with_cache(FingerprintCache::new(".codeplag_cache"));
                let func_results = engine.compare_functions(&files);
                print_function_results(&func_results, output);
            } else {
                let mut engine = SimilarityEngine::new(config)
                    .with_cache(FingerprintCache::new(".codeplag_cache"));
                engine.index_files(&files);
                let results = engine.compare_all();

                let file_refs: Vec<&codeplag::core::types::SourceFile> = files.iter().collect();
                print_results_with_files(&results, output, &file_refs);
            }
        }

        Commands::Compare { file, against, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let target_files = fetcher.collect_local(file).unwrap_or_else(|e| {
                eprintln!("Error reading target file: {}", e);
                std::process::exit(1);
            });

            let against_files = fetcher.collect_local(against).unwrap_or_else(|e| {
                eprintln!("Error collecting comparison files: {}", e);
                std::process::exit(1);
            });

            if target_files.is_empty() {
                eprintln!("No valid source file found at: {}", file);
                std::process::exit(1);
            }

            if against_files.len() == 1 {
                println!("Comparing {} <-> {}", target_files[0].path, against_files[0].path);
            } else {
                println!(
                    "Comparing {} against {} files in {}",
                    target_files[0].path,
                    against_files.len(),
                    against
                );
            }

            if cli.functions {
                // Function-level: merge target + against and compare all functions
                let all_files: Vec<codeplag::core::types::SourceFile> =
                    target_files.iter().chain(against_files.iter()).cloned().collect();
                let engine = SimilarityEngine::new(config)
                    .with_cache(FingerprintCache::new(".codeplag_cache"));
                let func_results = engine.compare_functions(&all_files);
                // Filter to only target vs against (not against vs against)
                let filtered: Vec<_> = func_results
                    .into_iter()
                    .filter(|r| {
                        r.file_a == target_files[0].path || r.file_b == target_files[0].path
                    })
                    .collect();
                print_function_results(&filtered, output);
            } else {
                let mut engine = SimilarityEngine::new(config)
                    .with_cache(FingerprintCache::new(".codeplag_cache"));
                engine.index_files(&against_files);
                let results = engine.compare_against(&target_files[0]);

                let all_files: Vec<&codeplag::core::types::SourceFile> =
                    target_files.iter().chain(against_files.iter()).collect();
                print_results_with_files(&results, output, &all_files);
            }
        }

        Commands::Fetch { repo, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files = fetcher.fetch_repo(repo).unwrap_or_else(|e| {
                eprintln!("Error fetching repository: {}", e);
                std::process::exit(1);
            });

            println!("Fetched {} source files from {}", files.len(), repo);

            let mut engine =
                SimilarityEngine::new(config).with_cache(FingerprintCache::new(".codeplag_cache"));
            engine.index_files(&files);
            let results = engine.compare_all();

            let file_refs: Vec<&codeplag::core::types::SourceFile> = files.iter().collect();
            print_results_with_files(&results, output, &file_refs);
        }

        Commands::Batch { repos, output } => {
            if repos.len() < 2 {
                eprintln!("Need at least 2 repos for batch comparison");
                std::process::exit(1);
            }

            println!("Batch fetching {} repositories...\n", repos.len());

            // Step 1: Fetch all repos
            let mut projects: Vec<(String, Vec<codeplag::core::types::SourceFile>)> = Vec::new();
            for repo_url in repos {
                let fetcher = GitHubFetcher::new(&work_dir);
                let name = repo_url
                    .trim_end_matches(".git")
                    .split('/')
                    .next_back()
                    .unwrap_or("unknown")
                    .to_string();
                print!("  Fetching {}... ", name);
                let files = fetcher.fetch_repo(repo_url).unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                });
                println!("{} files", files.len());
                projects.push((name, files));
            }

            // Step 2: Compare all pairs
            println!("\nComparing all pairs...\n");
            let engine =
                SimilarityEngine::new(config).with_cache(FingerprintCache::new(".codeplag_cache"));
            let mut results: Vec<(String, String, codeplag::core::types::ProjectResult)> =
                Vec::new();

            for i in 0..projects.len() {
                for j in (i + 1)..projects.len() {
                    let (name_a, files_a) = &projects[i];
                    let (name_b, files_b) = &projects[j];
                    let result = engine.compare_projects(files_a, files_b);
                    results.push((name_a.clone(), name_b.clone(), result));
                }
            }

            // Sort by project score descending
            results.sort_by(|(_, _, a), (_, _, b)| {
                b.project_score
                    .partial_cmp(&a.project_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Step 3: Display
            print_batch_results(&results, output);
        }

        Commands::Search { path, repo, limit, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);

            // Step 1: Get source files (from local path or GitHub repo)
            let (local_files, source_label) = if let Some(ref repo_url) = repo {
                println!("Fetching repository: {}\n", repo_url);
                let files = fetcher.fetch_repo(repo_url).unwrap_or_else(|e| {
                    eprintln!("Error fetching repo: {}", e);
                    std::process::exit(1);
                });
                (files, repo_url.clone())
            } else if let Some(ref local_path) = path {
                let files = fetcher.collect_local(local_path).unwrap_or_else(|e| {
                    eprintln!("Error reading {}: {}", local_path, e);
                    std::process::exit(1);
                });
                (files, local_path.clone())
            } else {
                eprintln!("Either --path or --repo must be provided.");
                std::process::exit(1);
            };

            if local_files.is_empty() {
                eprintln!("No source files found.");
                std::process::exit(1);
            }

            println!(
                "Searching GitHub for code similar to: {} ({} files)\n",
                source_label,
                local_files.len()
            );

            // Step 2: Extract search terms from ALL files, tracking language
            let mut term_lang_pairs: Vec<(String, String)> = Vec::new(); // (term, lang_name)
            let mut seen = std::collections::HashSet::new();
            for file in &local_files {
                let lang_str = match file.language {
                    codeplag::core::types::Language::Rust => "rust",
                    codeplag::core::types::Language::Python => "python",
                    codeplag::core::types::Language::JavaScript => "javascript",
                    _ => continue,
                };
                for term in extract_search_terms(&file.content, lang_str) {
                    if seen.insert(term.clone()) {
                        term_lang_pairs.push((term, lang_str.to_string()));
                    }
                }
            }

            // Pick mix of terms: some specific, some common
            // Sort by length: shorter = more common = better search results
            term_lang_pairs.sort_by_key(|a| a.0.len());
            // Take a mix: first few (most common) + last few (most specific)
            let mut picked: Vec<(String, String)> = Vec::new();
            let n = term_lang_pairs.len();
            for (term, lang) in term_lang_pairs.iter().take(3.min(n)) {
                picked.push((term.clone(), lang.clone())); // shorter terms
            }
            for (term, lang) in term_lang_pairs.iter().skip(n.saturating_sub(3)) {
                let pair = (term.clone(), lang.clone());
                if !picked.contains(&pair) {
                    picked.push(pair); // longer terms
                }
            }
            picked.truncate(8);

            let term_list: Vec<String> = picked.iter().map(|(t, _)| t.clone()).collect();
            println!("  Search terms: {}\n", term_list.join(", "));

            if picked.is_empty() {
                eprintln!("No searchable terms found.");
                std::process::exit(1);
            }

            // Step 3: Search GitHub for each term with its correct language
            let mut all_snippets: Vec<(String, String, String)> = Vec::new(); // (url, repo_name, content)
            let client = reqwest::blocking::Client::builder()
                .user_agent("codeplag-analyzer/1.0")
                .build()
                .unwrap();

            let primary_lang =
                &picked.first().map(|(_, l)| l.clone()).unwrap_or_else(|| "rust".into());
            for (term, lang) in picked.iter().take(6) {
                let query = format!("{} language:{}", term, lang);
                let encoded_query = query.replace(' ', "%20").replace(':', "%3A");
                let url = format!(
                    "https://api.github.com/search/code?q={}&per_page={}",
                    encoded_query, *limit
                );
                print!("  Searching: {}... ", query);

                match search_github_code(&client, &url, *limit, cli.github_token.as_deref()) {
                    Ok(snippets) => {
                        println!("{} results", snippets.len());
                        all_snippets.extend(snippets);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }

            if all_snippets.is_empty() {
                println!("\nNo matching code found on GitHub.");
                return;
            }

            println!("\n  Total snippets fetched: {}\n", all_snippets.len());

            // Step 4: Group snippets by repo, run project-level comparison
            let primary_lang: codeplag::core::types::Language = match primary_lang.as_str() {
                "rust" => codeplag::core::types::Language::Rust,
                "python" => codeplag::core::types::Language::Python,
                "javascript" => codeplag::core::types::Language::JavaScript,
                _ => codeplag::core::types::Language::Rust,
            };

            let mut repo_files: std::collections::HashMap<
                String,
                Vec<codeplag::core::types::SourceFile>,
            > = std::collections::HashMap::new();
            for (url, repo_name, content) in &all_snippets {
                let source_file = codeplag::core::types::SourceFile {
                    path: url.clone(),
                    content: content.clone(),
                    language: primary_lang,
                    size: content.len(),
                };
                repo_files.entry(repo_name.clone()).or_default().push(source_file);
            }

            let mut results: Vec<(String, String, f64)> = Vec::new(); // (repo, file, score)

            for (repo_name, gh_files) in &repo_files {
                let engine = SimilarityEngine::new(AnalyzerConfig {
                    threshold: 0.0,
                    k_gram_size: config.k_gram_size,
                    window_size: config.window_size,
                    ..Default::default()
                });

                let pr = engine.compare_projects(gh_files, &local_files);

                // Track individual file matches
                for m in &pr.file_matches {
                    if m.similarity_score > 0.15 {
                        results.push((repo_name.clone(), m.file_a.clone(), m.similarity_score));
                    }
                }
            }

            // Sort by score descending
            results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(*limit * 3);

            match output.as_str() {
                "json" => {
                    let json: Vec<serde_json::Value> = results.iter().map(|(repo, file, score)| {
                            serde_json::json!({"repo": repo, "file": file, "similarity": score})
                        }).collect();
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
                _ => {
                    println!("=== GitHub Search Results ===\n");
                    for (i, (repo, file, score)) in results.iter().enumerate() {
                        let bar = if *score >= 0.8 {
                            "🔴"
                        } else if *score >= 0.5 {
                            "🟡"
                        } else {
                            "🟢"
                        };
                        println!(
                            "  {:>2}. {:>5.1}%  {:<25} ↔ {}  {}",
                            i + 1,
                            score * 100.0,
                            truncate_name(file, 25),
                            truncate_name(repo, 25),
                            bar,
                        );
                    }
                    if results.is_empty() {
                        println!("  No similar code found.");
                    }

                    // Summary
                    let high_matches = results.iter().filter(|(_, _, s)| *s >= 0.5).count();
                    println!("\n  Found {} matches ({} above 50%)", results.len(), high_matches);
                }
            }
        }

        Commands::Project { project_a, project_b, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files_a = fetcher.collect_local(project_a).unwrap_or_else(|e| {
                eprintln!("Error reading project A: {}", e);
                std::process::exit(1);
            });
            let files_b = fetcher.collect_local(project_b).unwrap_or_else(|e| {
                eprintln!("Error reading project B: {}", e);
                std::process::exit(1);
            });

            println!(
                "Comparing project A ({} files) vs project B ({} files)",
                files_a.len(),
                files_b.len()
            );

            let engine =
                SimilarityEngine::new(config).with_cache(FingerprintCache::new(".codeplag_cache"));
            let result = engine.compare_projects(&files_a, &files_b);
            print_project_result(&result, output);
        }
    }
}

fn print_results_with_files(
    results: &[codeplag::core::types::SimilarityResult],
    format: &str,
    all_files: &[&codeplag::core::types::SourceFile],
) {
    // Build content lookup by path
    let content_map: std::collections::HashMap<&str, &str> =
        all_files.iter().map(|f| (f.path.as_str(), f.content.as_str())).collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(results)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if results.is_empty() {
                println!("\nNo similar file pairs found.");
                return;
            }

            println!("\n=== Similarity Results ===\n");
            for (i, result) in results.iter().enumerate() {
                println!("{}. {} <-> {}", i + 1, result.file_a, result.file_b);
                println!("   Overall:  {:.1}%", result.similarity_score * 100.0);
                println!(
                    "   Winnowing: {:.1}% | AST: {:.1}%",
                    result.winnowing_score * 100.0,
                    result.ast_score * 100.0
                );

                // Display matched chunks
                if !result.matched_chunks.is_empty() {
                    let lines_a: Vec<&str> = content_map
                        .get(result.file_a.as_str())
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    let lines_b: Vec<&str> = content_map
                        .get(result.file_b.as_str())
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    print_chunks(&result.matched_chunks, &lines_a, &lines_b);
                }
                println!();
            }

            println!("Found {} similar file pairs.", results.len());
        }
    }
}

/// Display matched chunks in a visual side-by-side format
fn print_chunks(chunks: &[codeplag::core::types::ChunkMatch], lines_a: &[&str], lines_b: &[&str]) {
    const MAX_WIDTH: usize = 50;

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        println!();
        println!(
            "   ── Chunk {} ({:.0}% match, lines {}-{} ↔ {}-{}) ──",
            chunk_idx + 1,
            chunk.score * 100.0,
            chunk.line_a,
            chunk.line_end_a,
            chunk.line_b,
            chunk.line_end_b,
        );

        let max_lines =
            (chunk.line_end_a - chunk.line_a + 1).max(chunk.line_end_b - chunk.line_b + 1);

        for offset in 0..max_lines {
            let idx_a = chunk.line_a.saturating_sub(1).saturating_add(offset);
            let idx_b = chunk.line_b.saturating_sub(1).saturating_add(offset);

            let left = format!(
                "{:>4} | {}",
                idx_a + 1,
                truncate(lines_a.get(idx_a).copied().unwrap_or(""), MAX_WIDTH)
            );
            let right = format!(
                "{:>4} | {}",
                idx_b + 1,
                truncate(lines_b.get(idx_b).copied().unwrap_or(""), MAX_WIDTH)
            );

            println!("   {:<54}  {}", left, right);
        }
    }
}

/// Display function-level comparison results
fn print_function_results(results: &[codeplag::core::types::FunctionMatch], format: &str) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(results)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if results.is_empty() {
                println!("\nNo similar functions found.");
                return;
            }

            println!("\n=== Function-Level Similarity Results ===\n");
            for (i, r) in results.iter().enumerate() {
                println!(
                    "{}: {}() [{}:{}-{}] ↔ {}() [{}:{}-{}]",
                    i + 1,
                    r.func_a,
                    r.file_a,
                    r.lines_a.0,
                    r.lines_a.1,
                    r.func_b,
                    r.file_b,
                    r.lines_b.0,
                    r.lines_b.1,
                );
                println!(
                    "   Overall:  {:.1}%  |  Winnowing: {:.1}%  |  AST: {:.1}%",
                    r.similarity_score * 100.0,
                    r.winnowing_score * 100.0,
                    r.ast_score * 100.0,
                );
                println!();
            }

            println!("Found {} similar function pairs.", results.len());
        }
    }
}

/// Display project-level comparison results
fn print_project_result(result: &codeplag::core::types::ProjectResult, format: &str) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(result)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if result.file_matches.is_empty() {
                println!("\nNo similar files found between the two projects.");
                return;
            }

            println!("\n=== Project Comparison Results ===\n");

            for (i, m) in result.file_matches.iter().enumerate() {
                println!("{:>3}. {: <40} ↔ {}", i + 1, m.file_a, m.file_b,);
                println!(
                    "      {:.1}%  |  Winnowing: {:.1}%  |  AST: {:.1}%",
                    m.similarity_score * 100.0,
                    m.winnowing_score * 100.0,
                    m.ast_score * 100.0,
                );
                println!();
            }

            println!("─────────────────────────────────────────────");
            println!(
                "  Project Similarity: {:.1}%  (avg of {} file matches)",
                result.project_score * 100.0,
                result.file_matches.len(),
            );
        }
    }
}

/// Display batch comparison results
fn print_batch_results(
    results: &[(String, String, codeplag::core::types::ProjectResult)],
    format: &str,
) {
    match format {
        "json" => {
            let output: Vec<serde_json::Value> = results
                .iter()
                .map(|(a, b, r)| {
                    serde_json::json!({
                        "project_a": a,
                        "project_b": b,
                        "project_score": r.project_score,
                                        "matches_count": r.file_matches.len(),
                                        "file_matches": r.file_matches,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => {
            if results.is_empty() {
                println!("\nNo results.");
                return;
            }

            println!("=== Batch Comparison Results ===\n");
            println!(
                "  {:>3}  {:<20} {:<20} {:>8}  {:>10}",
                "#", "Repository A", "Repository B", "Score", "Matches"
            );
            println!("  ───────────────────────────────────────────────────────────────");

            for (i, (name_a, name_b, result)) in results.iter().enumerate() {
                let bar = match result.project_score {
                    s if s >= 0.8 => "🔴",
                    s if s >= 0.5 => "🟡",
                    _ => "🟢",
                };
                println!(
                    "  {:>3}  {:<20} {:<20} {:>6.1}%  {:>4} {}",
                    i + 1,
                    truncate_name(name_a, 20),
                    truncate_name(name_b, 20),
                    result.project_score * 100.0,
                    result.file_matches.len(),
                    bar,
                );
            }

            // Detail for top 3 pairs
            println!("\n  Top matches detail:\n");
            let top_n = results.len().min(3);
            for (name_a, name_b, result) in results.iter().take(top_n) {
                println!(
                    "  {} ↔ {}  ({:.1}% overall)",
                    name_a,
                    name_b,
                    result.project_score * 100.0
                );
                for m in result.file_matches.iter().take(5) {
                    println!(
                        "    ✓ {: <30} ↔ {: <30}  {:.1}%",
                        truncate_name(&m.file_a, 30),
                        truncate_name(&m.file_b, 30),
                        m.similarity_score * 100.0,
                    );
                }
                if result.file_matches.len() > 5 {
                    println!("    ... and {} more files", result.file_matches.len() - 5);
                }
                println!();
            }
        }
    }
}

/// Extract searchable terms from code content (function names, unique literals)
fn extract_search_terms(content: &str, lang: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut terms = HashSet::new();

    let is_rust = lang == "rust" || lang == "rs";
    let is_python = lang == "python" || lang == "py";
    let is_js = lang == "javascript" || lang == "js";

    for line in content.lines() {
        let trimmed = line.trim();

        // Rust functions
        if is_rust && (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ")) {
            let name = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("fn ")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.len() >= 3 && name != "main" && name != "new" {
                terms.insert(name);
            }
        }

        // Python functions
        if is_python && trimmed.starts_with("def ") {
            let name = trimmed
                .strip_prefix("def ")
                .unwrap_or("")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.len() >= 3 && !name.starts_with('_') {
                terms.insert(name);
            }
        }
        // Python class names
        if is_python && trimmed.starts_with("class ") {
            let name = trimmed
                .strip_prefix("class ")
                .unwrap_or("")
                .split(['(', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.len() >= 3 {
                terms.insert(name);
            }
        }

        // JavaScript functions
        if is_js {
            // Extract from .prototype.method → just "method"
            if let Some(dot) = trimmed.find(".prototype.") {
                let after = &trimmed[dot + 11..]; // skip ".prototype."
                let name = after.split(['(', '=']).next().unwrap_or("").trim().to_string();
                if name.len() >= 3 {
                    terms.insert(name);
                }
            }
            // function foo()
            if let Some(rest) = trimmed.strip_prefix("function ") {
                let name = rest.split('(').next().unwrap_or("").trim().to_string();
                if name.len() >= 3 && name != "function" {
                    terms.insert(name);
                }
            }
            // const/let/var NAME = ...
            for prefix in &["const ", "let ", "var "] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    let parts: Vec<&str> = rest.split(['=', '(', ' ', ':']).collect();
                    if let Some(name) = parts.first() {
                        let clean = name.trim().to_string();
                        if clean.len() >= 3 && clean.len() <= 25 {
                            terms.insert(clean);
                        }
                    }
                }
            }
        }

        // Generic: class names for any language
        if trimmed.starts_with("class ") {
            let name = trimmed
                .strip_prefix("class ")
                .unwrap_or("")
                .split(['(', '{', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.len() >= 3 {
                terms.insert(name);
            }
        }
    }

    // Filter common unhelpful names
    let skip = ["main", "test", "run", "init", "new", "self", "this", "super"];
    terms.retain(|t| !skip.contains(&t.as_str()) && t.len() >= 3 && t.len() <= 40);

    terms.into_iter().take(10).collect()
}

/// Search GitHub code API and return raw file contents
fn search_github_code(
    client: &reqwest::blocking::Client,
    search_url: &str,
    limit: usize,
    token: Option<&str>,
) -> Result<Vec<(String, String, String)>, String> {
    // (url, repo_name, content)
    let mut req = client
        .get(search_url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "codeplag-analyzer");

    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().map_err(|e| format!("HTTP error: {}", e))?;

    if resp.status().as_u16() == 403 {
        return Err(
            "GitHub API rate limited — set GITHUB_TOKEN env var or use --github-token".into()
        );
    }
    if resp.status().as_u16() == 422 {
        return Err("GitHub API: search query too complex. Try fewer/simpler terms.".into());
    }

    let body: serde_json::Value = resp.json().map_err(|e| format!("JSON error: {}", e))?;

    let items = body["items"].as_array().ok_or("No search results")?;

    let mut results = Vec::new();
    for item in items.iter().take(limit) {
        let html_url = item["html_url"].as_str().unwrap_or("");
        let raw_url = html_url
            .replace("https://github.com", "https://raw.githubusercontent.com")
            .replace("/blob/", "/");

        // Extract repo name from URL: https://github.com/OWNER/REPO/blob/...
        let repo_name = item["repository"]["full_name"].as_str().unwrap_or("unknown").to_string();

        // Fetch raw content
        match client.get(&raw_url).send() {
            Ok(r) => {
                if let Ok(content) = r.text() {
                    results.push((html_url.to_string(), repo_name, content));
                }
            }
            Err(_) => continue,
        }
    }

    Ok(results)
}

fn truncate_name(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Truncate a string to max_width chars, adding "…" if cut.
/// Uses char boundaries to avoid panics with multi-byte UTF-8.
fn truncate(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        format!("{: <max_width$}", s, max_width = max_width)
    } else {
        // Find byte offset of the (max_width - 1)-th char
        let byte_end = s.char_indices().nth(max_width - 1).map(|(idx, _)| idx).unwrap_or(s.len());
        format!("{}…", &s[..byte_end])
    }
}
