use anyhow::{bail, Context, Result};
use std::io::{self, IsTerminal, Read};
use std::path::Path;

/// Resolves the prompt from (in priority order):
///   1. --prompt flag
///   2. --file flag
///   3. stdin (only if not a TTY, i.e. piped input)
pub fn resolve(prompt_flag: Option<String>, file_flag: Option<&Path>) -> Result<String> {
    // 1. Explicit flag takes priority
    if let Some(p) = prompt_flag {
        let trimmed = p.trim().to_string();
        if trimmed.is_empty() {
            bail!("--prompt was provided but is empty");
        }
        return Ok(trimmed);
    }

    // 2. File path
    if let Some(path) = file_flag {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read prompt file: {}", path.display()))?;
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            bail!("prompt file is empty: {}", path.display());
        }
        return Ok(trimmed);
    }

    // 3. Stdin (only when piped)
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        let mut buf = String::new();
        stdin
            .lock()
            .read_to_string(&mut buf)
            .context("failed to read prompt from stdin")?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            bail!("stdin was empty");
        }
        return Ok(trimmed);
    }

    bail!("no prompt provided — use --prompt, --file, or pipe text to stdin")
}
