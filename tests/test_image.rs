use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use shrinky_rs::{
    ImageFormat,
    imagedata::{Geometry, Image},
};

const IMAGE_NAME: &str = "bruny-oysters";
const JPG_EXPECTED_WIDTH: u32 = 1330;
const JPG_EXPECTED_HEIGHT: u32 = 2364;
const PNG_EXPECTED_WIDTH: u32 = 450;
const PNG_EXPECTED_HEIGHT: u32 = 800;

#[test]
fn test_loading_multiple() {
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
fn test_loading_png() {
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
}

#[test]
fn test_output_format() {
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
        !image.will_overwrite(),
        "Image should not report it will overwrite when output format does not match input file extension"
    );
}
