use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use shrinky_rs::{
    ImageFormat,
    cli::test_setup_logging,
    imagedata::{Geometry, Image},
};
use std::io::Cursor;
use std::path::PathBuf;

use clap::Parser;

const IMAGE_NAME: &str = "bruny-oysters";
const JPG_EXPECTED_WIDTH: u32 = 1330;
const JPG_EXPECTED_HEIGHT: u32 = 2364;
const PNG_EXPECTED_WIDTH: u32 = 450;
const PNG_EXPECTED_HEIGHT: u32 = 800;

#[test]
fn test_loading_multiple() {
    test_setup_logging();
    let test_fmts = vec![
        ImageFormat::Jpg,
        ImageFormat::Webp,
        ImageFormat::Avif,
        ImageFormat::Heif,
        ImageFormat::Heic,
    ];

    // use rayon because it's faster
    test_fmts.par_iter().for_each(|fmt| {
        let img_path = std::path::PathBuf::from(format!(
            "tests/test_images/{}.{}",
            IMAGE_NAME,
            fmt.extension()
        ));
        let img = Image::try_from(&img_path).unwrap_or_else(|_| {
            panic!(
                "failed to load Image from path for format {}",
                fmt.extension()
            )
        });
        let geometry = img.final_geometry();

        let Geometry { width, height } = geometry;
        assert_eq!(
            width,
            Some(JPG_EXPECTED_WIDTH),
            "{} Image width should be {JPG_EXPECTED_WIDTH}",
            fmt.extension()
        );
        assert_eq!(
            height,
            Some(JPG_EXPECTED_HEIGHT),
            "{} Image height should be {JPG_EXPECTED_HEIGHT}",
            fmt.extension()
        );
    })
}

#[test]
fn test_with_png() {
    test_setup_logging();
    let img_path = std::path::PathBuf::from(format!(
        "tests/test_images/{}.{}",
        IMAGE_NAME,
        ImageFormat::Png.extension()
    ));

    let mut img = Image::try_from(&img_path).expect("failed to load Image from path");

    let Geometry { width, height } = img.final_geometry();
    assert_eq!(
        width,
        Some(PNG_EXPECTED_WIDTH),
        "Image width should be {PNG_EXPECTED_WIDTH}"
    );
    assert_eq!(
        height,
        Some(PNG_EXPECTED_HEIGHT),
        "Image height should be {PNG_EXPECTED_HEIGHT}"
    );

    img = img.with_target_geometry(Geometry {
        width: Some(1234),
        height: None,
    });
    assert_eq!(
        img.final_geometry(),
        Geometry {
            width: Some(1234),
            height: Some(
                (PNG_EXPECTED_HEIGHT as f32 * (1234_f32 / PNG_EXPECTED_WIDTH as f32)) as u32
            ),
        },
        "Image should have target geometry set"
    );

    // test resising the image
    img = img.with_target_geometry(Geometry {
        width: Some(400),
        height: Some(400),
    });
    img.resize().expect("failed to resize image");

    assert!(
        img.final_geometry() != Geometry::new(PNG_EXPECTED_WIDTH, PNG_EXPECTED_HEIGHT),
        "Image should have updated geometry"
    );

    assert!(
        img.final_geometry() == Geometry::new(400, 400),
        "Image should be resized to 400x400"
    );

    // exercise the output as format functionality
    for fmt in [ImageFormat::Jpg, ImageFormat::Heic] {
        let my_img = img.clone().with_output_format(fmt);
        assert!(
            my_img.output_as_format(fmt).is_ok(),
            "Image should output as format {}",
            fmt.extension()
        );
    }
}

#[test]
fn test_output_format() {
    test_setup_logging();
    let mut image = Image::try_from(&std::path::PathBuf::from(format!(
        "tests/test_images/{}.{}",
        IMAGE_NAME,
        ImageFormat::Jpg.extension()
    )))
    .expect("failed to load test Image from path");

    image = image.with_output_format(ImageFormat::Jpg);

    assert!(
        image.will_overwrite(),
        "Image should report it will overwrite when output format matches input file extension"
    );

    // change the output format to PNG
    image = image.with_output_format(ImageFormat::Png);
    assert_eq!(
        image.output_filename(),
        std::path::PathBuf::from(format!(
            "tests/test_images/{}.{}",
            IMAGE_NAME,
            ImageFormat::Png.extension()
        )),
        "Output filename should have the correct extension when output format is set"
    );
    assert!(
        image.will_overwrite(),
        "Image should report it will overwrite because test file should exist: input={} output={}, format={:?}",
        image.input_filename.display(),
        image.output_filename().display(),
        image.output_format
    );

    assert_eq!(
        image.resize().expect("Failed to resize image"),
        image.image,
        "Resizing without changing geometry should be a no-op"
    );

    image = image.with_target_geometry(Geometry {
        width: Some(100),
        height: None,
    });

    let resized_image = image.resize().expect("Failed to resize image");
    assert!(
        resized_image != image.image,
        "Resizing with changed geometry should produce a different image"
    );
    assert!(
        resized_image.width() == 100,
        "Resized image should have width of 100"
    );

    let (format, _bytes) = image
        .auto_format()
        .expect("Failed to convert to auto format");
    assert!(
        format != ImageFormat::Png,
        "Image output format should be something other than PNG 'cause that's huge"
    );
}

#[test]
fn test_output_filename_never_jpeg() {
    test_setup_logging();
    let base_image = Image {
        original_file_size: 0,
        input_filename: std::path::PathBuf::from("tests/test_images/sample.jpeg"),
        original_geometry: Geometry::new(1, 1),
        target_geometry: None,
        output_format: None,
        output_suffix: None,
        image: image::DynamicImage::new_rgba8(1, 1),
    };

    assert_eq!(
        base_image.output_filename(),
        std::path::PathBuf::from("tests/test_images/sample.jpg"),
        "Output filename should normalize .jpeg to .jpg"
    );

    let image_with_output = Image {
        output_format: Some(ImageFormat::Jpg),
        ..base_image.clone()
    };

    assert_eq!(
        image_with_output.output_filename(),
        std::path::PathBuf::from("tests/test_images/sample.jpg"),
        "Output filename should use .jpg for JPG output"
    );
}

#[test]
fn test_compare_identical_images() {
    test_setup_logging();
    let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(16, 16, |x, y| {
        let base = x.saturating_add(y) as u8;
        image::Rgb([base, 255 - base, base / 2])
    }));
    let image = Image {
        original_file_size: 0,
        input_filename: PathBuf::from("tests/test_images/source-compare.jpg"),
        original_geometry: Geometry::new(16, 16),
        target_geometry: None,
        output_format: Some(ImageFormat::Jpg),
        output_suffix: None,
        image: source.clone(),
    };

    let mut encoded = Vec::new();
    source
        .write_to(&mut Cursor::new(&mut encoded), image::ImageFormat::Png)
        .expect("failed to encode source");

    let score = image
        .compare_to_encoded(&encoded, true, true)
        .expect("compare failed");

    assert!(
        (score.ssim.unwrap_or_default() - 1.0).abs() < 0.000_001,
        "expected identical SSIM score"
    );
    assert!(
        score.psnr.unwrap_or_default().is_infinite(),
        "expected infinite PSNR for identical images"
    );
}

#[test]
fn test_compare_degraded_image() {
    test_setup_logging();
    let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(16, 16, |x, y| {
        let base = x.saturating_add(y) as u8;
        image::Rgb([base, 255 - base, base / 2])
    }));
    let image = Image {
        original_file_size: 0,
        input_filename: PathBuf::from("tests/test_images/source-compare.jpg"),
        original_geometry: Geometry::new(16, 16),
        target_geometry: None,
        output_format: Some(ImageFormat::Jpg),
        output_suffix: None,
        image: source.clone(),
    };

    let mut degraded = source.clone().to_rgb8();
    let pixel = degraded.get_pixel_mut(1, 1);
    *pixel = image::Rgb([0, 255, 128]);

    let mut encoded = Vec::new();
    image::DynamicImage::ImageRgb8(degraded)
        .write_to(&mut Cursor::new(&mut encoded), image::ImageFormat::Png)
        .expect("failed to encode degraded image");

    let score = image
        .compare_to_encoded(&encoded, true, true)
        .expect("compare failed");

    assert!(score.ssim.unwrap_or(1.0) < 1.0);
    assert!(score.psnr.unwrap_or(f64::INFINITY).is_finite());
}

#[test]
fn test_compare_dimension_mismatch() {
    test_setup_logging();
    let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(16, 16, |x, y| {
        let base = x.saturating_add(y) as u8;
        image::Rgb([base, 255 - base, base / 2])
    }));
    let image = Image {
        original_file_size: 0,
        input_filename: PathBuf::from("tests/test_images/source-compare.jpg"),
        original_geometry: Geometry::new(16, 16),
        target_geometry: None,
        output_format: Some(ImageFormat::Png),
        output_suffix: None,
        image: source,
    };

    let mut encoded = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::new(8, 8))
        .write_to(&mut Cursor::new(&mut encoded), image::ImageFormat::Png)
        .expect("failed to encode resized image");

    let score = image.compare_to_encoded(&encoded, true, true);
    assert!(score.is_err());
}

#[test]
fn test_cli_compare_flags() {
    use shrinky_rs::cli::Cli;

    let cli = Cli::parse_from([
        "shrinky-rs",
        "--compare",
        "--min-ssim",
        "0.9",
        "--min-psnr",
        "30",
        "--output-suffix",
        "-foo",
        "tests/test_images/bruny-oysters.jpg",
    ]);

    assert!(cli.compare);
    assert_eq!(cli.min_ssim, Some(0.9));
    assert_eq!(cli.min_psnr, Some(30.0));
    assert_eq!(cli.output_suffix, Some("-foo".to_string()));
    assert_eq!(
        cli.filename,
        PathBuf::from("tests/test_images/bruny-oysters.jpg")
    );
}

#[test]
fn test_output_filename_with_suffix() {
    test_setup_logging();
    let image = Image {
        original_file_size: 0,
        input_filename: std::path::PathBuf::from("tests/test_images/example.gif"),
        original_geometry: Geometry::new(1, 1),
        target_geometry: None,
        output_format: Some(ImageFormat::Jpg),
        output_suffix: Some("-foo".to_string()),
        image: image::DynamicImage::new_rgba8(1, 1),
    };

    assert_eq!(
        image.output_filename(),
        std::path::PathBuf::from("tests/test_images/example-foo.jpg"),
        "Output filename should include suffix before extension"
    );
}
