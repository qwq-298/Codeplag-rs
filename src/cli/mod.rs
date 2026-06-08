use clap::{Parser, Subcommand};

/// Codeplag - A GitHub code plagiarism analyzer
#[derive(Parser, Debug)]
#[command(name = "codeplag")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Minimum similarity threshold [0.0, 1.0]
    #[arg(short, long, default_value = "0.5")]
    pub threshold: f64,

    /// k-gram size for winnowing
    #[arg(long, default_value = "5")]
    pub k_gram: usize,

    /// Window size for winnowing
    #[arg(long, default_value = "4")]
    pub window: usize,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// GitHub personal access token (for API search)
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,

    /// Compare at function level (extract and compare individual functions)
    #[arg(long)]
    pub functions: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a local directory for code similarity
    Analyze {
        /// Path to the directory to analyze
        #[arg(short, long)]
        path: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Compare a single file against another file or directory
    Compare {
        /// Path to the target file
        #[arg(short, long)]
        file: String,

        /// Path to a file or directory to compare against
        #[arg(short, long)]
        against: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Fetch and analyze a GitHub repository
    Fetch {
        /// GitHub repository URL
        #[arg(short, long)]
        repo: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Search GitHub for code similar to a local project
    Search {
        /// Path to a source file or project directory
        #[arg(short, long)]
        path: Option<String>,

        /// GitHub repo URL to fetch and search against
        #[arg(short, long)]
        repo: Option<String>,

        /// Maximum number of search results per term (default: 5)
        #[arg(short, long, default_value = "5")]
        limit: usize,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Compare two projects (directories) at the project level
    Project {
        /// Path to project A
        #[arg(short = 'a', long)]
        project_a: String,

        /// Path to project B
        #[arg(short = 'b', long)]
        project_b: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Batch fetch multiple GitHub repos and compare all pairs
    Batch {
        /// Comma-separated GitHub repo URLs (e.g., "user/a,user/b")
        #[arg(short, long, value_delimiter = ',')]
        repos: Vec<String>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
}
