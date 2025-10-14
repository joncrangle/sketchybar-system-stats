# Agent Development Guide

## Build/Test Commands
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run a single test (e.g., `cargo test test_validate_cli_with_all_flag`)
- `cargo fmt` - Format code
- `cargo clippy` - Run linting checks

## Code Style
- Naming: `snake_case` for functions/variables/modules, `PascalCase` for structs/enums, `SCREAMING_SNAKE_CASE` for constants
- Imports: External crates first, then local `mod` and `use` statements, group by visibility (pub before private)
- Error handling: `anyhow::Result<T>` for fallible functions, `anyhow::bail!()` for errors, `?` for propagation, `.context()` for error context
- String formatting: `format!()` for dynamic, string literals for static, `String::with_capacity()` for buffers
- Function signatures: Accept `&str` slices, return owned `String` only when necessary
- Testing: Place unit tests in `#[cfg(test)] mod tests` at bottom of each file, use descriptive names like `test_<function>_<scenario>`

## Architecture
- Modular structure: `src/` contains `main.rs`, `cli.rs`, `sketchybar.rs`, and `stats/` module
- Stats module: Individual stat files (`cpu.rs`, `disk.rs`, etc.) with public functions exported via `mod.rs`
- CLI: `clap` with `Parser` derive, validation in separate `validate_cli()` function, constants for defaults/limits
- Async runtime: `tokio::main` macro, `tokio::select!` for concurrent operations, `tokio::time::sleep` for intervals
- Platform: macOS only - use `#[cfg(target_os = "macos")]` for platform-specific code
