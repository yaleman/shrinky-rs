# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

shrinky-rs is a Rust reimplementation of the Python "shrinky" tool - an image conversion and compression utility. It processes images, optionally resizes them based on geometry specifications, and converts them to optimal formats (JPG, PNG, WebP, AVIF, HEIC/HEIF).

## Key Dependencies

- **macOS-specific**: Requires `dav1d` and `libheif` system libraries (`brew install dav1d libheif`)
- **image crate**: For native format handling (JPG, PNG, WebP, AVIF)
- **libheif-rs**: For HEIC/HEIF format support
- **rayon**: For parallel image format optimization
- **clap**: CLI argument parsing with derive macros
- **log** and **stderrlog**: Logging infrastructure
- **strum**: Enum utilities with derive macros

## Build and Development Commands

**Primary workflow command:**

- **Full check** (runs clippy, tests, and formatting): `just check`

**Individual commands:**

- **Build**: `cargo build --workspace`
- **Run tests**: `just test` or `cargo test --quiet --workspace`
- **Run single test**: `cargo test test_name`
- **Run single test file**: `cargo test --test test_geometry`
- **Linting**: `just clippy` or `cargo clippy --all-targets --quiet --workspace`
- **Format**: `just fmt` or `cargo fmt --all`
- **Coverage**: `just coverage` (generates tarpaulin-report.html)
- **Security scan**: `just semgrep`

No task is complete unless `just check` passes without errors or warnings.

## Architecture

### Core Modules

- **lib.rs**: Defines `ImageFormat` enum with conversions and the main `Error` type
- **cli.rs**: CLI interface using clap's derive macros
- **imagedata.rs**: Contains `Image` and `Geometry` structs with all image processing logic
- **main.rs**: Entry point that wires together CLI parsing, image loading, processing, and output

### Image Format Handling

The codebase distinguishes between "native" formats (handled by the `image` crate: JPG, PNG, WebP, AVIF) and non-native formats (HEIC/HEIF via libheif-rs):

- `ImageFormat::is_native_image_format()` returns true for JPG/PNG/WebP/AVIF
- HEIC/HEIF require special handling via libheif-rs with manual plane creation
- Auto-optimization (`auto_format()`) uses rayon to try all formats in parallel and selects the smallest output

### Geometry System

Geometry parsing supports three formats:

- `WIDTHxHEIGHT` - exact dimensions (e.g., "800x600")
- `WIDTHx` - constrain width, maintain aspect ratio (e.g., "800x")
- `xHEIGHT` - constrain height, maintain aspect ratio (e.g., "x600")

### Strict Linting Rules

The codebase enforces very strict clippy lints in lib.rs:

- **Forbidden**: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`
- **Exception**: Tests allow `expect` and `dbg!` (via clippy.toml: `allow-expect-in-tests = true`, `allow-dbg-in-tests = true`)
- **Forbidden in tests**: `unwrap`, `panic` (even in tests)
- All warnings treated as denials

When adding code, always handle errors with `Result` and `?` operator. Never use unwrap/expect/panic outside of tests, and use expect sparingly in tests.

## Testing

Test files are in `tests/` directory:

- `test_geometry.rs`: Geometry parsing tests
- `test_image.rs`: Image loading and processing tests
- `test_imageformat.rs`: Image format conversion tests
- `test_images/`: Contains sample images in all supported formats (bruny-oysters.{jpg,png,webp,avif,heif,heic})

Run individual test files with: `cargo test --test test_geometry`

## Edition

- Rust edition: 2024 (note: uses `edition = "2024"` in Cargo.toml)

## Cargo Commands

Always use cargo commands for dependency management:

- Add dependency: `cargo add <crate>`
- Remove dependency: `cargo remove <crate>`
- Update dependencies: `cargo update`

Never manually edit Cargo.toml for dependency changes unless absolutely necessary.
