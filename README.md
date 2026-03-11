# novita

CLI for generating images with the [Novita Hunyuan Image 3](https://novita.ai/docs/api-reference/model-apis-hunyuan-image-3) API.

## Build

```bash
cargo build --release
# binary at: target/release/novita
```

## Usage

```bash
# API key via env var (recommended)
export NOVITA_API_KEY=your_key_here

# Prompt via flag
novita --prompt "a calico cat in a neon-lit alley"

# Prompt via file
novita --file prompt.txt

# Prompt via stdin
echo "a stormy sea at midnight" | novita

# Custom size and seed
novita --prompt "a forest at dawn" --width 1280 --height 720 --seed 42

# Specify output file
novita --prompt "a red panda" --output red_panda.png
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--prompt` / `-p` | — | Prompt text |
| `--file` / `-f` | — | Path to plaintext prompt file |
| `--width` | 1024 | Image width in pixels |
| `--height` | 1024 | Image height in pixels |
| `--seed` | -1 | Seed (-1 = random) |
| `--output` / `-o` | `novita_<timestamp>.png` | Output file path |
| `--api-key` | `$NOVITA_API_KEY` | Novita API key |

## Prompt resolution

Priority order:
1. `--prompt` flag
2. `--file` flag
3. stdin (if piped)

## How it works

1. POST to `/v3/async/hunyuan-image-3` → get `task_id`
2. Poll `/v3/async/task-result?task_id=...` until `TASK_STATUS_SUCCEED`
3. Download image from returned URL
4. Save to output file

The output file path is also printed to stdout (stderr gets the status messages), so you can pipe it:

```bash
open $(novita --prompt "a crab in a top hat" 2>/dev/null)
```
