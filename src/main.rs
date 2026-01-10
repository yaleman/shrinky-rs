use std::str::FromStr;

use clap::Parser;

use log::{debug, error, info};
use shrinky_rs::{
    cli::Cli,
    imagedata::{Geometry, Image},
};

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
}
