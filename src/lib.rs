#![warn(missing_docs)]
//! # Codeplag-rs
//!
//! A multi-dimensional code plagiarism analyzer supporting 7 programming languages
//! with 9 distinct fingerprint dimensions and weighted ensemble scoring.

/// CLI argument definitions and command dispatch.
pub mod cli;
/// Core data types used throughout the analysis pipeline.
pub mod core;
/// Similarity engine, fingerprint caching, and chunk matching logic.
pub mod engine;
/// GitHub repository fetching and local file collection.
pub mod fetcher;
/// Code fingerprint generation (Winnowing + AST structural analysis).
pub mod fingerprint;
