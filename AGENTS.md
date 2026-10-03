# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2021 CLI for fetching, judging, and managing Luogu problems.

- `src/main.rs` routes CLI commands to handlers.
- `src/cli.rs` defines the Clap command tree and argument structs.
- `src/commands/` contains one module per command, such as `fetch.rs` and `judge.rs`.
- `src/net.rs`, `src/models.rs`, `src/config.rs`, and `src/storage.rs` contain API access, Serde models, configuration, and persistence helpers.
- `.luogu/`, `problem/`, `luogu_config.json`, and `judge_cpp.json` are local runtime data/configuration. `ref/` contains upstream Luogu API reference material, not application code.

## Build, Test, and Development Commands

- `cargo build` builds a debug binary; `cargo build --release` or `make build` builds the release binary.
- `cargo run -- fetch P1000` runs the CLI locally. Makefile shortcuts include `make run-fetch P=P1000` and `make run-judge P=P1000`.
- `cargo test` runs the test suite.
- `cargo fmt` formats Rust sources; use `cargo fmt -- --check` to verify formatting.
- `cargo clippy --all-targets` checks common Rust issues.

## Coding Style & Naming Conventions

Follow standard `rustfmt` formatting and four-space indentation. Use `snake_case` for modules, files, functions, and variables; `UpperCamelCase` for types and traits; and `SCREAMING_SNAKE_CASE` for constants. Define CLI arguments in `src/cli.rs`, expose command handlers as `pub fn run(args: ...) -> anyhow::Result<()>`, and register new commands in `src/main.rs`. Use `anyhow::Context` for errors involving files, paths, or network requests.

## Testing Guidelines

Tests currently use Rust's built-in `#[cfg(test)] mod tests` pattern, as in `src/config.rs`. Name tests by behavior, such as `default_testcase_names`, and run them with `cargo test`. No coverage threshold or CI test job is configured. Add tests for bug fixes and shared parsing/configuration logic; use fixtures instead of live Luogu API calls. Place future integration tests under `tests/`.

## Commit & Pull Request Guidelines

Git history mixes Conventional Commit-style prefixes (`feat:`, `add:`) with plain English and Chinese messages. Prefer concise imperative messages such as `feat: add training pagination` or `fix: handle invalid testcase names`. Pull requests should describe the changed commands, link relevant issues, list tests run, and include terminal or screenshot evidence for user-visible output. Do not commit cookies, `~/.cookie` contents, bulk fetched problem data, or unrelated generated files.
