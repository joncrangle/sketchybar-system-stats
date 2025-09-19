# Agent Development Guide

## Build/Test Commands
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo test` - Run all tests
- `cargo fmt` - Format code
- `cargo clippy` - Run linting checks

## Code Style
- Use `snake_case` for functions, variables, and modules
- Use `PascalCase` for structs and enums
- Import external crates at top, then `use` statements grouped by visibility
- Error handling: Use `anyhow::Result<T>` for public APIs, `?` operator for propagation
- String formatting: Use `format!()` macro for dynamic strings, string literals for static

## Architecture
- Modular structure: `src/` contains `main.rs`, `cli.rs`, `sketchybar.rs`, and `stats/` module
- Stats module exports individual stat functions via `mod.rs`
- CLI uses `clap` with `Parser` derive macro
- Async main with `tokio` runtime for event loop

## Dependencies
- Core: `anyhow`, `clap`, `sysinfo`, `tokio`
- Build: `cc` for C bindings
- Platform: macOS only (`#[cfg(target_os = "macos")]`)
