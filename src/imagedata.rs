//! Image handling magic

use std::{fmt::Display, io::Cursor, path::PathBuf, str::FromStr};

use image::DynamicImage;
use libheif_rs::{Channel, CompressionFormat, EncoderQuality, HeifContext, LibHeif};
use log::{debug, error};
use rayon::iter::IntoParallelIterator;

use crate::{Error, ImageFormat};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Geometry {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Geometry {
    pub fn empty() -> Self {
        Geometry {
            width: None,
            height: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width.is_none() && self.height.is_none()
    }

    pub fn new(width: u32, height: u32) -> Self {
        Geometry {
            width: Some(width),
            height: Some(height),
        }
    }
}

impl Display for Geometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.width, self.height) {
            (Some(w), Some(h)) => write!(f, "{}x{}", w, h),
            (Some(w), None) => write!(f, "{}x", w),
            (None, Some(h)) => write!(f, "x{}", h),
            (None, None) => write!(f, "empty"),
        }
    }
}

impl FromStr for Geometry {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        debug!("Parsing geometry from string: {}", s);

        let (width, height) = if let Some(height_string) = s.strip_prefix('x') {
            (
                None,
                Some(
                    height_string
                        .parse::<u32>()
                        .map_err(|_| Error::InvalidGeometry(s.to_string()))?,
                ),
            )
        } else if let Some(width_string) = s.strip_suffix('x') {
            (
                Some(
                    width_string
                        .parse::<u32>()
                        .map_err(|_| Error::InvalidGeometry(s.to_string()))?,
                ),
                None,
            )
        } else if s.contains('x') {
            let parts = s.split('x').collect::<Vec<&str>>();
            if parts.len() != 2 {
                return Err(Error::InvalidGeometry("Too many x characters".to_string()));
            }
            let width = parts[0]
                .parse::<u32>()
                .map_err(|_| Error::InvalidGeometry(format!("Invalid width from {s}")))?;
            let height = parts[1]
                .parse::<u32>()
                .map_err(|_| Error::InvalidGeometry(format!("Invalid height from {s}")))?;
            (Some(width), Some(height))
        } else {
            return Err(Error::InvalidGeometry(s.to_string()));
        };

        Ok(Geometry { width, height })
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub original_file_size: u64,
    pub input_filename: PathBuf,
    pub original_geometry: Geometry,
    pub target_geometry: Option<Geometry>,
    pub output_format: Option<crate::ImageFormat>,
    pub image: image::DynamicImage,
}

impl TryFrom<&PathBuf> for Image {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let original_size = std::fs::metadata(path)
            .map_err(|e| Error::FileSystem(e.to_string()))?
            .len();

        let (image, original_geometry) = Image::load_image(path)?;

        Ok(Self {
            input_filename: path.clone(),
            target_geometry: None,
            output_format: None,
            image,
            original_file_size: original_size,
            original_geometry,
        })
    }
}

impl Image {
    pub fn with_target_geometry(mut self, target_geometry: Geometry) -> Self {
        self.target_geometry = Some(target_geometry);
        self
    }

    pub fn with_output_format(mut self, output_format: crate::ImageFormat) -> Self {
        self.output_format = Some(output_format);
        self
    }

    pub fn will_overwrite(&self) -> bool {
        if let Some(ref format) = self.output_format {
            match format {
                ImageFormat::Png => self
                    .input_filename
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png")),
                ImageFormat::Jpg => self
                    .input_filename
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg")),
                ImageFormat::Webp => self
                    .input_filename
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("webp")),
                ImageFormat::Heif | ImageFormat::Heic => {
                    self.input_filename.extension().is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("heif") || ext.eq_ignore_ascii_case("heic")
                    })
                }
                ImageFormat::Avif => self
                    .input_filename
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("avif")),
            }
        } else {
            true
        }
    }

    pub fn load_image(input_filename: &PathBuf) -> Result<(DynamicImage, Geometry), Error> {
        let image_format = ImageFormat::try_from(input_filename)?;

        match image_format {
            ImageFormat::Heif | ImageFormat::Heic => {
                // Ensure libheif is initialized
                libheif_rs::integration::image::register_all_decoding_hooks();
            }
            _ => {}
        }

        let img = image::open(input_filename)
            .map_err(|e| Error::ImageLoadingError(input_filename.display().to_string(), e))?;

        let geometry = Geometry::new(img.width(), img.height());

        Ok((img, geometry))
    }

    /// Get the final target geometry of the image after resizing (if any)
    pub fn final_geometry(&self) -> Geometry {
        match self.target_geometry {
            Some(ref geom) => match geom {
                Geometry {
                    width: Some(_w),
                    height: Some(_h),
                } => geom.clone(),
                Geometry {
                    width: Some(w),
                    height: None,
                } => {
                    let ratio = *w as f32 / self.image.width() as f32;
                    Geometry::new(*w, (self.image.height() as f32 * ratio) as u32)
                }
                Geometry {
                    width: None,
                    height: Some(h),
                } => {
                    let ratio = *h as f32 / self.image.height() as f32;
                    Geometry::new((self.image.width() as f32 * ratio) as u32, *h)
                }
                Geometry {
                    width: None,
                    height: None,
                } => Geometry::new(self.image.width(), self.image.height()),
            },
            None => Geometry::new(self.image.width(), self.image.height()),
        }
    }

    pub fn resize(&mut self) -> Result<DynamicImage, Error> {
        let final_geometry = self.final_geometry();
        if final_geometry != Geometry::new(self.image.width(), self.image.height()) {
            debug!(
                "Resizing image from {}x{} to {}",
                self.image.width(),
                self.image.height(),
                final_geometry,
            );
            let resized_img = self.image.resize_exact(
                final_geometry.width.unwrap_or(0), // safe unwraps, as final_geometry is derived from existing dimensions
                final_geometry.height.unwrap_or(0), // safe unwraps, as final_geometry is derived from existing dimensions
                image::imageops::FilterType::Lanczos3,
            );
            Ok(resized_img)
        } else {
            Ok(self.image.clone())
        }
    }

    /// build and return HEIF/HEIC image data
    fn output_heif(&self) -> Result<Vec<u8>, Error> {
        let lib_heif = LibHeif::new();
        let mut context = HeifContext::new()?;
        let mut encoder = lib_heif.encoder_for_format(CompressionFormat::Av1)?;
        let Geometry { width, height } = self.final_geometry();

        let width = width.ok_or_else(|| {
            Error::ImageEncodingError("Width must be specified for HEIF encoding".to_string())
        })?;
        let height = height.ok_or_else(|| {
            Error::ImageEncodingError("Height must be specified for HEIF encoding".to_string())
        })?;

        let mut image = libheif_rs::Image::new(
            width,
            height,
            libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::C444),
        )?;
        image.create_plane(Channel::R, width, height, 8)?;
        image.create_plane(Channel::G, width, height, 8)?;
        image.create_plane(Channel::B, width, height, 8)?;

        let planes = image.planes_mut();

        let (Some(plane_r), Some(plane_g), Some(plane_b)) = (planes.r, planes.g, planes.b) else {
            return Err(Error::ImageEncodingError(
                "Failed to get one of the planes for HEIF image, this is definitely a bug in the code!".to_string(),
            ));
        };

        if let Some(pixels) = self.image.as_rgba8() {
            debug!("handling rgb8 image with alpha channel for heif encoding");
            pixels
                .pixels()
                .enumerate()
                .for_each(|(pixel_index, pixel)| {
                    pixel
                        .0
                        .iter()
                        .enumerate()
                        .for_each(|(i, &channel)| match i {
                            0 => {
                                plane_r.data[pixel_index] = channel;
                            }
                            1 => {
                                plane_g.data[pixel_index] = channel;
                            }
                            2 => {
                                plane_b.data[pixel_index] = channel;
                            }
                            _ => {}
                        });
                });
        } else if let Some(pixels) = self.image.as_rgb8() {
            debug!("handling rgb8 image without alpha channel for heif encoding");
            pixels
                .pixels()
                .enumerate()
                .for_each(|(pixel_index, pixel)| {
                    pixel
                        .0
                        .iter()
                        .enumerate()
                        .for_each(|(i, &channel)| match i {
                            0 => {
                                plane_r.data[pixel_index] = channel;
                            }
                            1 => {
                                plane_g.data[pixel_index] = channel;
                            }
                            2 => {
                                plane_b.data[pixel_index] = channel;
                            }
                            _ => {}
                        });
                });
        } else {
            return Err(Error::ImageEncodingError(
                "Failed to get RGB8-ish data from image for HEIF encoding".to_string(),
            ));
        }

        encoder.set_quality(EncoderQuality::Lossy(85))?;
        context.encode_image(&image, &mut encoder, None)?;
        context.write_to_bytes().map_err(Error::from)
    }

    pub fn output_as_format(&self, format: ImageFormat) -> Result<Vec<u8>, Error> {
        let write_format: Result<image::ImageFormat, Error> = format.try_into();
        if let Ok(write_format) = write_format {
            let mut buffer: Vec<u8> = Vec::new();
            self.image
                .write_to(&mut Cursor::new(&mut buffer), write_format)
                .map_err(|e| Error::ImageEncodingError(e.to_string()))?;
            Ok(buffer)
        } else {
            if format.is_native_image_format() {
                return Err(Error::ImageEncodingError(
                    "Failed to convert to native image format".to_string(),
                ));
            }
            self.output_heif()
        }
    }

    pub fn output_filename(&self) -> PathBuf {
        if let Some(ref format) = self.output_format {
            let mut output_path = self.input_filename.clone();
            output_path.set_extension(format.extension());
            output_path
        } else {
            self.input_filename.clone()
        }
    }

    pub fn auto_format(&self) -> Result<(ImageFormat, Vec<u8>), Error> {
        debug!("Auto-optimizing image format");
        use rayon::iter::ParallelIterator;
        let results: Vec<(ImageFormat, Result<Vec<u8>, Error>)> = ImageFormat::all()
            .into_par_iter()
            .map(|fmt| {
                debug!("Trying format {:?}", fmt);
                (fmt, self.output_as_format(fmt))
            })
            .collect();

        let results = results.into_iter().filter_map(|(format, data)| match data {
            Ok(encoded_data) => {
                debug!("Format {} produced {} bytes", format, encoded_data.len());
                Some((format, encoded_data))
            }
            Err(err) => {
                error!("Failed to encode image as {}: {:?}", format, err);
                None
            }
        });

        if let Some((format, data)) = results.into_iter().min_by_key(|r| r.1.iter().len()) {
            debug!("Woo, the smallest is {}", format);
            return Ok((format, data));
        }
        Err(Error::ImageEncodingError(
            "Failed to determine optimal image format".to_string(),
        ))
    }
}
