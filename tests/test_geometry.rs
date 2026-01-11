use std::str::FromStr;

use shrinky_rs::imagedata::Geometry;

#[test]
fn test_geometry() {
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
                assert_eq!(
                    format!("{}", geometry),
                    input.to_string(),
                    "String representation mismatch for input '{}'",
                    input
                );
            }
            None => {
                assert!(result.is_err(), "Expected Err for input '{}'", input);
            }
        }
    }

    let empty_geometry = Geometry::empty();
    assert!(empty_geometry.is_empty(), "Expected geometry to be empty");

    assert!(
        Geometry::from_str("800x1234x12345").is_err(),
        "Expected Err for invalid input with multiple 'x' characters"
    );

    assert!(
        Geometry::from_str("abcx600").is_err(),
        "Expected Err for non-numeric width"
    );

    assert!(
        Geometry::from_str("800xdef").is_err(),
        "Expected Err for non-numeric height"
    );

    for should_int_error in [
        Geometry::from_str(
            "80000000000000000000000000000000000010987230984710984709182374092187409821740981234x",
        )
        .expect_err("should return an integer error"),
        Geometry::from_str(
            "x80000000000000000000000000000000000010987230984710984709182374092187409821740981234",
        )
        .expect_err("should return an integer error"),
        Geometry::from_str(
            "800000000000000000000000000000000000x800000000000000000000000000000000000",
        )
        .expect_err("should return an integer error"),
    ] {
        dbg!(&should_int_error);
        match should_int_error {
            shrinky_rs::Error::InvalidGeometry(_) => {}
            _ => panic!("Expected InvalidGeometry error"),
        }
    }
}
