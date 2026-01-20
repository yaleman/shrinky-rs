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

use libheif_rs::HeifError;
use std::{ffi::OsString, fmt::Display, path::PathBuf, str::FromStr};
use strum::EnumIter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, EnumIter)]
pub enum ImageFormat {
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
        ImageFormat::from_str(ext)
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

impl From<OsString> for ImageFormat {
    fn from(os_str: OsString) -> Self {
        let s = os_str.to_string_lossy();
        ImageFormat::from_str(&s).unwrap_or(ImageFormat::Jpg)
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
    FileSystem(String),
    ImageEncodingError(String),
}

impl From<HeifError> for Error {
    fn from(err: HeifError) -> Self {
        Error::ImageEncodingError(err.to_string())
    }
}
