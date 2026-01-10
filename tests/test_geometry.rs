use shrinky_rs::imagedata::Geometry;

#[test]
fn test_geometry_from_string() {
    let expected = [
        ("800x600", Some((Some(800), Some(600)))),
        ("800x", Some((Some(800), None))),
        ("x600", Some((None, Some(600)))),
        ("1024x768", Some((Some(1024), Some(768)))),
        ("500x", Some((Some(500), None))),
        ("x400", Some((None, Some(400)))),
        ("invalid", None),
        ("800by600", None),
        ("800", None),
    ];
    for (input, expected) in expected.iter() {
        let result = input.parse::<Geometry>();
        match expected {
            Some((w, h)) => {
                assert!(result.is_ok(), "Expected Ok for input '{}'", input);
                let geometry = result.expect("Failed to unwrap geometry");
                assert_eq!(geometry.width, *w, "Width mismatch for input '{}'", input);
                assert_eq!(geometry.height, *h, "Height mismatch for input '{}'", input);
            }
            None => {
                assert!(result.is_err(), "Expected Err for input '{}'", input);
            }
        }
    }
}
