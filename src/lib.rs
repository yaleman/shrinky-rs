#![deny(warnings)]
#![deny(deprecated)]
#![recursion_limit = "512"]
#![warn(unused_extern_crates)]
// Enable some groups of clippy lints.
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
// Specific lints to enforce.
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::disallowed_types)]
#![deny(clippy::manual_let_else)]
#![allow(clippy::unreachable)]

pub mod cli;
pub mod imagedata;

use clap::ValueEnum;
use libheif_rs::HeifError;
use log::{debug, error, info, warn};
use std::{
    cmp::max,
    fmt::Display,
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use strum::EnumIter;

use crate::{
    cli::Cli,
    imagedata::{Geometry, Image},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, EnumIter, ValueEnum)]
pub enum ImageFormat {
    #[value(alias = "jpeg")]
    Jpg,
    Png,
    Webp,
    Avif,
    Heic,
    Heif,
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_uppercase())
    }
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Heic => "heic",
            ImageFormat::Heif => "heif",
        }
    }

    pub fn try_from_filename(filename: &str) -> Result<Self, Error> {
        let ext = filename.to_ascii_lowercase();
        let ext = ext
            .rsplit('.')
            .next()
            .ok_or_else(|| Error::UnsupportedFormat(filename.to_string()))?;
        <ImageFormat as std::str::FromStr>::from_str(ext)
    }

    pub fn is_native_image_format(&self) -> bool {
        !matches!(
            self,
            ImageFormat::Avif | ImageFormat::Heic | ImageFormat::Heif
        )
    }

    pub fn all() -> Vec<ImageFormat> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PsnrQuality {
    PrettyUgly,
    VisibleDegradation,
    GoodCompression,
    ExtremelyHighQuality,
    AlmostIdentical,
}

impl PsnrQuality {
    pub fn from_psnr(psnr: f64) -> Option<Self> {
        if psnr.is_nan() {
            return None;
        }

        if psnr >= 50.0 {
            Some(Self::AlmostIdentical)
        } else if psnr >= 40.0 {
            Some(Self::ExtremelyHighQuality)
        } else if psnr >= 30.0 {
            Some(Self::GoodCompression)
        } else if psnr >= 20.0 {
            Some(Self::VisibleDegradation)
        } else {
            Some(Self::PrettyUgly)
        }
    }

    pub const fn meaning(self) -> &'static str {
        match self {
            Self::AlmostIdentical => "almost identical",
            Self::ExtremelyHighQuality => "extremely high quality",
            Self::GoodCompression => "good compression",
            Self::VisibleDegradation => "visible degradation",
            Self::PrettyUgly => "pretty ugly",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SsimQuality {
    NoticeableDegradation,
    SmallVisibleDifferences,
    ExtremelySimilar,
    Identical,
}

impl SsimQuality {
    pub fn from_ssim(ssim: f64) -> Option<Self> {
        if ssim.is_nan() {
            return None;
        }

        if ssim >= 1.0 {
            Some(Self::Identical)
        } else if ssim >= 0.9 {
            Some(Self::ExtremelySimilar)
        // } else if ssim >= 0.8 {
        //     Some(Self::SmallVisibleDifferences)
        } else if ssim >= 0.7 {
            Some(Self::SmallVisibleDifferences)
        } else {
            Some(Self::NoticeableDegradation)
        }
    }

    pub const fn meaning(self) -> &'static str {
        match self {
            Self::Identical => "identical images",
            Self::ExtremelySimilar => "extremely similar",
            Self::SmallVisibleDifferences => "small visible differences",
            Self::NoticeableDegradation => "noticeable degradation",
        }
    }
}

impl FromStr for ImageFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('.') {
            return ImageFormat::try_from_filename(s);
        }
        match s.to_lowercase().as_str() {
            "jpg" | "jpeg" => Ok(ImageFormat::Jpg),
            "png" => Ok(ImageFormat::Png),
            "webp" => Ok(ImageFormat::Webp),
            "avif" => Ok(ImageFormat::Avif),
            "heic" => Ok(ImageFormat::Heic),
            "heif" => Ok(ImageFormat::Heif),
            _ => Err(Error::UnsupportedFormat(s.to_string())),
        }
    }
}

impl TryFrom<&PathBuf> for ImageFormat {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let filename = path
            .to_str()
            .ok_or_else(|| Error::UnsupportedFormat("Invalid path".to_string()))?;
        ImageFormat::try_from_filename(filename)
    }
}

impl TryInto<image::ImageFormat> for ImageFormat {
    type Error = Error;
    fn try_into(self) -> Result<image::ImageFormat, Self::Error> {
        match self {
            ImageFormat::Jpg => Ok(image::ImageFormat::Jpeg),
            ImageFormat::Png => Ok(image::ImageFormat::Png),
            ImageFormat::Webp => Ok(image::ImageFormat::WebP),
            ImageFormat::Avif | ImageFormat::Heic | ImageFormat::Heif => {
                Err(Error::UnsupportedFormat(
                    "AVIF/HEIC/HEIF format not supported by image crate".to_string(),
                ))
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidOptions(String),
    UnsupportedFormat(String),
    InvalidGeometry(String),
    ImageLoadingError(String, image::ImageError),
    ImageComparisonError(String),
    FileSystem(String),
    ImageEncodingError(String),
}

impl From<HeifError> for Error {
    fn from(err: HeifError) -> Self {
        Error::ImageEncodingError(err.to_string())
    }
}

/// Format a byte count as a string with comma separators
pub fn format_bytes(bytes: u64) -> String {
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

pub fn should_prompt_delete_source(
    output_existed_before_write: bool,
    format_changed: bool,
    size_reduced: bool,
) -> bool {
    !output_existed_before_write && (format_changed || size_reduced)
}

/// Prompt user to delete source file, showing comparison information
pub fn prompt_delete_source(
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

pub fn process_image(cli: &Cli, target_geometry: Option<&Geometry>, input_path: &Path) -> u8 {
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

    let output_existed_before_write = image.will_overwrite();

    if output_existed_before_write && !cli.force {
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
        if !output_existed_before_write {
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
                        if should_prompt_delete_source(
                            output_existed_before_write,
                            format_changed,
                            size_reduced,
                        ) {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_should_prompt_delete_source_for_new_output_with_benefit() {
        assert!(should_prompt_delete_source(false, true, false));
        assert!(should_prompt_delete_source(false, false, true));
    }

    #[test]
    fn test_should_not_prompt_delete_source_when_output_already_existed() {
        assert!(!should_prompt_delete_source(true, true, true));
    }

    #[test]
    fn test_should_not_prompt_delete_source_without_benefit() {
        assert!(!should_prompt_delete_source(false, false, false));
    }
}
