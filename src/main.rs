mod api;
mod cli;
mod prompt;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::parse();

    // Resolve prompt
    let prompt_text = prompt::resolve(args.prompt, args.file.as_deref())?;

    // Resolve output path
    let output_path = args.output.unwrap_or_else(default_output_path);

    let client = reqwest::Client::new();

    // ── Submit ────────────────────────────────────────────────────────────────

    eprintln!("Submitting generation request...");
    let task_id = api::submit(
        &client,
        &args.api_key,
        &api::GenerateRequest {
            prompt: prompt_text,
            width: args.width,
            height: args.height,
            seed: args.seed,
        },
    )
    .await?;

    eprintln!("Task ID: {task_id}");

    // ── Poll ──────────────────────────────────────────────────────────────────

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Waiting for generation...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let image_urls = loop {
        sleep(Duration::from_millis(args.poll_interval_ms)).await;

        match api::poll(&client, &args.api_key, &task_id).await? {
            api::PollResult::Pending => {
                // keep spinning
            }
            api::PollResult::Done(urls) => {
                spinner.finish_with_message("Done!");
                break urls;
            }
            api::PollResult::Failed(reason) => {
                spinner.finish_with_message("Failed.");
                anyhow::bail!("generation failed: {reason}");
            }
        }
    };

    // ── Download ──────────────────────────────────────────────────────────────

    // For now: download the first image. Future: handle multiple.
    let url = &image_urls[0];
    eprint!("Downloading image... ");
    let image_bytes = api::download(&client, url)
        .await
        .context("image download failed")?;

    std::fs::write(&output_path, &image_bytes)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    eprintln!("Saved to {}", output_path.display());
    println!("{}", output_path.display());

    Ok(())
}

fn default_output_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    PathBuf::from(format!("novita_hunyuan3_{ts}.png"))
}
