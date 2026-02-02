use libheif_rs::HeifError;
use shrinky_rs::{ImageFormat, cli::test_setup_logging};
use std::str::FromStr;

#[test]
fn test_imageformat() {
    test_setup_logging();
    let expected = [
        ("jpg", Some(ImageFormat::Jpg)),
        ("jpeg", Some(ImageFormat::Jpg)),
        ("png", Some(ImageFormat::Png)),
        ("webp", Some(ImageFormat::Webp)),
        ("avif", Some(ImageFormat::Avif)),
        ("bmp", None),
        ("tiff", None),
    ];

    for (input, expected) in expected.iter() {
        let result = input.parse::<ImageFormat>();
        match expected {
            Some(fmt) => {
                assert!(result.is_ok(), "Expected Ok for input '{}'", input);
                let ofmt = result.expect("Failed to unwrap OutputFormat");
                assert_eq!(ofmt, *fmt, "OutputFormat mismatch for input '{}'", input);
            }
            None => {
                assert!(result.is_err(), "Expected Err for input '{}'", input);
            }
        }
    }

    assert_eq!(format!("{}", ImageFormat::Jpg), "JPG");

    assert_eq!(
        <ImageFormat as FromStr>::from_str("testfile.jpg").expect("Failed to parse from filename"),
        ImageFormat::Jpg
    );

    assert_eq!(
        <ImageFormat as FromStr>::from_str("jpeg").expect("Failed to parse from filename"),
        ImageFormat::Jpg
    );
    assert_eq!(
        <ImageFormat as FromStr>::from_str("jpg").expect("Failed to parse from filename"),
        ImageFormat::Jpg
    );

    assert!(<ImageFormat as FromStr>::from_str("cheese").is_err());

    assert!(ImageFormat::all().len() == 6);

    assert!(ImageFormat::Jpg.is_native_image_format());
    assert!(!ImageFormat::Avif.is_native_image_format());

    // test that we can convert to image::ImageFormat
    for (fmt, expected_result) in [
        (ImageFormat::Jpg, true),
        (ImageFormat::Png, true),
        (ImageFormat::Webp, true),
        (ImageFormat::Avif, false),
        (ImageFormat::Heic, false),
        (ImageFormat::Heif, false),
    ] {
        let test_format: Result<image::ImageFormat, shrinky_rs::Error> = fmt.try_into();
        if expected_result {
            assert!(
                test_format.is_ok(),
                "Expected Ok converting supported format {:?}",
                fmt
            );
        } else {
            assert!(
                test_format.is_err(),
                "Expected Err converting unsupported format {:?}",
                fmt
            );
        }
    }

    let test_format: Result<image::ImageFormat, shrinky_rs::Error> = ImageFormat::Heic.try_into();
    test_format.expect_err("Expected error converting unsupported format");
}

#[test]
fn test_error() {
    test_setup_logging();

    let error = HeifError::from_heif_error(libheif_sys::heif_error {
        code: 5u32,
        subcode: 42,
        message: std::ffi::CString::new("Test error message")
            .unwrap()
            .into_raw(),
    })
    .expect_err("Failed to generate error");
    let shrinky_error: shrinky_rs::Error = error.into();
    assert!(format!("{:?}", shrinky_error).contains("Test error message"));
}
