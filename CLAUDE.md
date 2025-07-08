# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

easy-hyoka is a Rust CLI tool that generates performance evaluation summaries from GitHub activity using AI. It fetches Pull Requests and Issues from GitHub using the `gh` CLI and processes them with OpenAI to create comprehensive evaluation reports for engineers.

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run in development mode
cargo run -- --owner=<org-name> --author=<username> --since=2025-01-01 --until=2025-06-30

# Build release version
cargo build --release

# Install locally
cargo install --path .
```

### Code Quality
```bash
# Format code
cargo fmt

# Run linter with all warnings
cargo clippy -- -D warnings

# Run tests
cargo test

# Type check
cargo check
```

### Testing with Different Parameters
```bash
# Test with specific date range
cargo run -- --owner=heyinc --since=2025-01-01 --until=2025-03-31

# Show the prompts being sent to OpenAI
cargo run -- --owner=heyinc --show-prompts

# Use current GitHub user (auto-detected)
cargo run -- --owner=heyinc
```

## Project Architecture

### Core Components

1. **CLI Interface** (`src/main.rs:7-25`)
   - Uses `clap` for command-line argument parsing
   - Supports owner, author, date range, and debug options

2. **GitHub Data Fetching** (`src/main.rs:154-286`)
   - `fetch_prs()`: Retrieves Pull Requests using `gh search prs`
   - `fetch_issues()`: Retrieves Issues using `gh search issues`  
   - `fetch_pr_comments()` / `fetch_issue_comments()`: Gets comments for recent items
   - Handles GitHub API's 1000-item limit with warnings

3. **OpenAI Integration** (`src/main.rs:288-451`)
   - `generate_summary()`: Formats GitHub data into prompts and sends to OpenAI
   - Uses structured JSONL format for PR/Issue data
   - Generates comprehensive evaluation summaries in Japanese

### Data Flow
1. Parse CLI arguments and auto-detect GitHub user if needed
2. Fetch PRs and Issues from GitHub using `gh` CLI
3. Retrieve comments for the 5 most recent items
4. Format all data as JSONL and create evaluation prompt
5. Send to OpenAI API (gpt-4.1-mini-2025-04-14)
6. Display formatted evaluation summary

## Dependencies and Environment

### Required Tools
- `gh` CLI must be installed and authenticated (`gh auth login`)
- Rust toolchain (edition 2024)

### Environment Variables
- `OPENAI_API_KEY`: Required for OpenAI API access
- Can use `.env` file for local development

### Key Dependencies
- `tokio`: Async runtime for HTTP requests
- `reqwest`: HTTP client for OpenAI API
- `serde`/`serde_json`: JSON serialization
- `clap`: CLI argument parsing
- `chrono`: Date/time handling
- `anyhow`: Error handling

## Important Notes

- The tool fetches up to 1000 PRs and 1000 Issues per query (GitHub API limit)
- If results reach 1000, it warns to use more specific date ranges
- Comments are only fetched for the 5 most recent PRs/Issues to avoid rate limits
- All evaluation summaries are generated in Japanese
- The tool uses the latest GPT-4 mini model for cost-effective processing

## Common Issues and Solutions

1. **GitHub Authentication**
   - Ensure `gh auth login` has been run
   - Check `gh auth status` if authentication fails

2. **Rate Limiting**
   - Use more specific date ranges to reduce API calls
   - Wait a few minutes if rate limited

3. **OpenAI API Errors**
   - Verify OPENAI_API_KEY is set correctly
   - Check API quota and billing status