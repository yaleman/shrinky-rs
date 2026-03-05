# shrinky-rs

shrinky-rs is a Rust reimplementation of the Python "shrinky" tool. It is a CLI that loads a single image, optionally resizes it, and then converts it to the smallest output among supported formats (or a user-selected format).

## Features

- Auto-selects the smallest output by encoding all supported formats in parallel.
- Converts between JPG, PNG, WebP, HEIC, HEIF, and AVIF.
- Optional resize with geometry strings like `800x`, `x800`, or `800x600`.
- Optional prompt to delete the original after a successful conversion.

## Requirements

- Rust toolchain (edition 2024).
- System libraries for HEIF/HEIC support: `libheif` and `dav1d`.
  - macOS (Homebrew): `brew install libheif dav1d`
  - Linux: install `libheif` and `dav1d` via your package manager.

## Build and Test

- Full check (lint + tests + fmt): `just check`
- Build: `cargo build --workspace`
- Tests: `cargo test --quiet --workspace` or `just test`
- Lint: `cargo clippy --all-targets --quiet --workspace` or `just clippy`

## Usage

```
shrinky-rs [OPTIONS] <FILENAME>
```

Options:

- `--debug` (env `SHRINKY_DEBUG`): enable debug logging.
- `-t, --type <FORMAT>` (env `SHRINKY_TYPE`): output format (`jpg`, `png`, `webp`, `avif`, `heic`, `heif`).
- `-d, --delete` (env `SHRINKY_DELETE`): prompt to delete the source file after conversion if beneficial.
- `-g, --geometry <GEOMETRY>` (env `SHRINKY_GEOMETRY`): resize geometry (`800x600`, `800x`, `x600`).
- `-f, --force` (env `SHRINKY_FORCE`): overwrite existing output files.
- `-i, --info`: print image info (dimensions and bytes) before processing.
- `-c, --compare`: compute and print SSIM and PSNR for the selected output.
- `--output-suffix <SUFFIX>`: append SUFFIX to the output basename before extension (for example `example.jpg` -> `example-small.jpg` when using `--output-suffix -small`).
- `--min-ssim <SSIM>`: require a minimum SSIM score when set.
- `--min-psnr <PSNR>`: require a minimum PSNR score when set.

Examples:

- Auto-optimize an image:
  - `cargo run -- path/to/image.jpg`
- Convert to WebP:
  - `cargo run -- --type webp path/to/image.png`
- Resize to width 800, preserve aspect ratio:
  - `cargo run -- --geometry 800x path/to/image.heic`
- Resize to exact dimensions:
  - `cargo run -- --geometry 800x600 path/to/image.webp`
- Overwrite output if it already exists:
  - `cargo run -- --force path/to/image.jpg`
- Prompt to delete the original after conversion:
  - `cargo run -- --delete path/to/image.png`
- Add a suffix to generated output filenames:
  - `cargo run -- --output-suffix -small path/to/image.jpg`
- Compare perceptual quality for selected output (SSIM + PSNR):
  - `cargo run -- --compare path/to/image.jpg`
- Enforce a quality floor with auto-selection:
  - `cargo run -- --compare --min-ssim 0.96 --min-psnr 30 path/to/image.png`

Example output with `--compare`:

```text
Perceptual comparison:
  SSIM: 0.992314
  PSNR: 41.22 dB
```

Example failure output when a threshold is not met:

```text
Perceptual quality gate failed: PSNR 28.41 dB is below minimum 30
```

## Notes

- The output filename is always the input filename with the extension replaced by the selected format. There is no output directory option yet.
- The output filename can include an optional suffix with `--output-suffix`, appended before the extension.
- When `--type` is not specified, the tool encodes all formats in parallel and keeps the smallest result.
- `--info` prints dimensions and file size but does not currently stop further processing.
- `--compare` prints perceptual scores for the selected output in all modes.
- `--min-ssim` and `--min-psnr` are optional quality gates; when provided, exits non-zero if the comparison score falls below the threshold.
- AVIF is treated as a non-native format and is routed through the same libheif HEVC encoder used for HEIC/HEIF. This means AVIF output is not AV1-encoded at the moment.

## Development Notes

- The CLI is defined in `src/cli.rs` (clap derive).
- Image processing lives in `src/imagedata.rs`.
- Entry point and workflow are in `src/main.rs`.
