use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.novita.ai/v3/async";

// ── Submit ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct GenerateRequest {
    pub prompt: String,
    pub width: u32,
    pub height: u32,
    pub seed: i64,
}

#[derive(Deserialize, Debug)]
pub struct GenerateResponse {
    pub task_id: String,
}

pub async fn submit(client: &Client, api_key: &str, req: &GenerateRequest) -> Result<String> {
    let resp = client
        .post(format!("{BASE_URL}/hunyuan-image-3"))
        .bearer_auth(api_key)
        .json(req)
        .send()
        .await
        .context("failed to send generation request")?;

    let status = resp.status();
    let body = resp.text().await.context("failed to read response body")?;

    if !status.is_success() {
        bail!("API error {status}: {body}");
    }

    let parsed: GenerateResponse =
        serde_json::from_str(&body).context("failed to parse generation response")?;

    Ok(parsed.task_id)
}

// ── Poll ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct TaskResult {
    pub task: TaskStatus,
    #[serde(default)]
    pub images: Vec<TaskImage>,
}

#[derive(Deserialize, Debug)]
pub struct TaskStatus {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TaskImage {
    pub image_url: String,
}

#[derive(Debug)]
pub enum PollResult {
    Pending,
    Done(Vec<String>), // image URLs
    Failed(String),
}

pub async fn poll(client: &Client, api_key: &str, task_id: &str) -> Result<PollResult> {
    let resp = client
        .get(format!("{BASE_URL}/task-result"))
        .query(&[("task_id", task_id)])
        .bearer_auth(api_key)
        .send()
        .await
        .context("failed to poll task result")?;

    let status = resp.status();
    let body = resp.text().await.context("failed to read poll response")?;

    if !status.is_success() {
        bail!("API error {status} while polling: {body}");
    }

    let result: TaskResult =
        serde_json::from_str(&body).context("failed to parse task result")?;

    match result.task.status.as_str() {
        "TASK_STATUS_QUEUED" | "TASK_STATUS_PROCESSING" => Ok(PollResult::Pending),
        "TASK_STATUS_SUCCEED" => {
            let urls: Vec<String> = result.images.into_iter().map(|i| i.image_url).collect();
            if urls.is_empty() {
                bail!("task succeeded but returned no images");
            }
            Ok(PollResult::Done(urls))
        }
        "TASK_STATUS_FAILED" => {
            let reason = result
                .task
                .reason
                .unwrap_or_else(|| "unknown reason".to_string());
            Ok(PollResult::Failed(reason))
        }
        other => bail!("unexpected task status: {other}"),
    }
}

// ── Download ──────────────────────────────────────────────────────────────────

pub async fn download(client: &Client, url: &str) -> Result<Vec<u8>> {
    let resp = client
        .get(url)
        .send()
        .await
        .context("failed to download image")?;

    if !resp.status().is_success() {
        bail!("image download failed with status {}", resp.status());
    }

    let bytes = resp.bytes().await.context("failed to read image bytes")?;
    Ok(bytes.to_vec())
}
