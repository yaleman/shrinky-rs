use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;

use clap::Parser;

use log::{debug, error, info, warn};
use shrinky_rs::{
    ImageFormat,
    cli::Cli,
    imagedata::{Geometry, Image},
};

/// Format a byte count as a string with comma separators
fn format_bytes(bytes: u64) -> String {
    let s = bytes.to_string();
    let mut result = String::new();

    for (count, c) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }

    result
}

/// Prompt user to delete source file, showing comparison information
fn prompt_delete_source(
    input_path: &Path,
    original_size: u64,
    original_format: ImageFormat,
    output_path: &Path,
    output_size: usize,
    output_format: ImageFormat,
) -> Result<bool, io::Error> {
    println!();
    println!(
        "Original: {} ({}, {} bytes)",
        input_path.display(),
        original_format.extension().to_uppercase(),
        format_bytes(original_size)
    );
    println!(
        "New:      {} ({}, {} bytes)",
        output_path.display(),
        output_format.extension().to_uppercase(),
        format_bytes(output_size as u64)
    );

    if output_size < original_size as usize {
        let savings = original_size - output_size as u64;
        let percent = (savings as f64 / original_size as f64) * 100.0;
        println!(
            "Savings:  {} bytes ({:.0}% smaller)",
            format_bytes(savings),
            percent
        );
    } else if output_size > original_size as usize {
        let increase = output_size as u64 - original_size;
        let percent = (increase as f64 / original_size as f64) * 100.0;
        println!(
            "Increase: {} bytes ({:.0}% larger)",
            format_bytes(increase),
            percent
        );
    }

    println!();
    print!("Delete original file? [y/N]: ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(matches!(response.as_str(), "y" | "yes"))
}

fn main() {
    let cli = Cli::parse();
    let log_level = if cli.debug {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    if let Err(err) = stderrlog::new()
        .verbosity(log_level)
        .show_module_names(cli.debug)
        .init()
    {
        eprintln!("Failed to initialize logger: {}", err);
        std::process::exit(1);
    }

    if !cli.filename.exists() {
        error!("File not found: {}", cli.filename.display());
        std::process::exit(1);
    }
    if !cli.filename.is_file() {
        error!("Not a file: {}", cli.filename.display());
        std::process::exit(1);
    }

    info!("Processing image: {}", cli.filename.display());
    let mut image = match Image::try_from(&cli.filename) {
        Ok(img) => img,
        Err(e) => {
            error!("Error loading image: {:?}", e);
            std::process::exit(1);
        }
    };

    if let Some(target_geometry) = cli.geometry {
        let target_geometry = match Geometry::from_str(target_geometry.as_str()) {
            Ok(geom) => geom,
            Err(e) => {
                error!("Error parsing geometry: {:?}", e);
                std::process::exit(1);
            }
        };
        if !target_geometry.is_empty() {
            image = image.with_target_geometry(target_geometry);

            match image.resize() {
                Ok(new_image) => {
                    debug!(
                        "Resized image to {}x{}",
                        new_image.width(),
                        new_image.height()
                    );
                }
                Err(e) => {
                    error!("Error resizing image: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    let bytes_to_write = match cli.output_type {
        None => match image.auto_format() {
            Ok((format, data)) => {
                info!(
                    "Auto-optimized image to format {:?}, size {} bytes",
                    format,
                    data.len()
                );
                image.output_format = Some(format);
                data
            }
            Err(e) => {
                error!("Error auto-optimizing image: {:?}", e);
                std::process::exit(1);
            }
        },
        Some(format) => match image.output_as_format(format) {
            Ok(data) => {
                info!(
                    "Encoded image to format {:?}, size {} bytes",
                    format,
                    data.len()
                );
                image.output_format = Some(format);
                data
            }
            Err(e) => {
                error!("Error encoding image as {:?}: {:?}", format, e);
                std::process::exit(1);
            }
        },
    };

    if bytes_to_write.is_empty() {
        error!("No image data to write. This is probably a bug!");
        std::process::exit(1);
    }

    if image.will_overwrite() && !cli.force {
        error!(
            "Output file {} already exists. Use --force to overwrite.",
            image.output_filename().display()
        );
        std::process::exit(1);
    }

    match std::fs::write(image.output_filename(), &bytes_to_write) {
        Ok(_) => {
            info!(
                "Wrote optimized image to {} ({} bytes)",
                image.output_filename().display(),
                bytes_to_write.len()
            );
        }
        Err(e) => {
            error!(
                "Error writing optimized image to {}: {}",
                image.output_filename().display(),
                e
            );
            std::process::exit(1);
        }
    }

    // Handle --delete flag: prompt user to delete source file if beneficial
    if cli.delete {
        // Don't delete if output overwrote input (file already replaced)
        if !image.will_overwrite() {
            // Get original format to compare
            match ImageFormat::try_from(&image.input_filename) {
                Ok(original_format) => {
                    // Output format should always be set at this point
                    if let Some(output_format) = &image.output_format {
                        let format_changed = &original_format != output_format;
                        let size_reduced = bytes_to_write.len() < image.original_file_size as usize;

                        debug!(
                            "Delete check: format_changed={}, size_reduced={}",
                            format_changed, size_reduced
                        );

                        // Only prompt if there's a benefit (smaller or different format)
                        if format_changed || size_reduced {
                            match prompt_delete_source(
                                &image.input_filename,
                                image.original_file_size,
                                original_format,
                                &image.output_filename(),
                                bytes_to_write.len(),
                                *output_format,
                            ) {
                                Ok(should_delete) => {
                                    if should_delete {
                                        match std::fs::remove_file(&image.input_filename) {
                                            Ok(_) => {
                                                info!(
                                                    "Deleted original file: {}",
                                                    image.input_filename.display()
                                                );
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to delete original file {}: {}",
                                                    image.input_filename.display(),
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        info!(
                                            "Keeping original file: {}",
                                            image.input_filename.display()
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!("Error prompting for deletion: {}", e);
                                }
                            }
                        } else {
                            debug!(
                                "No benefit to deleting original file (same format and not smaller)"
                            );
                        }
                    } else {
                        warn!("Output format not set after conversion");
                    }
                }
                Err(e) => {
                    warn!(
                        "Could not determine original format for {}: {:?}",
                        image.input_filename.display(),
                        e
                    );
                }
            }
        } else {
            debug!("Skipping deletion: output overwrote input file");
        }
    }
}
