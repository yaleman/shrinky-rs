use clap::Parser;
use log::{debug, error, info, warn};
use shrinky_rs::{
    ImageFormat, PsnrQuality, SsimQuality,
    cli::Cli,
    imagedata::{Geometry, Image},
};
use std::{
    cmp::max,
    io::{self, Write},
    path::Path,
    process::ExitCode,
    str::FromStr,
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

pub fn setup_logging(debug: bool) {
    let log_level = if debug {
        log::Level::Debug
    } else {
        log::Level::Info
    };
    if let Err(err) = stderrlog::new()
        .verbosity(log_level)
        .show_module_names(debug)
        .init()
    {
        eprintln!("Failed to initialize logger: {}", err);
        std::process::exit(1);
    }
}

fn aggregate_exit_code(current: u8, next: u8) -> u8 {
    max(current, next)
}

fn process_image(cli: &Cli, target_geometry: Option<&Geometry>, input_path: &Path) -> u8 {
    if !input_path.exists() {
        error!("File not found: {}", input_path.display());
        return 1;
    }
    if !input_path.is_file() {
        error!("Not a file: {}", input_path.display());
        return 1;
    }

    debug!("Processing image: {}", input_path.display());
    let input_filename = input_path.to_path_buf();
    let mut image = match Image::try_from(&input_filename) {
        Ok(img) => img,
        Err(e) => {
            error!("Error loading image {}: {:?}", input_path.display(), e);
            return 1;
        }
    };
    image = image.with_output_suffix(cli.output_suffix.clone());
    if cli.info {
        info!(
            "{}: Dimensions: {}x{} Size: {} bytes",
            input_path.display(),
            image.image.width(),
            image.image.height(),
            format_bytes(image.original_file_size)
        );
    }

    if let Some(target_geometry) = target_geometry {
        image = image.with_target_geometry(target_geometry.clone());

        match image.resize() {
            Ok(new_image) => {
                debug!(
                    "{}: Resized image to {}x{}",
                    input_path.display(),
                    new_image.width(),
                    new_image.height()
                );
            }
            Err(e) => {
                error!("Error resizing image {}: {:?}", input_path.display(), e);
                return 1;
            }
        }
    }

    let bytes_to_write = match cli.output_type {
        None => match image.auto_format() {
            Ok((format, data)) => {
                debug!(
                    "{}: Auto-optimized image to format {}",
                    input_path.display(),
                    format,
                );
                if data.len() > image.original_file_size as usize {
                    let original_size = image.original_file_size as usize;
                    let increase = data.len() - original_size;
                    let pct_change = (data.len() as f64 / max(original_size, 1) as f64) * 100.0;
                    error!(
                        "{}: Auto-mode output would be larger; skipping write (format {}, {} -> {} bytes, +{}, {:.1}%)",
                        input_path.display(),
                        format,
                        format_bytes(original_size as u64),
                        format_bytes(data.len() as u64),
                        format_bytes(increase as u64),
                        pct_change
                    );
                    return 2;
                }
                image.output_format = Some(format);
                data
            }
            Err(e) => {
                error!(
                    "Error auto-optimizing image {}: {:?}",
                    input_path.display(),
                    e
                );
                return 1;
            }
        },
        Some(format) => match image.output_as_format(format) {
            Ok(data) => {
                info!(
                    "{}: Encoded image to format {}, size {} bytes",
                    input_path.display(),
                    format,
                    data.len()
                );
                image.output_format = Some(format);
                data
            }
            Err(e) => {
                error!(
                    "Error encoding image {} as {:?}: {:?}",
                    input_path.display(),
                    format,
                    e
                );
                return 1;
            }
        },
    };

    if cli.compare || cli.min_ssim.is_some() || cli.min_psnr.is_some() {
        let compute_ssim = cli.compare || cli.min_ssim.is_some();
        let compute_psnr = cli.compare || cli.min_psnr.is_some();
        match image.compare_to_encoded(&bytes_to_write, compute_ssim, compute_psnr) {
            Ok(score) => {
                info!("{}: Perceptual comparison:", input_path.display());
                if let Some(ssim_score) = score.ssim {
                    let quality = SsimQuality::from_ssim(ssim_score)
                        .map(|q| q.meaning())
                        .unwrap_or("unmeasurable");
                    info!("  SSIM: {:.6} ({})", ssim_score, quality);
                }
                if let Some(psnr_score) = score.psnr {
                    if psnr_score.is_infinite() {
                        if let Some(quality) = PsnrQuality::from_psnr(psnr_score) {
                            info!("  PSNR: inf dB ({})", quality.meaning());
                        } else {
                            info!("  PSNR: inf dB");
                        }
                    } else {
                        let quality = PsnrQuality::from_psnr(psnr_score)
                            .map(|q| q.meaning())
                            .unwrap_or("pretty ugly");
                        info!("  PSNR: {:.2} dB ({})", psnr_score, quality);
                    }
                }

                if let Some(min_ssim) = cli.min_ssim {
                    if score.ssim.is_none() {
                        error!(
                            "{}: SSIM score was not computed, cannot enforce --min-ssim",
                            input_path.display()
                        );
                        return 3;
                    }

                    if let Some(actual_ssim) = score.ssim
                        && actual_ssim < min_ssim
                    {
                        error!(
                            "{}: Perceptual quality gate failed: SSIM {:.6} is below minimum {}",
                            input_path.display(),
                            actual_ssim,
                            min_ssim
                        );
                        return 3;
                    }
                }

                if let Some(min_psnr) = cli.min_psnr {
                    if score.psnr.is_none() {
                        error!(
                            "{}: PSNR score was not computed, cannot enforce --min-psnr",
                            input_path.display()
                        );
                        return 3;
                    }

                    if let Some(actual_psnr) = score.psnr
                        && actual_psnr < min_psnr
                    {
                        error!(
                            "{}: Perceptual quality gate failed: PSNR {:.2} dB is below minimum {}",
                            input_path.display(),
                            actual_psnr,
                            min_psnr
                        );
                        return 3;
                    }
                }
            }
            Err(e) => {
                if cli.min_ssim.is_some() || cli.min_psnr.is_some() {
                    error!(
                        "{}: Perceptual comparison failed: {:?}",
                        input_path.display(),
                        e
                    );
                    return 3;
                }
                warn!(
                    "{}: Perceptual comparison failed, continuing: {:?}",
                    input_path.display(),
                    e
                );
            }
        }
    }

    if bytes_to_write.is_empty() {
        error!(
            "{}: No image data to write. This is probably a bug!",
            input_path.display()
        );
        return 1;
    }

    if image.will_overwrite() && !cli.force {
        error!(
            "{}: Output file {} already exists. Use --force to overwrite.",
            input_path.display(),
            image.output_filename().display()
        );
        return 1;
    }

    match std::fs::write(image.output_filename(), &bytes_to_write) {
        Ok(_) => {
            let original_size = max(image.original_file_size, 1) as f64;
            let output_size = max(bytes_to_write.len(), 1) as f64;
            let pct_change = output_size / original_size * 100.0;
            info!(
                "{}: Wrote optimized image to {} ({} -> {} bytes, {:.1}% of original)",
                input_path.display(),
                image.output_filename().display(),
                format_bytes(original_size as u64),
                format_bytes(output_size as u64),
                pct_change
            );
        }
        Err(e) => {
            error!(
                "{}: Error writing optimized image to {}: {}",
                input_path.display(),
                image.output_filename().display(),
                e
            );
            return 1;
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
                            "{}: Delete check: format_changed={}, size_reduced={}",
                            input_path.display(),
                            format_changed,
                            size_reduced
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
                                    warn!(
                                        "{}: Error prompting for deletion: {}",
                                        input_path.display(),
                                        e
                                    );
                                }
                            }
                        } else {
                            debug!(
                                "{}: No benefit to deleting original file (same format and not smaller)",
                                input_path.display()
                            );
                        }
                    } else {
                        warn!(
                            "{}: Output format not set after conversion",
                            input_path.display()
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "{}: Could not determine original format for {}: {:?}",
                        input_path.display(),
                        image.input_filename.display(),
                        e
                    );
                }
            }
        } else {
            debug!(
                "{}: Skipping deletion: output overwrote input file",
                input_path.display()
            );
        }
    }

    0
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.debug);

    let target_geometry = match cli.geometry.as_deref() {
        Some(target_geometry) => match Geometry::from_str(target_geometry) {
            Ok(geometry) if geometry.is_empty() => None,
            Ok(geometry) => Some(geometry),
            Err(e) => {
                error!("Error parsing geometry: {:?}", e);
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    let mut exit_code = 0;
    for filename in &cli.filenames {
        let current_exit_code = process_image(&cli, target_geometry.as_ref(), filename.as_path());
        exit_code = aggregate_exit_code(exit_code, current_exit_code);
    }

    ExitCode::from(exit_code)
}

#[cfg(test)]
mod tests {
    use super::aggregate_exit_code;

    #[test]
    fn test_aggregate_exit_code_all_success() {
        let mut exit_code = 0;
        for current_code in [0, 0, 0] {
            exit_code = aggregate_exit_code(exit_code, current_code);
        }

        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_aggregate_exit_code_uses_highest_failure() {
        let mut exit_code = 0;
        for current_code in [1, 3, 2, 1] {
            exit_code = aggregate_exit_code(exit_code, current_code);
        }

        assert_eq!(exit_code, 3);
    }
}
