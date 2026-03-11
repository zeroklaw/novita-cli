use clap::Parser;
use std::path::PathBuf;

/// Generate images with the Novita Hunyuan Image 3 API.
#[derive(Parser, Debug)]
#[command(name = "novita", version, about)]
pub struct Args {
    /// Prompt text. If omitted, reads from --file or stdin.
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Path to a plaintext file containing the prompt.
    #[arg(short, long, value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Image width in pixels (API default: 1024).
    #[arg(long, default_value_t = 1024)]
    pub width: u32,

    /// Image height in pixels (API default: 1024).
    #[arg(long, default_value_t = 1024)]
    pub height: u32,

    /// Random seed. Use -1 for a random seed (API default: -1).
    #[arg(long, default_value_t = -1)]
    pub seed: i64,

    /// Output file path. Defaults to novita_<timestamp>.png
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Novita API key. Falls back to NOVITA_API_KEY env var.
    #[arg(long, env = "NOVITA_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// How often to poll for task completion, in milliseconds.
    #[arg(long, default_value_t = 2000, hide = true)]
    pub poll_interval_ms: u64,
}
