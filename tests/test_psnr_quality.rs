use shrinky_rs::PsnrQuality;

#[test]
fn test_psnr_quality_boundaries_and_meaning() {
    let cases = [
        (f64::INFINITY, PsnrQuality::AlmostIdentical, "almost identical"),
        (50.0, PsnrQuality::AlmostIdentical, "almost identical"),
        (49.9, PsnrQuality::ExtremelyHighQuality, "extremely high quality"),
        (40.0, PsnrQuality::ExtremelyHighQuality, "extremely high quality"),
        (39.9, PsnrQuality::GoodCompression, "good compression"),
        (30.0, PsnrQuality::GoodCompression, "good compression"),
        (29.9, PsnrQuality::VisibleDegradation, "visible degradation"),
        (20.0, PsnrQuality::VisibleDegradation, "visible degradation"),
        (19.9, PsnrQuality::PrettyUgly, "pretty ugly"),
        (-5.0, PsnrQuality::PrettyUgly, "pretty ugly"),
    ];

    for (psnr, expected_quality, expected_meaning) in cases {
        let mapped = PsnrQuality::from_psnr(psnr).expect("Expected quality bucket for valid PSNR");
        assert_eq!(mapped, expected_quality, "Unexpected bucket for PSNR {psnr}");
        assert_eq!(mapped.meaning(), expected_meaning, "Unexpected meaning for {psnr}");
    }
}

#[test]
fn test_psnr_quality_rejects_nan() {
    assert!(PsnrQuality::from_psnr(f64::NAN).is_none());
}
