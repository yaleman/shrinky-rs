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
use std::{fmt::Display, path::PathBuf, str::FromStr};
use strum::EnumIter;

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
