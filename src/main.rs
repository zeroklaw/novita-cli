mod api;
mod cli;
mod prompt;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::parse();

    if args.count == 0 {
        anyhow::bail!("--count must be at least 1");
    }

    // Resolve API key from environment
    let api_key = std::env::var("NOVITA_API_KEY")
        .map_err(|_| anyhow::anyhow!("NOVITA_API_KEY environment variable is not set"))?;

    // Resolve prompt
    let prompt_text = prompt::resolve(args.prompt, args.file.as_deref())?;

    let client = reqwest::Client::new();

    let req = api::GenerateRequest {
        prompt: prompt_text,
        width: args.width,
        height: args.height,
        seed: args.seed,
    };

    // ── Submit all requests ───────────────────────────────────────────────────

    eprintln!("Submitting {} generation request(s)...", args.count);

    let mut submit_handles = tokio::task::JoinSet::new();
    for i in 0..args.count {
        let client = client.clone();
        let api_key = api_key.clone();
        let req = req.clone();
        submit_handles.spawn(async move {
            let task_id = api::submit(&client, &api_key, &req).await?;
            Ok::<(usize, String), anyhow::Error>((i, task_id))
        });
    }

    let mut task_ids: Vec<(usize, String)> = Vec::with_capacity(args.count);
    while let Some(res) = submit_handles.join_next().await {
        let (i, task_id) = res.context("submit task panicked")??;
        eprintln!("  [{i}] task_id: {task_id}");
        task_ids.push((i, task_id));
    }
    // Sort by index so output is deterministic
    task_ids.sort_by_key(|(i, _)| *i);

    // ── Poll all tasks concurrently ───────────────────────────────────────────

    let mp = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix} {spinner:.green} {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);

    let mut poll_handles = tokio::task::JoinSet::new();
    for (i, task_id) in task_ids {
        let client = client.clone();
        let api_key = api_key.clone();
        let poll_interval_ms = args.poll_interval_ms;

        let spinner = mp.add(ProgressBar::new_spinner());
        spinner.set_style(spinner_style.clone());
        spinner.set_prefix(format!("[{i}]"));
        spinner.set_message("waiting...");
        spinner.enable_steady_tick(Duration::from_millis(80));

        poll_handles.spawn(async move {
            let urls = loop {
                sleep(Duration::from_millis(poll_interval_ms)).await;
                match api::poll(&client, &api_key, &task_id).await? {
                    api::PollResult::Pending => {}
                    api::PollResult::Done(urls) => {
                        spinner.finish_with_message("done");
                        break urls;
                    }
                    api::PollResult::Failed(reason) => {
                        spinner.finish_with_message(format!("failed: {reason}"));
                        anyhow::bail!("task [{i}] failed: {reason}");
                    }
                }
            };
            Ok::<(usize, Vec<String>), anyhow::Error>((i, urls))
        });
    }

    let mut results: Vec<(usize, Vec<String>)> = Vec::with_capacity(args.count);
    while let Some(res) = poll_handles.join_next().await {
        results.push(res.context("poll task panicked")??);
    }
    results.sort_by_key(|(i, _)| *i);

    // ── Download all concurrently ─────────────────────────────────────────────

    let base_path = args.output.unwrap_or_else(default_output_path);
    let count = args.count;

    let mut dl_handles = tokio::task::JoinSet::new();
    for (i, urls) in results {
        let client = client.clone();
        let url = urls.into_iter().next().unwrap();
        let out_path = indexed_path(&base_path, i, count);

        dl_handles.spawn(async move {
            let bytes = api::download(&client, &url)
                .await
                .context("image download failed")?;
            std::fs::write(&out_path, &bytes)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
            Ok::<PathBuf, anyhow::Error>(out_path)
        });
    }

    while let Some(res) = dl_handles.join_next().await {
        let path = res.context("download task panicked")??;
        eprintln!("Saved: {}", path.display());
        println!("{}", path.display());
    }

    Ok(())
}

/// Returns the output path for image `i` of `total`.
/// If total == 1, returns `base` unchanged.
/// If total > 1, inserts `_i` before the extension: `novita_ts_0.png`
fn indexed_path(base: &PathBuf, i: usize, total: usize) -> PathBuf {
    if total == 1 {
        return base.clone();
    }
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("novita");
    let ext = base
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let parent = base.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent.join(format!("{stem}_{i}.{ext}"))
}

fn default_output_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    PathBuf::from(format!("novita_hunyuan3_{ts}.png"))
}
