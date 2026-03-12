# csl

A fast, single-binary statusline for [Claude Code](https://docs.anthropic.com/en/docs/claude-code).

![Rust](https://img.shields.io/badge/Rust-2024_edition-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## What it does

`csl` replaces Claude Code's default statusline with a rich, informative display:

```
claude-sonnet  ▰▰▰▰▰▱▱▱▱▱ 45%  ~/projects (main)  ⏱ 1h23m
hourly  ▱▱▱▱▱▱▱▱▱▱ 0%  resets in 42m
daily  ▰▰▱▱▱▱▱▱▱▱ 15%  resets in 8h
```

**Session line** — model name, context window usage (color-coded progress bar), working directory, git branch, and session duration.

**Rate limit lines** — Anthropic API rate limits for your account, cached and refreshed every 60 seconds.

## Features

- **Color-coded context usage** — green (<50%), yellow (50-70%), orange (70-90%), red (≥90%)
- **Git-aware** — shows branch name with dirty-tree indicator (pure Rust via `gix`, no `git` binary needed)
- **API rate limits** — fetches usage from Anthropic's API with token auto-discovery (env var, keyring, or credentials file)
- **Graceful degradation** — never crashes; falls back to cached or empty data on errors
- **Tiny binary** — release build optimized for size with LTO and stripping

## Installation

### Quick install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/rockia/csl/main/install.sh | sh
```

Downloads a pre-built binary for your platform, installs it to `~/.claude/csl`, and configures Claude Code's statusline.

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/rockia/csl/releases/latest):

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `csl-aarch64-macos.tar.gz` |
| macOS (Intel) | `csl-x86_64-macos.tar.gz` |
| Linux (x86_64) | `csl-x86_64-linux.tar.gz` |
| Linux (ARM64) | `csl-aarch64-linux.tar.gz` |

```bash
tar xzf csl-*.tar.gz
./csl install
```

### Install with Cargo

Requires [Rust](https://rustup.rs/).

```bash
cargo install --git https://github.com/rockia/csl && csl install
```

### Build from source

```bash
git clone https://github.com/rockia/csl && cd csl
cargo build --release
./target/release/csl install
```

### Uninstall

```bash
~/.claude/csl uninstall
```

Restores your previous statusline configuration and removes the binary.

## How it works

Claude Code pipes JSON to the statusline binary via stdin:

```json
{
  "model": { "id": "claude-sonnet-4-6", "display_name": "Sonnet" },
  "context_window": { "used_percentage": 48, "remaining_percentage": 52 },
  "workspace": { "current_dir": "/Users/you/projects", "project_dir": "/Users/you/projects" },
  "cost": { "total_cost_usd": 0.12, "total_duration_ms": 45000 }
}
```

`csl` reads this, formats the statusline, and writes it to stdout. Claude Code displays the result.

## Token resolution

Rate limit data requires an Anthropic API token. `csl` checks these sources in order:

1. `CLAUDE_OAUTH_TOKEN` environment variable
2. System keyring (`claude-api` service)
3. `~/.claude/.credentials.json`

## Project structure

```
src/
├── main.rs        # Entry point, CLI dispatch
├── statusline.rs  # Statusline rendering from stdin JSON
├── cli.rs         # Install/uninstall logic
├── git.rs         # Git branch and dirty-tree detection
├── format.rs      # Colors, progress bars, duration, paths
└── usage.rs       # API rate limit fetching and caching
```

## License

MIT
