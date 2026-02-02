# AGENTS.md

Guidance for automated contributors working on shrinky-rs.

## Project Summary

shrinky-rs is a Rust CLI that loads a single image, optionally resizes it, and writes an optimized output. If no explicit output format is provided, it encodes all supported formats in parallel and keeps the smallest result.

## CLI Behavior (src/main.rs, src/cli.rs)

- Required positional argument: input filename.
- `--type/-t` selects the output format; otherwise `auto_format()` tries all formats in parallel and keeps the smallest.
- Output file path is the input path with the extension replaced by the output format.
- `--force/-f` allows overwriting an existing output file.
- `--delete/-d` prompts to delete the original only if output did not overwrite input and there is a benefit (smaller size or format change).
- `--info/-i` prints dimensions and file size but does not stop further processing.
- Logging is configured via `stderrlog` and `--debug`/`SHRINKY_DEBUG`.

## Image Pipeline (src/imagedata.rs)

- Input loading uses the `image` crate; HEIC/HEIF inputs register libheif decoding hooks before loading.
- Geometry parsing accepts `WIDTHxHEIGHT`, `WIDTHx`, and `xHEIGHT`.
- Resizing uses `resize_exact` with `Lanczos3`. Width-only or height-only preserves aspect ratio.
- HEIC/HEIF output is encoded through libheif with HEVC (`CompressionFormat::Hevc`) at quality 85.
- AVIF is treated as non-native and currently uses the same libheif HEVC output path as HEIC/HEIF (not AV1-encoded).

## Key Types (src/lib.rs)

- `ImageFormat` enum: `Jpg`, `Png`, `Webp`, `Avif`, `Heic`, `Heif`.
- `ImageFormat::is_native_image_format()` is true only for JPG/PNG/WebP.
- `ImageFormat::try_from_filename()` and `FromStr` power format selection by extension/CLI.
- `Error` enum centralizes error handling; avoid panics in non-test code.

## Dependencies

- System libs: `libheif` and `dav1d` (for HEIF/HEIC handling).
- Rust crates: `image`, `libheif-rs`, `rayon`, `clap`, `stderrlog`, `log`, `strum`.

## Build and Development Commands

- Full check: `just check`
- Build: `cargo build --workspace`
- Tests: `cargo test --quiet --workspace` or `just test`
- Lint: `cargo clippy --all-targets --quiet --workspace` or `just clippy`
- Format: `cargo fmt --all` or `just fmt`
- Coverage: `just coverage` (generates `tarpaulin-report.html`)

No task is complete unless `just check` passes without errors or warnings.

## Linting Rules

Strict clippy lints are enabled in `src/lib.rs`:

- Denied: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and more.
- `clippy.toml` allows `expect` and `dbg!` in tests, but disallows `unwrap` and `panic` in tests.
- New code should avoid `unwrap`/`expect`/`panic` outside tests and avoid `unwrap`/`panic` in tests.

## Tests

- `tests/test_geometry.rs`: geometry parsing and error cases.
- `tests/test_imageformat.rs`: format parsing and conversion behavior.
- `tests/test_image.rs`: load/resize/encode flows using fixtures in `tests/test_images/`.
