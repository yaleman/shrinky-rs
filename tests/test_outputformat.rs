#[test]
fn test_outputformat() {
    let expected = [
        ("jpg", Some(shrinky_rs::ImageFormat::Jpg)),
        ("jpeg", Some(shrinky_rs::ImageFormat::Jpg)),
        ("png", Some(shrinky_rs::ImageFormat::Png)),
        ("webp", Some(shrinky_rs::ImageFormat::Webp)),
        ("avif", Some(shrinky_rs::ImageFormat::Avif)),
        ("bmp", None),
        ("tiff", None),
    ];

    for (input, expected) in expected.iter() {
        let result = input.parse::<shrinky_rs::ImageFormat>();
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
}
